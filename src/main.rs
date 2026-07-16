mod app;
mod models;
mod calculator;
mod db;
mod csv_io;
mod config;

fn main() -> eframe::Result<()> {
    let cfg = config::Config::load();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Airfare Manager",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app::TaxApp::new(cfg, cc)))
        }),
    )
}
