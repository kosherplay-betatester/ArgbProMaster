//! ArgbProMaster Configurator — a polished dark-themed control panel for the
//! thermal ARGB daemon. See `thermal_daemon` for the background renderer.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod builder;
mod preview;
mod tabs;
mod theme;
mod tray;
mod util;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("ArgbProMaster Configurator")
            .with_inner_size([1200.0, 780.0])
            .with_min_inner_size([1000.0, 640.0])
            .with_icon(util::app_icon(64)),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "ArgbProMaster",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
