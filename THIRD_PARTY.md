# Third-party components

- Servo: Mozilla Public License 2.0, https://github.com/servo/servo . The exact revision is in servo.lock.json; scripts/prepare.py and the shell/media source contain the downstream changes. Retain Servo's license headers and make the corresponding modified source available with distributed binaries.
- GStreamer and its Rust bindings: https://gstreamer.freedesktop.org/ and https://gitlab.freedesktop.org/gstreamer/gstreamer-rs . Components and plugins carry their own licenses. Windows packaging copies available license/documentation directories from the installed distribution. Linux uses distribution packages and their license records.
- The Windows full plugin bundle can contain LGPL, GPL and other licensed plugins. Review the actual packaged plugin set and satisfy its notices/corresponding-source obligations before public redistribution. Codec patent/licensing requirements depend on the codec and jurisdiction.
- MSVC runtime and ANGLE DLLs copied by Servo's build retain their own distribution terms. Servo's locked Cargo dependencies retain their respective licenses.

YTM Servo is an unofficial downstream application. YouTube and YouTube Music are Google trademarks. The bundled icon is an original generic musical-note graphic; no Google product assets are redistributed.
