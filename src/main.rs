pub mod config;
pub mod device;
pub mod input;
pub mod render;
pub mod ui;

use ui::StreamDeckApp;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([980.0, 660.0])
            .with_min_inner_size([800.0, 520.0])
            .with_title("StreamDeck (Wayland Native)")
            .with_app_id("streamdeck"),
        ..Default::default()
    };

    eframe::run_native(
        "StreamDeck",
        native_options,
        Box::new(|cc| Ok(Box::new(StreamDeckApp::new(cc)))),
    )
}
