mod app;
mod colors;
mod keyboard;
mod pixel_logo;
mod renderer;
mod tab;

use app::TheWorstApp;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("The-Worst")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([400.0, 200.0]),
        ..Default::default()
    };

    eframe::run_native(
        "The-Worst",
        options,
        Box::new(|_cc| Ok(Box::new(TheWorstApp::new()))),
    )
}
