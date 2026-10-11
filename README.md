# YTM Servo

An **experimental** Windows and Linux desktop shell that opens **https://music.youtube.com/** using the Servo browser engine. This is a native Rust/Servo application derived from Servo's reference desktop embedder, with native media settings and a dedicated browser profile.

> **Playback blocker:** the pinned Servo revision does not implement Media Source Extensions (`MediaSource` / `SourceBuffer`). YouTube's adaptive player relies on these APIs. This project is **not currently a working YouTube Music player**. Loading the page, account sign-in, subscription content and playback have not been verified. GStreamer codecs cannot fill missing browser APIs, and no Widevine/EME implementation is supplied. The app shows this limitation in its native toolbar.

## Build artifacts

| Platform | Native GitHub runner | Artifacts |
| --- | --- | --- |
| Windows x86_64 | windows-2022 | Portable ZIP with EXE, DLLs and GStreamer plugins |
| Linux x86_64 | ubuntu-24.04 | tar.gz and Debian package (amd64) |
| Linux ARM64 | ubuntu-24.04-arm | tar.gz and Debian package (arm64) |

Linux packages target Ubuntu 24.04 and compatible systems; the tarball is **not** a universal, dependency-free Linux bundle. ARM builds run natively, without QEMU. Windows ARM64 and 32-bit x86 are not targets.

## Build on GitHub Actions

1. Create or choose a GitHub repository, extract this source package into its root, and push it. Include the `.github` directory.
2. Enable Actions. Open **Actions → Build desktop binaries → Run workflow**, or push to `main`/`master`.
3. Download the three platform artifacts after the jobs succeed. A separate **Source and media checks** workflow validates the shared media implementation.
4. Each artifact includes checksums. A successful job proves compilation, packaging, GStreamer initialization and a synthetic **CPU encoding** test; it does **not** prove YouTube playback or GPU support.

The engine is pinned in `servo.lock.json`; build scripts refuse to patch a different revision or a dirty checkout. There is no automatic upstream update. Servo is a large native build: jobs may require substantial disk/RAM and take hours on standard runners. ARM hosted-runner access and usage limits depend on the repository/account. Larger runners may be needed if a job exhausts memory or disk. Workflows upload private/repository-accessible Actions artifacts, not public Releases. Binaries are unsigned.

**No GitHub Actions run has been submitted from this source package. A destination repository is required.**

## Application behavior

- Opens YouTube Music by default, with a native Home button, navigation and origin/address display.
- Preserves Servo's native pop-up/tab handling so authentication can use separate windows. Sign-in compatibility remains unverified.
- Uses a dedicated `ytm-servo/default` profile under the OS configuration directory. Media preferences are in `ytm-servo/media.json`.
- Native dropdown selects automatic decoding or CPU decoding. Changes affect newly constructed pipelines; restart in CPU mode if driver recovery is needed.
- Shows registered hardware/software codecs and GStreamer version. Registration is not proof of successful device initialization or active decoding.
- Retains Servo's GPU rendering. GPU rendering and GPU media decoding are separate capabilities.
- Uses the real GStreamer media backend; the build explicitly passes `--media-stack gstreamer`.

## Hardware and software media

Automatic mode raises the rank of registered hardware decoders while retaining software decoders. GStreamer can select software when hardware is unavailable; a hardware plugin that fails after selection may require restarting the app in CPU mode. CPU mode disables known/classified hardware decoder factories. Vendor-specific factories with unrecognized names and missing Hardware metadata may need to be added to the classifier.

Candidate implementations include Windows D3D11/D3D12, Intel QSV, NVIDIA, Linux VA/VA-API and V4L2 codecs. Support depends on the installed plugin, codec, GPU, driver, device permissions and platform. There is **no** claim that every backend works on every architecture. Most audio decoding is performed on the CPU.

YouTube Music **playback does not need encoding**. The included encoding facility creates a three-second synthetic H.264/MP4 test clip, allowing hardware encoder or CPU fallback validation without capturing or downloading streaming content. It is not a general-purpose transcoder or a WebCodecs implementation.

```sh
# Linux; use .\ytm-servo.exe on Windows
./ytm-servo --ytm-help
./ytm-servo --ytm-media=cpu
./ytm-servo --ytm-diagnostics
./ytm-servo --ytm-encode-test=codec-test.mp4 --ytm-encoder=auto
./ytm-servo --ytm-encode-test=cpu-test.mp4 --ytm-encoder=cpu
./ytm-servo --ytm-encode-test=nv-test.mp4 --ytm-encoder=nvh264enc
```

Supported explicit encoder choices: `d3d12h264enc`, `qsvh264enc`, `nvh264enc`, `vah264enc`, `vaapih264enc`, `mfh264enc`, `x264enc`, `openh264enc`. Auto tries installed hardware candidates then CPU encoders. Existing output files are never overwritten. Each candidate has a 30-second pipeline timeout and is shut down before trying the next.

Multiprocess mode is rejected because the media policy is process-local. Other Servo options, including `--config-dir PATH`, a custom URL, `--help`, and `--temporary-storage`, are retained. The decoder preference is app-global even when a custom browser profile is selected. `--temporary-storage` applies to browser data; the GUI's media preference is still saved.

## Local build

Prerequisites are the pinned Servo source's documented native toolchain. Windows requires Visual Studio 2022 C++ build tools/ATL, LLVM, Rust and Python. Linux requires a supported desktop stack and the packages installed by Servo bootstrap. Read [BUILDING.md](docs/BUILDING.md).

```sh
python scripts/prepare.py
cd vendor/servo
./mach bootstrap --yes --skip-lints --skip-nextest
./mach build --release --locked --media-stack gstreamer
cd ../..
python scripts/package.py --target linux-x86_64
# Use linux-arm64 on a native ARM64 host.
```

On Windows use `.\mach`, run `.\mach fetch` and `.\mach bootstrap-gstreamer`, and package with `--target windows-x86_64`. Runtime files must remain together. Linux `.deb` packages install dependencies through apt; tarball users must install them separately.

## Development and status

`src/lib.rs` is shared between the standalone media checker and the patched desktop shell. `shell/ytm.rs` adds native app controls. `scripts/prepare.py` applies narrowly scoped changes to the pinned embedder and preserves upstream resources and license headers. This is a downstream derivative, not a maintained fork of Servo's entire source tree.

Read [COMPATIBILITY.md](docs/COMPATIBILITY.md) for the blocking work and manual acceptance checks, and [VALIDATION.md](docs/VALIDATION.md) for what was actually tested. This app is unofficial and is not affiliated with Google, YouTube or the Servo project. License: MPL-2.0; dependencies retain their licenses. See [THIRD_PARTY.md](THIRD_PARTY.md).
