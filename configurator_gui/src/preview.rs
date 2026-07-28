//! The virtual live preview: mock temperature sliders plus real-time LED
//! strips rendered with the exact same engine the daemon uses.

use crate::app::App;
use crate::theme;
use argb_core::engine;
use argb_core::settings::{ColorConfig, TargetSource};
use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Sense};

pub fn show(app: &mut App, ctx: &egui::Context) {
    egui::SidePanel::right("live_preview")
        .exact_width(348.0)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(theme::BG)
                .inner_margin(Margin::same(14)),
        )
        .show(ctx, |ui| {
            ui.add_space(2.0);
            ui.label(RichText::new("🔎 Live Preview").heading());
            ui.label(
                RichText::new("Exactly what your rig will do — before you save.")
                    .small()
                    .color(theme::TEXT_DIM),
            );
            ui.add_space(8.0);

            simulator_card(app, ui);
            ui.add_space(8.0);

            let time = ui.input(|i| i.time);
            let s = &app.settings;
            let cpu_n = engine::normalize_temp(app.sim_cpu_smooth, s.cpu_temp_min, s.cpu_temp_max);
            let gpu_n = engine::normalize_temp(app.sim_gpu_smooth, s.gpu_temp_min, s.gpu_temp_max);

            // Non-temperature sources: real readings when following the
            // sensors, the "other sensors" mock slider otherwise.
            let (cl, gl, ram, fps, fps_max) = if let Some(r) = &app.live_readings {
                (
                    r.cpu_load.unwrap_or(0.0),
                    r.gpu_load.unwrap_or(0.0),
                    r.ram_pct.unwrap_or(0.0),
                    r.fps.unwrap_or(0.0),
                    if r.fps_max > 0.0 { r.fps_max } else { 240.0 },
                )
            } else {
                let v = app.sim_other;
                (v, v, v, v * 2.4, 240.0)
            };
            let norm = [
                cpu_n,
                gpu_n,
                (cl / 100.0).clamp(0.0, 1.0),
                (gl / 100.0).clamp(0.0, 1.0),
                (ram / 100.0).clamp(0.0, 1.0),
                (fps / fps_max).clamp(0.0, 1.0),
            ];
            // Integrate the thermal clocks exactly like the daemon does, so
            // preview motion accelerates with heat without ever jumping.
            let dt = ui.input(|i| i.stable_dt).min(0.1);
            for i in 0..norm.len() {
                app.sim_phase[i] += dt * (0.5 + 1.5 * norm[i]);
            }
            let sources = engine::SourceValues {
                raw: [app.sim_cpu_smooth, app.sim_gpu_smooth, cl, gl, ram, fps],
                norm,
                phase: app.sim_phase,
            };

            let enabled: Vec<&argb_core::settings::ZoneConfig> = s
                .zones
                .iter()
                .filter(|z| z.enabled && z.effective_leds() > 0)
                .collect();

            if enabled.is_empty() {
                theme::card_frame().show(ui, |ui| {
                    ui.label(RichText::new("Nothing to preview yet").strong());
                    ui.label(
                        RichText::new(
                            "Switch a zone on in the Zones & Ports tab and it will animate here.",
                        )
                        .small()
                        .color(theme::TEXT_DIM),
                    );
                });
                return;
            }

            const MAX_ROWS: usize = 8;
            for zone in enabled.iter().take(MAX_ROWS) {
                let leds = zone.effective_leds();
                let idle = engine::idle_wants(s, zone, &sources);
                let frame =
                    engine::render_zone_config(s, zone, leds as usize, time, &sources, idle);
                let title = if zone.display_name.is_empty() {
                    zone.device_name.as_str()
                } else {
                    zone.display_name.as_str()
                };
                zone_card(
                    ui,
                    title,
                    leds,
                    zone.target_source,
                    true,
                    &effect_label(s, zone),
                    &[&frame],
                );
                ui.add_space(6.0);
            }
            if enabled.len() > MAX_ROWS {
                ui.label(
                    RichText::new(format!("…and {} more zone(s)", enabled.len() - MAX_ROWS))
                        .small()
                        .color(theme::TEXT_DIM),
                );
            }
        });
}

fn effect_label(s: &argb_core::settings::Settings, zone: &argb_core::settings::ZoneConfig) -> String {
    let custom = zone.custom_effect.as_deref().or(if zone.effect_override.is_none() {
        s.global_custom_effect.as_deref()
    } else {
        None
    });
    if let Some(name) = custom {
        if s.custom_effect(name).is_some() {
            return format!("★ {name}");
        }
    }
    zone.effect_override.unwrap_or(s.effects_mode).label().to_string()
}

