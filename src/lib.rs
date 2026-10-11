// SPDX-License-Identifier: MPL-2.0
//! GStreamer policy used by the desktop shell and its standalone diagnostics.
//! Registry presence is not proof that a device/driver can run a codec.
use gst::prelude::*;
use gstreamer as gst;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Auto,
    Cpu,
}
impl Mode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "cpu" => Ok(Self::Cpu),
            _ => Err(format!("Unknown media mode {value:?}; use auto or cpu")),
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
        }
    }
}

fn hardware(name: &str, klass: &str) -> bool {
    klass.split('/').any(|part| part == "Hardware")
        || [
            "vaapi", "vah", "vavp", "vaav", "nvh", "nvvp", "nvav", "d3d11", "d3d12", "qsv", "v4l2",
            "amf", "mfh",
        ]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

fn factories() -> Vec<gst::ElementFactory> {
    let mut all: Vec<_> =
        gst::ElementFactory::factories_with_type(gst::ElementFactoryType::DECODER, gst::Rank::NONE)
            .into_iter()
            .collect();
    all.extend(gst::ElementFactory::factories_with_type(
        gst::ElementFactoryType::ENCODER,
        gst::Rank::NONE,
    ));
    all.sort_by_key(|f| f.name().to_string());
    all.dedup_by(|a, b| a.name() == b.name());
    all
}

// Preserve original plugin ranks so a mode toggle can restore them exactly.
static ORIGINAL_RANKS: OnceLock<Mutex<Vec<(String, gst::Rank)>>> = OnceLock::new();
pub fn initialize(mode: Mode) -> Result<(), String> {
    gst::init().map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let directory = exe.parent().ok_or("Missing executable directory")?;
        // The Windows portable archive carries plugins beside the executable.
        gst::Registry::get().scan_path(directory);
    }
    apply_mode(mode)
}

pub fn apply_mode(mode: Mode) -> Result<(), String> {
    let all = factories();
    let mut ranks = ORIGINAL_RANKS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .map_err(|_| "Media rank state is poisoned")?;
    for factory in all {
        let name = factory.name().to_string();
        let original = match ranks.iter().find(|(n, _)| n == &name) {
            Some((_, rank)) => *rank,
            None => {
                let rank = factory.rank();
                ranks.push((name.clone(), rank));
                rank
            }
        };
        if hardware(&name, factory.klass()) && factory.klass().contains("Decoder") {
            factory.set_rank(match mode {
                // Prefer hardware but leave all software decoders available.
                Mode::Auto => original.max(gst::Rank::PRIMARY + 100),
                Mode::Cpu => gst::Rank::NONE,
            });
        } else {
            factory.set_rank(original);
        }
    }
    Ok(())
}

pub fn report() -> String {
    let mut result = format!(
        "GStreamer {}\n\nRegistered codecs (availability is not a hardware playback test):\n",
        gst::version_string()
    );
    for f in factories() {
        let kind = if hardware(f.name().as_str(), f.klass()) {
            "hardware"
        } else {
            "software"
        };
        result.push_str(&format!(
            "{} | {} | {} | rank {:?}\n",
            f.name(),
            kind,
            f.klass(),
            f.rank()
        ));
    }
    result.push_str("\nAuto mode retains software decoders. Device initialization failures can still require a restart in CPU mode.\nAudio decoding normally uses CPU codecs. Rendering acceleration is independent of decoding.\nNo MSE or DRM implementation is supplied by these plugins.\n");
    result
}

