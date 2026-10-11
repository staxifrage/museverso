// SPDX-License-Identifier: MPL-2.0
use crate::ytm_media::{self as media, Mode};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

pub const HOME: &str = "https://music.youtube.com/";
#[derive(Default)]
struct Settings {
    mode: Mode,
    diagnostics: bool,
    report: String,
    error: Option<String>,
}
static SETTINGS: OnceLock<Mutex<Settings>> = OnceLock::new();
fn settings() -> &'static Mutex<Settings> {
    SETTINGS.get_or_init(|| Mutex::new(Settings::default()))
}
fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("ytm-servo").join("media.json"))
}
fn saved_mode() -> Mode {
    settings_path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|v| {
            v.get("mode")
                .and_then(|m| m.as_str())
                .and_then(|s| Mode::parse(s).ok())
        })
        .unwrap_or_default()
}
fn save_mode(mode: Mode) -> Result<(), String> {
    let path = settings_path().ok_or("No application configuration directory")?;
    std::fs::create_dir_all(path.parent().ok_or("Invalid settings path")?)
        .map_err(|e| e.to_string())?;
    std::fs::write(path, serde_json::json!({"mode": mode.as_str()}).to_string())
        .map_err(|e| e.to_string())
}

/// Consume only our namespaced options; keep Servo's content-process arguments intact.
pub fn arguments(args: Vec<String>) -> Vec<String> {
    if args
        .iter()
        .any(|a| a == "--content-process" || a.starts_with("--content-process="))
    {
        return args;
    }
    if args.iter().any(|a| a == "-M" || a == "--multiprocess") {
        fail(
            "This build uses a single process so decoder policy applies consistently; --multiprocess is not supported",
        );
    }
    let mut mode = saved_mode();
    let mut diagnostics = false;
    let mut output = None;
    let mut encoder = String::from("auto");
    let mut result = Vec::new();
    for arg in args {
        if arg == "--ytm-help" {
            println!(
                "YTM Servo (experimental)\n\n--ytm-media=auto|cpu   Decoder preference (auto keeps software fallback)\n--ytm-diagnostics     Print registered codecs and exit\n--ytm-encode-test=PATH.mp4  Encode a 3-second synthetic clip, never overwrite\n--ytm-encoder=auto|cpu|NAME  Encoder for that test\n\nServo options such as --config-dir PATH and a URL are also accepted.\nYouTube Music playback is blocked by missing upstream Media Source Extensions."
            );
            std::process::exit(0);
        } else if let Some(value) = arg.strip_prefix("--ytm-media=") {
            mode = Mode::parse(value).unwrap_or_else(|e| fail(&e));
        } else if arg == "--ytm-diagnostics" {
            diagnostics = true;
        } else if let Some(value) = arg.strip_prefix("--ytm-encode-test=") {
            output = Some(PathBuf::from(value));
        } else if let Some(value) = arg.strip_prefix("--ytm-encoder=") {
            encoder = value.to_string();
        } else {
            result.push(arg);
        }
    }
    if encoder != "auto" && output.is_none() {
        fail("--ytm-encoder requires --ytm-encode-test");
    }
    settings().lock().unwrap().mode = mode;
    if diagnostics || output.is_some() {
        media::initialize(mode).unwrap_or_else(|e| fail(&e));
        if diagnostics {
            println!(
                "YTM Servo: MSE unavailable; YouTube Music playback is not supported yet.\n{}",
                media::report()
            );
        }
        if let Some(path) = output {
            let selected = media::encode_test(&path, &encoder, mode).unwrap_or_else(|e| fail(&e));
            println!("Encoded {} with {selected}", path.display());
        }
        std::process::exit(0);
    }
    result
}
fn fail(message: &str) -> ! {
    eprintln!("YTM Servo: {message}");
    std::process::exit(2)
}

pub fn initialize() {
    let mut state = settings().lock().unwrap();
    state.error = media::initialize(state.mode).err();
}

pub fn toolbar(ui: &mut egui::Ui) -> bool {
    let mut state = settings().lock().unwrap();
    let mut go_home = false;
    ui.horizontal_wrapped(|ui| {
        ui.strong("YTM Servo");
        go_home = ui.button("YouTube Music").clicked();
        ui.separator();
        let previous = state.mode;
        egui::ComboBox::from_id_salt("ytm_media_mode")
            .selected_text(match state.mode {
                Mode::Auto => "Decoding: automatic",
                Mode::Cpu => "Decoding: CPU",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.mode, Mode::Auto, "Automatic · prefer hardware");
                ui.selectable_value(&mut state.mode, Mode::Cpu, "CPU · software decoding");
            });
        if previous != state.mode {
            state.error = media::apply_mode(state.mode)
                .and_then(|_| save_mode(state.mode))
                .err();
        }
        if ui.button("Media diagnostics").clicked() {
            state.report = media::report();
            state.diagnostics = !state.diagnostics;
        }
        ui.label("Decoder changes apply to new streams; restart for recovery.");
    });
    ui.colored_label(egui::Color32::from_rgb(255, 190, 90),
        "Experimental: Servo lacks Media Source Extensions. YouTube Music playback is not supported yet.");
    if let Some(error) = &state.error {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
    if state.diagnostics {
        egui::ScrollArea::vertical()
            .max_height(240.0)
            .show(ui, |ui| {
                ui.monospace(&state.report);
            });
    }
    go_home
}
