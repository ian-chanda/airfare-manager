#![windows_subsystem = "windows"]

mod app;
mod models;
mod calculator;
mod db;
mod csv_io;
mod config;

fn load_icon_from_memory(bytes: &[u8]) -> egui::IconData {
    let image = image::load_from_memory(bytes)
        .expect("Failed to open icon path")
        .into_rgba8();
    let (width, height) = image.dimensions();
    let rgba = image.into_raw();
    
    egui::IconData { rgba, width, height }
}

fn main() -> eframe::Result<()> {
    let cfg = config::Config::load();
    let icon_bytes = include_bytes!("../assets/icon.ico");
    let icon = load_icon_from_memory(icon_bytes);
    
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0])
        .with_icon(icon),

        vsync: false,
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

