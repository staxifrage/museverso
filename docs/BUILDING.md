# Building and packaging

The workflow builds the exact engine revision in `servo.lock.json`. Upstream `rust-toolchain.toml` selects Rust 1.97.1. Its Cargo.lock stays locked except for the direct `gstreamer` dependency edge added to the shell; the dependency already exists in the engine lockfile. The standalone media crate has its own Cargo.lock.

The patch is intentionally fail-closed: it verifies HEAD, rejects dirty worktrees and requires unique source anchors. To regenerate after changing this project's source, use a new clean Servo checkout or explicitly restore your own disposable build checkout first. The preparation tool does not delete existing code for you.

## Linux

Use a native Ubuntu 24.04 x86_64 or ARM64 host. Install Rust, Git, Python 3.11 and uv, then LLVM/Clang 20.1. Set LIBCLANG_PATH to LLVM's lib directory. Run the preparation and build commands in README. Servo's bootstrap installs system build dependencies. Ensure runtime codec packages are installed too:

```
sudo apt-get install gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
  gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav \
  mesa-va-drivers dpkg-dev
```

Install a produced `.deb` with `sudo apt install ./ytm-servo-linux-x86_64.deb` (or ARM64 filename). For tarballs, install the dependencies in `packaging/control.in`; extract the archive and run `./ytm-servo` inside it. Libraries are intentionally supplied by the distribution, including GStreamer plugins and GPU drivers.

## Windows

Use x64 Windows with Visual Studio 2022 C++ build tools, MSVC x64/x86 and ATL components, Rust, Python 3.11, uv, and LLVM/Clang 20.1. Set LIBCLANG_PATH to LLVM's bin directory. From the prepared Servo checkout:

```
.\mach fetch
.\mach bootstrap-gstreamer
.\mach build --release --locked --media-stack gstreamer
```

`python scripts/package.py --target windows-x86_64` runs from the project root. It copies the engine's runtime DLLs and full GStreamer plugin set into the portable directory. This includes plugins absent from Servo's minimal playback bundle. It resolves GStreamer from Servo's target/dependencies directory, the standard environment variable, then the standard installation directory. Keep DLLs, resources and EXE together.

## Resource limits and updates

The workflow uses two compile jobs, disables incremental artifacts and release debug information, and has a six-hour job timeout. No shared compiler cache is required. If standard runners exhaust resources, move the same native build job to an appropriately sized runner rather than dropping media features or silently skipping a target.

To update Servo: inspect engine/API/media changes, change the immutable revision and Rust version together, update patch anchors, re-run the full matrix and all acceptance tests. Do not infer compatibility from a build alone. The current native compatibility banner must be changed only after verified support.

No credentials, OAuth client secrets, external API keys, browser-cookie imports or service-account configuration are needed to build. Website sign-in, when the engine supports it, remains inside the web app.
