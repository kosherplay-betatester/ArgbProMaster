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
    _tray: Option<tray::Tray>,
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
            _tray: tray,
        }
    }

    pub fn toast(&mut self, message: String, color: Color32) {
        self.status = Some((message, color, Instant::now()));
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
            Err(e) => self.toast(
                format!("Couldn't scan for zones: {e}. Is OpenRGB running with its SDK server on?"),
                theme::WARN,
            ),
        }
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
                ui.horizontal(|ui| {
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
                        if ui.add(button).clicked() {
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

        // Keep the preview animating (~30 FPS) and tray/status responsive.
        ctx.request_repaint_after(Duration::from_millis(33));
    }
}