fn simulator_card(app: &mut App, ui: &mut egui::Ui) {
    theme::card_frame().show(ui, |ui| {
        ui.label(RichText::new("🌡 Thermal Simulator").strong());
        ui.label(
            RichText::new("Drag to see how the lights shift as temperatures rise.")
                .small()
                .color(theme::TEXT_DIM),
        );
        ui.add_space(4.0);
        ui.checkbox(&mut app.live_temps, "📡 Follow real temperatures")
            .on_hover_text(
                "Drive this preview with your actual CPU/GPU sensors (via MSI Afterburner) so it \
                 animates exactly like your LEDs. Untick to experiment with the mock sliders.",
            );
        // Leave room for the value box and the CPU/GPU label so nothing clips
        // at the panel edge.
        ui.spacing_mut().slider_width = (ui.available_width() - 150.0).clamp(110.0, 190.0);
        let live = app.live_temps;
        ui.add_enabled(
            !live,
            egui::Slider::new(&mut app.sim_cpu, 20.0..=105.0)
                .text("CPU")
                .suffix(" °C"),
        )
        .on_hover_text("Mock CPU temperature. Only affects this preview — never the real daemon.");
        ui.add_enabled(
            !live,
            egui::Slider::new(&mut app.sim_gpu, 20.0..=105.0)
                .text("GPU")
                .suffix(" °C"),
        )
        .on_hover_text("Mock GPU temperature. Only affects this preview — never the real daemon.");
        ui.add_enabled(
            !live,
            egui::Slider::new(&mut app.sim_other, 0.0..=100.0)
                .text("Loads")
                .suffix(" %"),
        )
        .on_hover_text("Mock value for the non-temperature sources: CPU/GPU load, RAM use, and FPS (as % of max). Only affects this preview.");
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            temp_badge(ui, "CPU", app.sim_cpu_smooth,
                engine::normalize_temp(app.sim_cpu_smooth, app.settings.cpu_temp_min, app.settings.cpu_temp_max),
                &app.settings.colors);
            temp_badge(ui, "GPU", app.sim_gpu_smooth,
                engine::normalize_temp(app.sim_gpu_smooth, app.settings.gpu_temp_min, app.settings.gpu_temp_max),
                &app.settings.colors);
        });
    });
}

fn temp_badge(ui: &mut egui::Ui, label: &str, temp: f32, norm: f32, colors: &ColorConfig) {
    let c = engine::thermal_color(colors, norm);
    let color = Color32::from_rgb(c[0] as u8, c[1] as u8, c[2] as u8);
    theme::chip(ui, &format!("{label} {temp:.0}°C · {:.0}%", norm * 100.0), color);
}

fn zone_card(
    ui: &mut egui::Ui,
    title: &str,
    led_count: u32,
    target: TargetSource,
    enabled: bool,
    mode_label: &str,
    rows: &[&[[u8; 3]]],
) {
    theme::card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if enabled {
                    let color = match target {
                        TargetSource::Cpu => theme::ACCENT,
                        _ => theme::ACCENT_2,
                    };
                    theme::chip(ui, target.label(), color);
                } else {
                    theme::chip(ui, "Disabled", theme::TEXT_DIM);
                }
            });
        });
        ui.label(
            RichText::new(format!("{led_count} LEDs · {mode_label}"))
                .small()
                .color(theme::TEXT_DIM),
        );
        ui.add_space(2.0);
        if rows.is_empty() {
            led_strip(ui, &[], 22.0);
        }
        for row in rows {
            led_strip(ui, row, 22.0);
        }
    });
}

/// Paint one LED strip as individual glowing cells on a dark channel.
fn led_strip(ui: &mut egui::Ui, colors: &[[u8; 3]], height: f32) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(6), Color32::from_rgb(8, 9, 15));
    if colors.is_empty() {
        return;
    }
    let inner = rect.shrink(3.0);
    let n = colors.len();
    let cell_w = inner.width() / n as f32;
    let gap = (cell_w * 0.22).clamp(0.4, 2.0);
    for (i, c) in colors.iter().enumerate() {
        let x0 = inner.left() + i as f32 * cell_w;
        let cell = egui::Rect::from_min_max(
            egui::pos2(x0, inner.top()),
            egui::pos2((x0 + cell_w - gap).max(x0 + 0.5), inner.bottom()),
        );
        // soft glow underlay, then the LED itself
        painter.rect_filled(
            cell.expand(1.6),
            CornerRadius::same(3),
            Color32::from_rgba_unmultiplied(c[0], c[1], c[2], 42),
        );
        painter.rect_filled(cell, CornerRadius::same(2), Color32::from_rgb(c[0], c[1], c[2]));
    }
}