/// Encode a short synthetic H.264/MP4 clip. This validates an encoder without
/// capturing, downloading or transcoding YouTube content. Never overwrites files.
pub fn encode_test(output: &Path, encoder: &str, mode: Mode) -> Result<String, String> {
    use std::fs::OpenOptions;
    if output.extension().and_then(|x| x.to_str()) != Some("mp4") {
        return Err("Encoding test output must end in .mp4".into());
    }
    let mut destination = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| format!("Cannot create {}: {e}", output.display()))?;
    let candidates: Vec<&str> = match encoder {
        "auto" if mode == Mode::Auto => vec![
            "d3d12h264enc",
            "qsvh264enc",
            "nvh264enc",
            "vah264enc",
            "vaapih264enc",
            "mfh264enc",
            "x264enc",
            "openh264enc",
        ],
        "auto" | "cpu" => vec!["x264enc", "openh264enc"],
        // Explicit choices use a fixed allowlist: no pipeline-string injection.
        "d3d12h264enc" | "qsvh264enc" | "nvh264enc" | "vah264enc" | "vaapih264enc"
        | "mfh264enc" | "x264enc" | "openh264enc" => vec![encoder],
        _ => {
            drop(destination);
            let _ = std::fs::remove_file(output);
            return Err("Unsupported H.264 encoder".into());
        }
    };
    let temporary = output.with_extension(format!("{}.encoding", std::process::id()));
    // Refuse a stale or concurrent temporary path as well.
    let guard = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary);
    if let Err(e) = guard {
        drop(destination);
        let _ = std::fs::remove_file(output);
        return Err(format!("Cannot reserve encoding temporary file: {e}"));
    }
    drop(guard);
    let mut failures = Vec::new();
    let mut success = None;
    for candidate in candidates {
        if mode == Mode::Cpu && hardware(candidate, "") {
            continue;
        }
        if gst::ElementFactory::find(candidate).is_none() {
            continue;
        }
        match run_encoder(&temporary, candidate) {
            Ok(()) => {
                success = Some(candidate.to_string());
                break;
            }
            Err(e) => failures.push(format!("{candidate}: {e}")),
        }
    }
    let result = if let Some(name) = success {
        (|| {
            let mut source = std::fs::File::open(&temporary).map_err(|e| e.to_string())?;
            if source.metadata().map_err(|e| e.to_string())?.len() == 0 {
                return Err("Encoder produced an empty file".into());
            }
            std::io::copy(&mut source, &mut destination).map_err(|e| e.to_string())?;
            destination.sync_all().map_err(|e| e.to_string())?;
            Ok(name)
        })()
    } else {
        Err(format!("No usable H.264 encoder. {}", failures.join("; ")))
    };
    drop(destination);
    let _ = std::fs::remove_file(&temporary);
    if result.is_err() {
        let _ = std::fs::remove_file(output);
    }
    result
}

fn run_encoder(path: &Path, encoder: &str) -> Result<(), String> {
    // All pipeline tokens are static or selected from the allowlist above.
    let text = format!(
        "videotestsrc num-buffers=90 ! video/x-raw,width=640,height=360,framerate=30/1 ! videoconvert ! {encoder} ! h264parse ! mp4mux ! filesink name=output"
    );
    let pipeline = gst::parse::launch(&text)
        .map_err(|e| e.to_string())?
        .downcast::<gst::Pipeline>()
        .map_err(|_| "Not a pipeline")?;
    let output = pipeline.by_name("output").ok_or("Missing filesink")?;
    let path = path.to_str().ok_or("Encoding output path must be UTF-8")?;
    output.set_property("location", path);
    let result = (|| {
        pipeline
            .set_state(gst::State::Playing)
            .map_err(|e| e.to_string())?;
        let bus = pipeline.bus().ok_or("Missing pipeline bus")?;
        match bus.timed_pop_filtered(
            gst::ClockTime::from_seconds(30),
            &[gst::MessageType::Eos, gst::MessageType::Error],
        ) {
            Some(msg) => match msg.view() {
                gst::MessageView::Eos(..) => Ok(()),
                gst::MessageView::Error(e) => Err(format!("{} ({:?})", e.error(), e.debug())),
                _ => Err("Unexpected bus message".into()),
            },
            None => Err("Encoder timed out after 30 seconds".into()),
        }
    })();
    let teardown = pipeline
        .set_state(gst::State::Null)
        .map_err(|e| e.to_string());
    result.and(teardown.map(|_| ()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unknown_mode() {
        assert!(Mode::parse("gpu-only").is_err());
    }
    #[test]
    fn identifies_accelerators_without_classifying_software_as_hardware() {
        for name in [
            "d3d11h264dec",
            "vah264dec",
            "nvav1dec",
            "v4l2h264dec",
            "qsvh264dec",
        ] {
            assert!(hardware(name, "Codec/Decoder/Video"));
        }
        for name in ["avdec_h264", "vp9dec", "opusdec", "x264enc", "openh264enc"] {
            assert!(!hardware(name, "Codec/Decoder/Video"));
        }
        assert!(hardware("vendor_codec", "Codec/Decoder/Video/Hardware"));
    }
    #[test]
    fn cpu_policy_disables_registered_hardware_decoders_and_is_reversible() {
        initialize(Mode::Cpu).unwrap();
        for f in factories() {
            if hardware(f.name().as_str(), f.klass()) && f.klass().contains("Decoder") {
                assert_eq!(f.rank(), gst::Rank::NONE);
            }
        }
        apply_mode(Mode::Auto).unwrap();
        for f in factories() {
            if hardware(f.name().as_str(), f.klass()) && f.klass().contains("Decoder") {
                assert!(f.rank() >= gst::Rank::PRIMARY);
            }
        }
    }
}
