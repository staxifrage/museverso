// Compile the exact native toolbar module against the pinned egui API.
#[allow(dead_code)]
#[path = "../shell/ytm.rs"]
mod ytm;
#[allow(dead_code)]
#[path = "../src/lib.rs"]
mod ytm_media;

#[test]
fn content_process_arguments_are_not_rewritten() {
    let args = vec!["--content-process".into(), "ipc-token".into()];
    assert_eq!(ytm::arguments(args.clone()), args);
}

#[test]
fn toolbar_renders_with_the_real_egui_api() {
    let context = egui::Context::default();
    let _ = context.run_ui(egui::RawInput::default(), |ui| {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            assert!(!ytm::toolbar(ui));
        });
    });
}
