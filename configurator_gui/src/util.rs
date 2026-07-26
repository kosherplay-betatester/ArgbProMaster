//! Odds and ends: procedural app icon, daemon process control.

use eframe::egui;

/// Procedurally draw the app icon: an RGB spectrum ring on a dark disc.
pub fn icon_rgba(size: u32) -> (Vec<u8>, u32, u32) {
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    let center = (size as f32 - 1.0) / 2.0;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt() / center;
            let angle = dy.atan2(dx).to_degrees() + 180.0;
            let px: [u8; 4] = if dist <= 1.0 {
                if dist >= 0.52 {
                    let c = argb_core::engine::hsv_to_rgb(angle, 1.0, 1.0);
                    let edge = ((1.0 - dist) / 0.10).clamp(0.0, 1.0);
                    [c[0] as u8, c[1] as u8, c[2] as u8, (edge * 255.0) as u8]
                } else {
                    [16, 18, 28, 255]
                }
            } else {
                [0, 0, 0, 0]
            };
            rgba.extend_from_slice(&px);
        }
    }
    (rgba, size, size)
}

pub fn app_icon(size: u32) -> egui::IconData {
    let (rgba, width, height) = icon_rgba(size);
    egui::IconData {
        rgba,
        width,
        height,
    }
}

/// Launch `thermal_daemon.exe` from next to the configurator executable.
/// The daemon's named-mutex guard makes duplicate launches harmless.
pub fn spawn_daemon() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or_else(|| "executable has no parent directory".to_string())?;
    let daemon = dir.join(if cfg!(windows) { "thermal_daemon.exe" } else { "thermal_daemon" });
    if !daemon.exists() {
        return Err(format!("daemon not found at {}", daemon.display()));
    }
    std::process::Command::new(daemon)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(windows)]
pub fn stop_daemon() {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = std::process::Command::new("taskkill")
        .args(["/IM", "thermal_daemon.exe", "/F"])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

#[cfg(not(windows))]
pub fn stop_daemon() {}

pub fn open_settings_folder() {
    let dir = argb_core::settings::settings_dir();
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(&dir).spawn();
    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
}
