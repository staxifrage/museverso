# Validation performed

Date: 2026-10-11. Local test host: Linux x86_64, Rust 1.99.0, GStreamer 1.24.2. The hosted build selects Servo's pinned Rust 1.97.1 instead.

## Passed locally

- Inspected the pinned Servo embedder, media initialization, GStreamer dependencies, build commands, platform packaging paths and upstream CI.
- Applied the complete patch to pristine files from the pinned commit. Verified the TOML lockfile, direct dependency edge, actual CLI default URL, profile separation, and byte-for-byte inclusion of shared media code.
- Four Python tests for immutable revision metadata, patch-anchor failure behavior and script parsing.
- Rust shared media crate compilation against GStreamer 0.25.4, matching the pinned Servo version.
- Native toolbar module compilation and layout execution against egui 0.34.3, matching Servo's pinned API.
- Eight Rust tests (three media cases also compiled/run in the shell integration target): decoder classification, invalid mode rejection, registered decoder rank transitions, content-process argument preservation and native toolbar rendering.
- `cargo fmt -- --check` and `cargo clippy --locked --all-targets -- -D warnings`.
- Actual synthetic H.264/MP4 encoding with `x264enc` on the CPU, not a mocked pipeline.
- Existing output rejection; SHA-256 was unchanged after the rejected overwrite.
- GitHub Actions workflow syntax checked with actionlint 1.7.7.

## Not performed / still blocked

- Full Servo application compilation and linking on any of the three target platforms.
- A GitHub Actions run, because no destination repository was supplied.
- Installation or launch of the full packaged desktop app on Windows/Linux.
- Actual GPU decode/encode testing, consumer driver validation, or ARM64 runtime testing.
- YouTube Music page loading, login, playback, seeking, account persistence or subscription-content checks.
- Production release readiness, signing and a completed redistribution-license audit.

The pinned engine lacks Media Source Extensions. The desktop code and workflows are an experimental implementation, not evidence of a working YouTube Music player. Synthetic codec tests cannot establish web-app compatibility.
