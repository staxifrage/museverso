// SPDX-License-Identifier: MPL-2.0
use ytm_servo_media::{Mode, encode_test, initialize, report};
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    initialize(Mode::Auto)?;
    match args.as_slice() {
        [] => {
            println!("{}", report());
            Ok(())
        }
        [command, file, encoder] if command == "encode-test" => {
            let mode = if encoder == "cpu" {
                Mode::Cpu
            } else {
                Mode::Auto
            };
            let used = encode_test(std::path::Path::new(file), encoder, mode)?;
            println!("Encoded {file} using {used}");
            Ok(())
        }
        _ => Err("Usage: ytm-media-check [encode-test OUTPUT.mp4 auto|cpu|ENCODER]".into()),
    }
}
