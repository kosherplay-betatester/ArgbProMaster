use crate::{builder, preview, tabs, theme, tray, util};
use argb_core::settings::{CustomEffect, Settings};
use argb_core::zones::DetectedZone;
use eframe::egui::{self, Color32, CornerRadius, Margin, RichText};
use std::collections::HashSet;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Presets,
    Ports,
    Curves,
    Lab,
    Advanced,
}

impl Tab {
    const ALL: [(Tab, &'static str); 5] = [
        (Tab::Presets, "🎨 Presets & Themes"),
        (Tab::Ports, "🔌 Zones & Ports"),
        (Tab::Curves, "🌡 Thermal Curves"),
        (Tab::Lab, "🧪 Effect Lab"),
        (Tab::Advanced, "⚙ Advanced"),
    ];

    fn describe(self) -> &'static str {
        match self {
            Tab::Presets => "One-click looks — apply a ready-made style or save your own.",
            Tab::Ports => "Your hardware: every port and device, what it reacts to, its effect and colors.",
            Tab::Curves => "Temperature ranges, the color gradient, the global effect, idle mode and effect tuning.",
            Tab::Lab => "Invent your own effects from building blocks, with a live preview.",
            Tab::Advanced => "FPS, smoothing, brightness safety — and the fix-it buttons.",
        }
    }
}

pub struct App {
    pub settings: Settings,
    pub saved: Settings,
    pub tab: Tab,
    // live preview simulation
    pub sim_cpu: f32,
    pub sim_gpu: f32,
    pub sim_cpu_smooth: f32,
    pub sim_gpu_smooth: f32,
    /// Mock value (0..100 %) for the non-temperature sources (loads/RAM/FPS).
    pub sim_other: f32,
    /// Latest real sensor readings while "follow real temperatures" is on.
    pub live_readings: Option<argb_core::afterburner::Readings>,
    // custom presets
    pub new_preset_name: String,
    // Effect Lab editor state
    pub effect_draft: CustomEffect,
    // zone detection (background scan of the OpenRGB server)
    pub detect_rx: Option<mpsc::Receiver<Result<Vec<DetectedZone>, String>>>,
    pub detected_devices: HashSet<String>,
    detect_autorun: bool,
    // toast: message, color, born-at
    status: Option<(String, Color32, Instant)>,
    // daemon status polling
    daemon_running: bool,
    last_daemon_check: Instant,
    // settings.json mtime, so tray quick-switches (which write the file
    // directly) refresh the editor instead of being clobbered by stale state
    settings_stamp: Option<std::time::SystemTime>,
    // setup assistant: detect + offer to install/start missing software
    setup: SetupState,
    // "restore original lighting" background operation
    restore_rx: Option<mpsc::Receiver<Result<usize, String>>>,
    // live preview temperature source: mock sliders or real sensors
    pub live_temps: bool,
    mahm: Option<argb_core::afterburner::MahmReader>,
    _tray: Option<tray::Tray>,
}

/// What the setup assistant currently knows about the software around us.
struct SetupState {
    openrgb_connected: bool,
    openrgb_path: Option<std::path::PathBuf>,
    afterburner_ok: bool,
    installing: Option<&'static str>,
    install_rx: Option<mpsc::Receiver<(&'static str, Result<(), String>)>>,
    /// Re-run zone detection shortly after we auto-start OpenRGB.
    reprobe_at: Option<Instant>,
    dismissed: bool,
}

impl SetupState {
    fn new() -> SetupState {
        SetupState {
            // Optimistic until the first scan says otherwise — avoids a
            // one-second "not reachable" flash on every healthy launch.
            openrgb_connected: true,
            openrgb_path: util::find_openrgb(),
            afterburner_ok: util::afterburner_running(),
            installing: None,
            install_rx: None,
            reprobe_at: None,
            dismissed: false,
        }
    }

