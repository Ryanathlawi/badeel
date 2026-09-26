#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod about;
mod app;
mod core;
mod i18n;
mod motion;
mod showcase;
mod theme;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    env_logger::init();
    core::update::cleanup_previous();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1240.0, 790.0])
            .with_min_inner_size([900.0, 600.0])
            .with_decorations(false)
            .with_title("badeel")
            .with_app_id("badeel"),
        ..Default::default()
    };

    eframe::run_native(
        "badeel",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
