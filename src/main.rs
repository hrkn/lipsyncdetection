mod analysis;
mod audio;
mod media;
mod ui;
mod utils;

use ui::SyncDetectorApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([800.0, 550.0])
            .with_title("Media & Lip Sync Detector"),
        ..Default::default()
    };

    eframe::run_native(
        "Media & Lip Sync Detector",
        native_options,
        Box::new(|cc| Ok(Box::new(SyncDetectorApp::new(cc)))),
    )
}