    fn needs_attention(&self) -> bool {
        !self.dismissed
            && (self.installing.is_some() || !self.openrgb_connected || !self.afterburner_ok)
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        theme::apply(&cc.egui_ctx);
        let settings = Settings::load_or_default();
        let (tray, tray_error) = match tray::Tray::new(cc) {
            Ok(t) => (Some(t), None),
            Err(e) => (None, Some(e)),
        };
        App {
            saved: settings.clone(),
            settings,
            tab: Tab::Presets,
            sim_cpu: 55.0,
            sim_gpu: 45.0,
            sim_cpu_smooth: 55.0,
            sim_gpu_smooth: 45.0,
            sim_other: 40.0,
            live_readings: None,
            new_preset_name: String::new(),
            effect_draft: CustomEffect::default(),
            detect_rx: None,
            detected_devices: HashSet::new(),
            detect_autorun: false,
            status: tray_error.map(|e| {
                (
                    format!("Tray unavailable ({e}) — Run in Background will minimize instead."),
                    theme::WARN,
                    Instant::now(),
                )
            }),
            daemon_running: argb_core::win::daemon_running(),
            last_daemon_check: Instant::now(),
            settings_stamp: argb_core::settings::settings_mtime(),
            setup: SetupState::new(),
            restore_rx: None,
            live_temps: false,
            mahm: None,
            _tray: tray,
        }
    }

