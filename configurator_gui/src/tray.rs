//! System tray integration. "Run in Background" hides the window; the tray
//! icon (and its menu) brings it back or quits.
//!
//! Restoring cannot go through `ViewportCommand` alone: eframe only applies
//! queued viewport commands while painting a frame, frames are driven by
//! `RedrawRequested`, and Windows never delivers paint messages to a hidden
//! window — so a `Visible(true)` command queued against a hidden window is
//! never processed. The tray threads therefore call `ShowWindow` on the raw
//! HWND first (making the window visible at the OS level, which revives the
//! repaint pipeline) before asking eframe for anything. Quit needs the same
//! treatment: eframe only latches a close request during a frame, so a
//! `WM_CLOSE` posted at a hidden window would sit as a queued close event
//! and fire on the next restore instead of quitting.

#[cfg(windows)]
mod imp {
    use argb_core::settings::{EffectsMode, Settings};
    use eframe::egui;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
    use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

    const SW_RESTORE: i32 = 9;
    const WM_CLOSE: u32 = 0x0010;

    // HWND is carried as `isize` (matching raw-window-handle) rather than the
    // `*mut c_void` style of argb_core::win because the handle must cross into
    // the tray threads, and raw pointers are not Send.
    #[link(name = "user32")]
    extern "system" {
        fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
        fn PostMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> i32;
    }

    pub struct Tray {
        _icon: TrayIcon,
    }

    impl Tray {
        pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Tray, String> {
            let ctx = cc.egui_ctx.clone();
            let hwnd = match cc.window_handle().map_err(|e| e.to_string())?.as_raw() {
                RawWindowHandle::Win32(h) => h.hwnd.get(),
                _ => return Err("not a Win32 window".to_string()),
            };

            let open_item = MenuItem::with_id("open", "Open Configurator", true, None);
            let quit_item = MenuItem::with_id("quit", "Quit (lights keep running)", true, None);
            let stop_item = MenuItem::with_id("stop_quit", "⏹ Stop lights && quit", true, None);

            // Quick effect switcher: applies + saves instantly, so the daemon
            // restyles the rig on the fly without opening the window.
            let effect_menu = Submenu::new("🎨 Quick Effect", true);
            for (i, mode) in EffectsMode::ALL.iter().enumerate() {
                let item = MenuItem::with_id(format!("effect:{i}"), mode.label(), true, None);
                effect_menu.append(&item).map_err(|e| e.to_string())?;
            }

            let menu = Menu::new();
            menu.append(&open_item).map_err(|e| e.to_string())?;
            menu.append(&PredefinedMenuItem::separator()).map_err(|e| e.to_string())?;
            menu.append(&effect_menu).map_err(|e| e.to_string())?;
            menu.append(&PredefinedMenuItem::separator()).map_err(|e| e.to_string())?;
            menu.append(&quit_item).map_err(|e| e.to_string())?;
            menu.append(&stop_item).map_err(|e| e.to_string())?;

            let (rgba, w, h) = crate::util::icon_rgba(32);
            let icon = Icon::from_rgba(rgba, w, h).map_err(|e| e.to_string())?;

            let tray = TrayIconBuilder::new()
                .with_tooltip("ArgbProMaster — thermal ARGB lighting")
                .with_menu(Box::new(menu))
                .with_icon(icon)
                .build()
                .map_err(|e| e.to_string())?;

            let menu_ctx = ctx.clone();
            std::thread::spawn(move || {
                while let Ok(event) = MenuEvent::receiver().recv() {
                    let id = event.id().0.as_str();
                    if let Some(idx) = id.strip_prefix("effect:").and_then(|s| s.parse::<usize>().ok()) {
                        if let Some(mode) = EffectsMode::ALL.get(idx) {
                            // Write straight to disk: the daemon hot-reloads it
                            // within a frame, and the GUI notices the file
                            // change and refreshes itself when next visible.
                            let mut s = Settings::load_or_default();
                            s.effects_mode = *mode;
                            let _ = s.save();
                        }
                        continue;
                    }
                    match id {
                        "open" => restore(&menu_ctx, hwnd),
                        "quit" | "stop_quit" => unsafe {
                            if id == "stop_quit" {
                                crate::util::stop_daemon();
                            }
                            // eframe only latches CloseRequested inside a
                            // frame, and hidden windows never paint — show the
                            // window so the close can complete. PostMessageW
                            // can itself fail (dead hwnd at shutdown); that
                            // failure is unreportable from this thread and
                            // harmless, so its result is ignored.
                            ShowWindow(hwnd, SW_RESTORE);
                            PostMessageW(hwnd, WM_CLOSE, 0, 0);
                        },
                        _ => {}
                    }
                }
            });

            let click_ctx = ctx.clone();
            std::thread::spawn(move || {
                while let Ok(event) = TrayIconEvent::receiver().recv() {
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                        | TrayIconEvent::DoubleClick { .. } => restore(&click_ctx, hwnd),
                        _ => {}
                    }
                }
            });

            Ok(Tray { _icon: tray })
        }
    }

    fn restore(ctx: &egui::Context, hwnd: isize) {
        // OS-level show must come first; only a visible window repaints, and
        // only a painting window applies the viewport commands below.
        unsafe {
            ShowWindow(hwnd, SW_RESTORE);
            SetForegroundWindow(hwnd);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }
}

#[cfg(not(windows))]
mod imp {
    pub struct Tray;

    impl Tray {
        pub fn new(_cc: &eframe::CreationContext<'_>) -> Result<Tray, String> {
            Err("system tray is only supported on Windows".to_string())
        }
    }
}

pub use imp::Tray;
