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

// ---------------------------------------------------------------------------
// Setup assistant: find / start / install the software we depend on
// ---------------------------------------------------------------------------

/// Locate an installed OpenRGB.exe in the usual places.
pub fn find_openrgb() -> Option<std::path::PathBuf> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Ok(base) = std::env::var(var) {
            candidates.push(std::path::PathBuf::from(&base).join("OpenRGB").join("OpenRGB.exe"));
            candidates.push(
                std::path::PathBuf::from(&base)
                    .join("Programs")
                    .join("OpenRGB")
                    .join("OpenRGB.exe"),
            );
        }
    }
    candidates.into_iter().find(|p| p.exists()).or_else(|| {
        // Fall back to whatever is on PATH (covers winget shims).
        let out = std::process::Command::new("where.exe").arg("OpenRGB").output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let line = text.lines().next()?.trim();
        if line.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from(line))
        }
    })
}

/// Start OpenRGB elevated with the SDK server on — exactly the configuration
/// the daemon needs. Elevation (one UAC prompt) lets it reach RAM/SMBus.
pub fn start_openrgb(path: &std::path::Path) -> Result<(), String> {
    let script = format!(
        "Start-Process -FilePath '{}' -ArgumentList '--server','--startminimized' -Verb RunAs",
        path.display()
    );
    run_hidden_powershell(&script)
}

/// The default MSI Afterburner install location.
pub fn find_afterburner() -> Option<std::path::PathBuf> {
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Ok(base) = std::env::var(var) {
            let p = std::path::PathBuf::from(base)
                .join("MSI Afterburner")
                .join("MSIAfterburner.exe");
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

pub fn start_afterburner(path: &std::path::Path) -> Result<(), String> {
    // Afterburner's manifest requests elevation itself; go through the shell
    // so the UAC prompt appears instead of an access-denied error.
    let script = format!("Start-Process -FilePath '{}'", path.display());
    run_hidden_powershell(&script)
}

fn run_hidden_powershell(script: &str) -> Result<(), String> {
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", script]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Is MSI Afterburner publishing its shared memory right now?
#[cfg(windows)]
pub fn afterburner_running() -> bool {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenFileMappingW(access: u32, inherit: i32, name: *const u16) -> *mut c_void;
        fn CloseHandle(h: *mut c_void) -> i32;
    }
    let name: Vec<u16> = "MAHMSharedMemory".encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let h = OpenFileMappingW(0x0004, 0, name.as_ptr());
        if h.is_null() {
            false
        } else {
            CloseHandle(h);
            true
        }
    }
}

#[cfg(not(windows))]
pub fn afterburner_running() -> bool {
    false
}

/// Install a package with winget on a background thread; reports the result
/// (package display name, success/error) through the channel when done.
pub fn winget_install(
    id: &'static str,
    display: &'static str,
    tx: std::sync::mpsc::Sender<(&'static str, Result<(), String>)>,
) {
    std::thread::spawn(move || {
        let result = std::process::Command::new("winget")
            .args([
                "install",
                "-e",
                "--id",
                id,
                "--silent",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ])
            .output();
        let outcome = match result {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout);
                let tail: String = text.lines().rev().take(3).collect::<Vec<_>>().join(" | ");
                Err(if tail.trim().is_empty() { format!("exit code {:?}", out.status.code()) } else { tail })
            }
            Err(e) => Err(format!("couldn't run winget: {e}")),
        };
        let _ = tx.send((display, outcome));
    });
}

pub fn open_settings_folder() {
    let dir = argb_core::settings::settings_dir();
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(&dir).spawn();
    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
}