    /// Stop the daemon and hand every device back to its own built-in effect
    /// (the look it shipped with before any software touched it).
    pub fn restore_hardware_lighting(&mut self) {
        if self.restore_rx.is_some() {
            return;
        }
        util::stop_daemon();
        let (tx, rx) = mpsc::channel();
        self.restore_rx = Some(rx);
        std::thread::spawn(move || {
            let result = (|| -> Result<usize, String> {
                // Give the daemon a moment to die, so its mode guard can't
                // steal the devices back into Direct behind us.
                std::thread::sleep(Duration::from_millis(800));
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 6742));
                let mut client =
                    argb_core::openrgb::OpenRgbClient::connect(addr, "ArgbProMaster restore")
                        .map_err(|e| e.to_string())?;
                let count = client.controller_count().map_err(|e| e.to_string())?;
                let mut restored = 0usize;
                for device in 0..count {
                    let Ok(info) = client.controller_data(device) else { continue };
                    if let Some((idx, mode)) = info.firmware_mode() {
                        let mode = mode.clone();
                        if client.update_mode(device, idx, &mode).is_ok() {
                            restored += 1;
                        }
                    }
                }
                Ok(restored)
            })();
            let _ = tx.send(result);
        });
    }

    pub fn toast(&mut self, message: String, color: Color32) {
        self.status = Some((message, color, Instant::now()));
    }

    pub fn restore_running(&self) -> bool {
        self.restore_rx.is_some()
    }

    /// 🔧 The panic button: put EVERYTHING back into a known-good, working
    /// state — stock settings, every detected zone switched on, Thermal Alert
    /// applied, daemon running. From there the user shapes it at will.
    pub fn fix_my_rgb(&mut self) {
        let kept_presets = self.settings.custom_presets.clone();
        let kept_zones = self.settings.zones.clone();
        self.settings = Settings::default();
        self.settings.custom_presets = kept_presets;
        self.settings.zones = kept_zones;
        let mut enabled = 0usize;
        for zone in self.settings.zones.iter_mut() {
            zone.effect_override = None;
            zone.custom_effect = None;
            zone.colors_override = None;
            if zone.effective_leds() > 0 {
                zone.enabled = true;
                enabled += 1;
            }
        }
        argb_core::presets::apply_builtin("Thermal Alert", &mut self.settings);
        if self.save_settings() {
            let _ = util::spawn_daemon();
            self.start_detection();
            self.toast(
                format!(
                    "🔧 Fixed! Stock defaults restored, {enabled} zone(s) switched on, daemon running — \
                     Thermal Alert is live. Now shape it however you like."
                ),
                theme::OK,
            );
        }
    }

    /// Kick off a background scan of the OpenRGB server. Runs on a thread so
    /// the UI never freezes; results are folded in by `poll_detection`.
    pub fn start_detection(&mut self) {
        if self.detect_rx.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.detect_rx = Some(rx);
        std::thread::spawn(move || {
            let result = (|| {
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 6742));
                let mut client = argb_core::openrgb::OpenRgbClient::connect(addr, "ArgbProMaster GUI")
                    .map_err(|e| e.to_string())?;
                argb_core::zones::detect(&mut client).map_err(|e| e.to_string())
            })();
            let _ = tx.send(result);
        });
    }

    fn poll_detection(&mut self) {
        let Some(rx) = &self.detect_rx else { return };
        let Ok(result) = rx.try_recv() else { return };
        self.detect_rx = None;
        match result {
            Ok(detected) => {
                self.setup.openrgb_connected = true;
                self.detected_devices = detected.iter().map(|d| d.device_name.clone()).collect();
                let clean_before = self.settings == self.saved;
                argb_core::zones::merge(&mut self.settings.zones, &detected);
                argb_core::zones::merge(&mut self.saved.zones, &detected);
                // Detection metadata alone shouldn't nag "Unsaved changes";
                // persist it quietly so the daemon sees concretized zones too.
                if clean_before && self.settings == self.saved {
                    let _ = self.settings.save();
                }
                self.toast(
                    format!(
                        "Found {} zone(s) across {} device(s) — flip them on in Zones & Ports.",
                        self.settings.zones.len(),
                        self.detected_devices.len()
                    ),
                    theme::OK,
                );
            }
            Err(e) => {
                self.setup.openrgb_connected = false;
                self.setup.openrgb_path = util::find_openrgb();
                self.toast(
                    format!("Couldn't scan for zones: {e}. Is OpenRGB running with its SDK server on?"),
                    theme::WARN,
                );
            }
        }
    }

    /// The setup assistant banner: shows what's missing and fixes it in one
    /// click — install via winget, or start OpenRGB with the right flags.
    fn setup_banner(&mut self, ctx: &egui::Context) {
        // Fold in finished installs.
        if let Some(rx) = &self.setup.install_rx {
            if let Ok((name, result)) = rx.try_recv() {
                self.setup.install_rx = None;
                self.setup.installing = None;
                match result {
                    Ok(()) => {
                        self.toast(format!("{name} installed! Setting it up…"), theme::OK);
                        self.setup.openrgb_path = util::find_openrgb();
                        if name == "OpenRGB" {
                            if let Some(path) = self.setup.openrgb_path.clone() {
                                let _ = util::start_openrgb(&path);
                                self.setup.reprobe_at = Some(Instant::now() + Duration::from_secs(10));
                            }
                        } else if let Some(path) = util::find_afterburner() {
                            let _ = util::start_afterburner(&path);
                        }
                    }
                    Err(e) => self.toast(
                        format!("{name} install didn't finish: {e}. You can install it manually and click Detect."),
                        theme::DANGER,
                    ),
                }
            }
        }
        // Re-probe after we auto-started OpenRGB.
        if let Some(at) = self.setup.reprobe_at {
            if Instant::now() >= at {
                self.setup.reprobe_at = None;
                self.start_detection();
            }
        }

        if !self.setup.needs_attention() {
            return;
        }
        egui::TopBottomPanel::top("setup_banner")
            .frame(
                egui::Frame::new()
                    .fill(theme::CARD)
                    .inner_margin(Margin::symmetric(16, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if let Some(name) = self.setup.installing {
                        ui.spinner();
                        ui.label(
                            RichText::new(format!(
                                "Installing {name} for you — this can take a minute or two…"
                            ))
                            .color(theme::ACCENT),
                        );
                        return;
                    }
                    if !self.setup.openrgb_connected {
                        ui.label(RichText::new("⚡").size(16.0));
                        if self.setup.openrgb_path.is_none() {
                            ui.label("OpenRGB isn't installed — it's the engine that talks to your LEDs. I can install and set it up for you.");
                            if ui.button("🚀 Install OpenRGB for me").on_hover_text("Installs OpenRGB via winget, then starts it with the SDK server on — fully automatic.").clicked() {
                                let (tx, rx) = mpsc::channel();
                                self.setup.install_rx = Some(rx);
                                self.setup.installing = Some("OpenRGB");
                                util::winget_install("OpenRGB.OpenRGB", "OpenRGB", tx);
                            }
                        } else {
                            ui.label("OpenRGB is installed but not reachable — one click starts it correctly (SDK server on, admin).");
                            if ui.button("▶ Start OpenRGB for me").on_hover_text("Launches OpenRGB with --server --startminimized. Click Yes on the Windows prompt.").clicked() {
                                if let Some(path) = self.setup.openrgb_path.clone() {
                                    match util::start_openrgb(&path) {
                                        Ok(()) => {
                                            self.toast("Starting OpenRGB — rescanning your zones in a few seconds…".to_string(), theme::OK);
                                            self.setup.reprobe_at = Some(Instant::now() + Duration::from_secs(8));
                                        }
                                        Err(e) => self.toast(format!("Couldn't start OpenRGB: {e}"), theme::DANGER),
                                    }
                                }
                            }
                        }
                    } else if !self.setup.afterburner_ok {
                        ui.label(RichText::new("🌡").size(16.0));
                        if let Some(path) = util::find_afterburner() {
                            ui.label("MSI Afterburner isn't running — without it, temperatures can't update.");
                            if ui.button("▶ Start MSI Afterburner").clicked() {
                                match util::start_afterburner(&path) {
                                    Ok(()) => self.toast("Starting MSI Afterburner…".to_string(), theme::OK),
                                    Err(e) => self.toast(format!("Couldn't start Afterburner: {e}"), theme::DANGER),
                                }
                            }
                        } else {
                            ui.label("MSI Afterburner isn't installed — it provides the CPU/GPU temperatures. I can install it for you.");
                            if ui.button("🚀 Install Afterburner for me").on_hover_text("Installs MSI Afterburner via winget and starts it — fully automatic.").clicked() {
                                let (tx, rx) = mpsc::channel();
                                self.setup.install_rx = Some(rx);
                                self.setup.installing = Some("MSI Afterburner");
                                util::winget_install("Guru3D.Afterburner", "MSI Afterburner", tx);
                            }
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✕").on_hover_text("Hide this helper for now.").clicked() {
                            self.setup.dismissed = true;
                        }
                    });
                });
            });
    }

    fn save_settings(&mut self) -> bool {
        self.settings.normalize();
        match self.settings.save() {
            Ok(()) => {
                self.saved = self.settings.clone();
                self.settings_stamp = argb_core::settings::settings_mtime();
                true
            }
            Err(e) => {
                self.toast(format!("Failed to save settings: {e}"), theme::DANGER);
                false
            }
        }
    }

    /// Fold in settings.json changes made outside this window (tray quick
    /// switches, hand edits) — but never over the user's unsaved edits.
    fn reload_if_changed_on_disk(&mut self) {
        let stamp = argb_core::settings::settings_mtime();
        if stamp == self.settings_stamp {
            return;
        }
        self.settings_stamp = stamp;
        if self.settings == self.saved {
            let fresh = Settings::load_or_default();
            self.settings = fresh.clone();
            self.saved = fresh;
        }
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .inner_margin(Margin::symmetric(16, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("ArgbProMaster").heading().strong().color(theme::ACCENT));
                    ui.label(
                        RichText::new("Thermal ARGB Studio — your lights follow your temperatures")
                            .small()
                            .color(theme::TEXT_DIM),
                    );
                });
                ui.add_space(6.0);
                // Wrapped so all five tabs stay reachable on narrow windows.
                ui.horizontal_wrapped(|ui| {
                    for (tab, label) in Tab::ALL {
                        let selected = self.tab == tab;
                        let text = if selected {
                            RichText::new(label).strong().color(Color32::WHITE)
                        } else {
                            RichText::new(label).color(theme::TEXT_DIM)
                        };
                        let mut button = egui::Button::new(text)
                            .corner_radius(CornerRadius::same(8));
                        button = if selected {
                            button.fill(theme::CARD).stroke(egui::Stroke::new(1.0, theme::ACCENT))
                        } else {
                            button.fill(Color32::TRANSPARENT)
                        };
                        if ui.add(button).on_hover_text(tab.describe()).clicked() {
                            self.tab = tab;
                        }
                    }
                });
                ui.add_space(2.0);
            });
    }

    fn bottom_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("actions")
            .frame(
                egui::Frame::new()
                    .fill(theme::BG)
                    .inner_margin(Margin::symmetric(16, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Daemon status indicator
                    let (dot, label) = if self.daemon_running {
                        (theme::OK, "Daemon active")
                    } else {
                        (theme::TEXT_DIM, "Daemon stopped")
                    };
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, dot);
                    ui.label(RichText::new(label).small().color(theme::TEXT_DIM));

                    if self.settings != self.saved {
                        ui.add_space(8.0);
                        theme::chip(ui, "Unsaved changes", theme::WARN);
                    }

                    if let Some((message, color, born)) = &self.status {
                        if born.elapsed() < Duration::from_secs(4) {
                            ui.add_space(8.0);
                            ui.label(RichText::new(message).small().color(*color));
                        } else {
                            self.status = None;
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let apply = egui::Button::new(
                            RichText::new("✔ Apply & Save").strong().color(Color32::BLACK),
                        )
                        .fill(theme::ACCENT)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(egui::vec2(140.0, 34.0));
                        if ui
                            .add(apply)
                            .on_hover_text("Saves, makes sure the daemon is running, and slips into the tray — your lights keep going. Quit fully from the tray menu.")
                            .clicked()
                            && self.save_settings()
                        {
                            let _ = util::spawn_daemon();
                            if self._tray.is_some() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                            } else {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                            }
                        }

                        let run_bg = egui::Button::new(RichText::new("🚀 Run in Background"))
                            .corner_radius(CornerRadius::same(8))
                            .min_size(egui::vec2(160.0, 34.0));
                        if ui
                            .add(run_bg)
                            .on_hover_text("Saves, makes sure the daemon is running, and minimizes quietly to the system tray.")
                            .clicked()
                            && self.save_settings()
                        {
                            match util::spawn_daemon() {
                                Ok(()) => {
                                    if self._tray.is_some() {
                                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                                    } else {
                                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                                    }
                                }
                                Err(e) => self.toast(
                                    format!("Saved, but couldn't start the daemon: {e}"),
                                    theme::DANGER,
                                ),
                            }
                        }

                        let save_only = egui::Button::new(RichText::new("💾 Save"))
                            .corner_radius(CornerRadius::same(8))
                            .min_size(egui::vec2(90.0, 34.0));
                        if ui
                            .add(save_only)
                            .on_hover_text("Saves settings.json without closing — the running daemon hot-reloads it instantly.")
                            .clicked()
                            && self.save_settings()
                        {
                            self.toast("Settings saved — daemon picks them up live.".to_string(), theme::OK);
                        }
                    });
                });
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Live preview temps: follow the real sensors when asked to, so the
        // preview animates exactly like the LEDs do right now.
        if self.live_temps {
            if self.mahm.is_none() {
                self.mahm = argb_core::afterburner::MahmReader::open();
            }
            if let Some(readings) = self.mahm.as_ref().and_then(|m| m.read_all()) {
                if let Some(c) = readings.cpu_temp {
                    self.sim_cpu = c;
                }
                if let Some(g) = readings.gpu_temp {
                    self.sim_gpu = g;
                }
                self.live_readings = Some(readings);
            }
        } else {
            self.mahm = None;
            self.live_readings = None;
        }

        // Report a finished "restore original lighting" run.
        if let Some(rx) = &self.restore_rx {
            if let Ok(result) = rx.try_recv() {
                self.restore_rx = None;
                match result {
                    Ok(n) => self.toast(
                        format!(
                            "✨ {n} device(s) handed back to their built-in lighting — daemon stopped. \
                             Press Apply & Save (or ▶ Start Daemon) whenever you want ArgbProMaster back."
                        ),
                        theme::OK,
                    ),
                    Err(e) => self.toast(
                        format!("Couldn't restore hardware lighting: {e}. Is OpenRGB running?"),
                        theme::DANGER,
                    ),
                }
            }
        }

        // Smooth the simulated temperatures with the same easing feel the
        // daemon applies per frame (scaled by real elapsed time here).
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        let alpha = (self.settings.smoothing_speed * self.settings.animation_fps as f32 * dt)
            .clamp(0.0, 1.0);
        self.sim_cpu_smooth += (self.sim_cpu - self.sim_cpu_smooth) * alpha;
        self.sim_gpu_smooth += (self.sim_gpu - self.sim_gpu_smooth) * alpha;

        if self.last_daemon_check.elapsed() > Duration::from_millis(1500) {
            self.last_daemon_check = Instant::now();
            self.daemon_running = argb_core::win::daemon_running();
            self.setup.afterburner_ok = util::afterburner_running();
        }

        // First frame: quietly scan for zones so new users see their hardware
        // without having to know a Detect button exists.
        if !self.detect_autorun {
            self.detect_autorun = true;
            self.start_detection();
        }
        self.poll_detection();
        self.reload_if_changed_on_disk();

        self.top_bar(ctx);
        self.setup_banner(ctx);
        self.bottom_bar(ctx);
        preview::show(self, ctx);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::PANEL)
                    .inner_margin(Margin::same(16)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.tab {
                        Tab::Presets => tabs::presets_tab(self, ui),
                        Tab::Ports => tabs::zones_tab(self, ui),
                        Tab::Curves => tabs::curves_tab(self, ui),
                        Tab::Lab => builder::builder_tab(self, ui),
                        Tab::Advanced => tabs::advanced_tab(self, ui),
                    });
            });

        // Repaint pacing = CPU use. Full 30 FPS only while the user is
        // actually looking; ~10 FPS unfocused; ~1 FPS minimized/hidden so a
        // parked configurator costs (almost) nothing.
        let (focused, minimized) = ctx.input(|i| {
            let v = i.viewport();
            (v.focused.unwrap_or(true), v.minimized.unwrap_or(false))
        });
        let delay = if minimized {
            Duration::from_millis(1000)
        } else if focused {
            Duration::from_millis(33)
        } else {
            Duration::from_millis(100)
        };
        ctx.request_repaint_after(delay);
    }
}
