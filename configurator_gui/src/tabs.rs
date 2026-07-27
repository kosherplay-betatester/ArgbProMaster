//! The four main tabs: Presets & Themes, Port & Target Assignment,
//! Thermal Response Curves, and Advanced Settings.

use crate::app::App;
use crate::theme;
use crate::util;
use argb_core::presets::{apply_builtin, BUILTIN_PRESETS};
use argb_core::settings::{
    ColorConfig, CustomPreset, EffectsMode, PresetData, Settings, TargetSource, ZoneConfig,
};
use argb_core::zones::{device_emoji, device_kind_label};
use eframe::egui::{self, Color32, CornerRadius, RichText};

// ---------------------------------------------------------------------------
// 🎨 Presets & Themes
// ---------------------------------------------------------------------------

pub fn presets_tab(app: &mut App, ui: &mut egui::Ui) {
    theme::section_title(
        ui,
        "🎨 Presets & Themes",
        "One-click visual styles. Applying a preset rewrites the settings below — fine-tune afterwards in the other tabs.",
    );

    let mut to_apply: Option<&'static str> = None;
    // Two cards per row, sized to whatever width is actually available so
    // nothing clips on smaller windows.
    let card_w = ((ui.available_width() - 24.0) / 2.0).clamp(240.0, 380.0);
    for pair in BUILTIN_PRESETS.chunks(2) {
        ui.horizontal(|ui| {
            for preset in pair {
                let active = app.settings.active_preset == preset.name;
                let mut frame = theme::card_frame();
                if active {
                    frame = frame.stroke(egui::Stroke::new(1.5, theme::ACCENT));
                }
                frame.show(ui, |ui| {
                    // The frame sits inside a horizontal row, so its ui
                    // inherits left-to-right flow — without this explicit
                    // vertical layout the card's rows all render on one line
                    // and blow past the window edge.
                    ui.vertical(|ui| {
                        ui.set_width(card_w);
                        ui.set_max_width(card_w);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(preset.emoji).size(22.0));
                            ui.label(RichText::new(preset.name).strong().size(16.0))
                                .on_hover_text(format!(
                                    "{} Applying loads it into the editor — your LEDs only change when you Save.",
                                    preset.tagline
                                ));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if active {
                                        theme::chip(ui, "ACTIVE", theme::ACCENT);
                                    }
                                },
                            );
                        });
                        ui.label(
                            RichText::new(preset.tagline)
                                .small()
                                .color(theme::TEXT_DIM),
                        );
                        ui.add_space(4.0);
                        swatch_bar(ui, &preset.swatch);
                        ui.add_space(6.0);
                        let btn = ui
                            .button(if active { "Re-apply" } else { "Apply Preset" })
                            .on_hover_text("Loads this preset into the editor. Nothing touches your LEDs until you hit Apply & Save.");
                        if btn.clicked() {
                            to_apply = Some(preset.name);
                        }
                    });
                });
            }
        });
        ui.add_space(4.0);
    }
    if let Some(name) = to_apply {
        apply_builtin(name, &mut app.settings);
        app.toast(format!("“{name}” loaded — check the preview, then Apply & Save."), theme::OK);
    }

    ui.add_space(12.0);
    custom_presets_section(app, ui);
}

fn swatch_bar(ui: &mut egui::Ui, swatch: &[[u8; 3]; 3]) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 10.0), egui::Sense::hover());
    let painter = ui.painter();
    const STEPS: usize = 36;
    let step_w = rect.width() / STEPS as f32;
    let sw = [
        [swatch[0][0] as f32, swatch[0][1] as f32, swatch[0][2] as f32],
        [swatch[1][0] as f32, swatch[1][1] as f32, swatch[1][2] as f32],
        [swatch[2][0] as f32, swatch[2][1] as f32, swatch[2][2] as f32],
    ];
    for i in 0..STEPS {
        let t = i as f32 / (STEPS - 1) as f32;
        let c = if t <= 0.5 {
            argb_core::engine::lerp3(sw[0], sw[1], t * 2.0)
        } else {
            argb_core::engine::lerp3(sw[1], sw[2], (t - 0.5) * 2.0)
        };
        let x0 = rect.left() + i as f32 * step_w;
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x0, rect.top()),
                egui::pos2(x0 + step_w + 0.5, rect.bottom()),
            ),
            CornerRadius::ZERO,
            Color32::from_rgb(c[0] as u8, c[1] as u8, c[2] as u8),
        );
    }
}

fn custom_presets_section(app: &mut App, ui: &mut egui::Ui) {
    ui.label(RichText::new("💾 Custom Presets").strong().size(16.0));
    ui.label(
        RichText::new("Freeze your current look with a name, and bring it back any time.")
            .small()
            .color(theme::TEXT_DIM),
    );
    ui.add_space(6.0);

    theme::card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            let name_w = (ui.available_width() * 0.4).clamp(150.0, 280.0);
            ui.add(
                egui::TextEdit::singleline(&mut app.new_preset_name)
                    .hint_text("Name your masterpiece…")
                    .desired_width(name_w),
            )
            .on_hover_text("The name for the new custom preset. Re-using a name overwrites that preset.");
            let save = ui
                .button("💾 Save Current Look")
                .on_hover_text("Snapshots every setting — colors, curves, ports, brightness — under this name.");
            if save.clicked() {
                let name = app.new_preset_name.trim().to_string();
                if name.is_empty() {
                    app.toast("Give your preset a name first.".to_string(), theme::WARN);
                } else {
                    let data = PresetData::capture(&app.settings);
                    app.settings.custom_presets.retain(|p| p.name != name);
                    app.settings.custom_presets.push(CustomPreset {
                        name: name.clone(),
                        data,
                    });
                    app.settings.active_preset = name.clone();
                    app.new_preset_name.clear();
                    app.toast(format!("“{name}” saved to your presets."), theme::OK);
                }
            }
        });

        if app.settings.custom_presets.is_empty() {
            ui.add_space(4.0);
            ui.label(
                RichText::new("No custom presets yet — tweak something and save your first!")
                    .small()
                    .color(theme::TEXT_DIM),
            );
        }

        enum Action {
            Load(usize),
            Delete(usize),
        }
        let mut action: Option<Action> = None;
        for (i, preset) in app.settings.custom_presets.iter().enumerate() {
            ui.separator();
            ui.horizontal(|ui| {
                let active = app.settings.active_preset == preset.name;
                ui.label(RichText::new(&preset.name).strong());
                theme::chip(ui, preset.data.effects_mode.label(), theme::ACCENT_2);
                if active {
                    theme::chip(ui, "ACTIVE", theme::ACCENT);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("🗑")
                        .on_hover_text("Delete this custom preset (cannot be undone).")
                        .clicked()
                    {
                        action = Some(Action::Delete(i));
                    }
                    if ui
                        .button("Load")
                        .on_hover_text("Restore every setting from this snapshot.")
                        .clicked()
                    {
                        action = Some(Action::Load(i));
                    }
                });
            });
        }
        match action {
            Some(Action::Load(i)) => {
                let preset = app.settings.custom_presets[i].clone();
                preset.data.apply(&mut app.settings);
                // A snapshot may predate the safety lock — clamp it now so the
                // preview matches what the daemon will render after saving.
                app.settings.normalize();
                app.settings.active_preset = preset.name.clone();
                app.toast(format!("“{}” loaded.", preset.name), theme::OK);
            }
            Some(Action::Delete(i)) => {
                let name = app.settings.custom_presets.remove(i).name;
                app.toast(format!("“{name}” deleted."), theme::WARN);
            }
            None => {}
        }
    });
}

// ---------------------------------------------------------------------------
// 🔌 Zones & Ports
// ---------------------------------------------------------------------------

pub fn zones_tab(app: &mut App, ui: &mut egui::Ui) {
    theme::section_title(
        ui,
        "🔌 Zones & Ports",
        "Every ARGB port and RGB device OpenRGB can see — including empty headers. \
         Flip a zone on, pick what it reacts to, and give it its own effect or colors.",
    );

    ui.horizontal(|ui| {
        let detecting = app.detect_rx.is_some();
        let label = if detecting { "⏳ Scanning…" } else { "🔄 Detect zones" };
        if ui
            .add_enabled(!detecting, egui::Button::new(label))
            .on_hover_text("Asks OpenRGB for every device, port and RAM stick it controls. Safe to click any time.")
            .clicked()
        {
            app.start_detection();
        }
        if !app.detected_devices.is_empty() {
            ui.label(
                RichText::new(format!("{} device(s) detected", app.detected_devices.len()))
                    .small()
                    .color(theme::TEXT_DIM),
            );
        }
    });
    ui.add_space(6.0);

    if app.settings.zones.is_empty() {
        theme::card_frame().show(ui, |ui| {
            ui.label(RichText::new("No zones yet").strong());
            ui.label(
                RichText::new(
                    "Click “Detect zones” above. If nothing appears, make sure OpenRGB is \
                     running with its SDK server enabled (the daemon needs that too).",
                )
                .small()
                .color(theme::TEXT_DIM),
            );
        });
        return;
    }

    // Group zone indices by device, preserving first-seen order.
    let mut device_order: Vec<String> = Vec::new();
    for z in &app.settings.zones {
        if !device_order.contains(&z.device_name) {
            device_order.push(z.device_name.clone());
        }
    }

    let global_colors = app.settings.colors;
    let custom_names: Vec<String> =
        app.settings.custom_effects.iter().map(|f| f.name.clone()).collect();
    for device in device_order {
        let indices: Vec<usize> = app
            .settings
            .zones
            .iter()
            .enumerate()
            .filter(|(_, z)| z.device_name == device)
            .map(|(i, _)| i)
            .collect();
        let device_type = app.settings.zones[indices[0]].device_type;
        let missing = !app.detected_devices.is_empty() && !app.detected_devices.contains(&device);

        theme::card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(device_emoji(device_type)).size(18.0));
                let title = if device.is_empty() { "Waiting for first detection…" } else { device.as_str() };
                ui.label(RichText::new(title).strong().size(16.0));
                theme::chip(ui, device_kind_label(device_type), theme::ACCENT_2);
                if missing {
                    theme::chip(ui, "Not detected right now", theme::WARN);
                }
            });
            for idx in indices {
                ui.separator();
                zone_row(ui, &mut app.settings.zones[idx], global_colors, &custom_names, idx);
            }
        });
        ui.add_space(8.0);
    }
}

fn zone_row(
    ui: &mut egui::Ui,
    zone: &mut ZoneConfig,
    global_colors: ColorConfig,
    custom_names: &[String],
    idx: usize,
) {
    let salt = format!("zone{idx}");
    ui.horizontal(|ui| {
        ui.checkbox(&mut zone.enabled, "")
            .on_hover_text("Switch this zone's lighting on or off. Off = we leave it completely alone.");
        let name_w = (ui.available_width() * 0.4).clamp(140.0, 260.0);
        ui.add(
            egui::TextEdit::singleline(&mut zone.display_name)
                .desired_width(name_w)
                .hint_text("Friendly name…"),
        )
        .on_hover_text("Your label for this zone — for you, not the hardware.");
        if !zone.zone_name.is_empty() && zone.zone_name != zone.display_name {
            ui.label(RichText::new(&zone.zone_name).small().color(theme::TEXT_DIM));
        }
    });
    if zone.last_seen_leds == 0 && zone.resizable && zone.led_count == 0 {
        ui.label(
            RichText::new("💤 Empty port — set a LED count below to bring it to life.")
                .small()
                .color(theme::WARN),
        );
    }
    egui::Grid::new(format!("grid_{salt}"))
        .num_columns(2)
        .spacing([16.0, 6.0])
        .show(ui, |ui| {
            ui.label("LEDs");
            ui.horizontal(|ui| {
                let max = if zone.max_leds > 0 { zone.max_leds } else { 300 };
                ui.add(egui::DragValue::new(&mut zone.led_count).range(0..=max))
                    .on_hover_text("How many LEDs are connected here (count them or check the strip's box). 0 = use what OpenRGB reports.");
                let auto = if zone.led_count == 0 {
                    format!("auto ({} detected)", zone.last_seen_leds)
                } else {
                    String::new()
                };
                ui.label(RichText::new(auto).small().color(theme::TEXT_DIM));
            });
            ui.end_row();

            ui.label("Reacts to");
            egui::ComboBox::from_id_salt(format!("target_{salt}"))
                .selected_text(zone.target_source.label())
                .show_ui(ui, |ui| {
                    for source in TargetSource::ALL {
                        ui.selectable_value(&mut zone.target_source, source, source.label())
                            .on_hover_text(source.describe());
                    }
                })
                .response
                .on_hover_text("Which system component drives this zone — temperatures, loads, RAM use or framerate.");
            ui.end_row();

            ui.label("Effect");
            zone_effect_combo(ui, zone, custom_names, &salt);
            ui.end_row();
        });
    colors_override_editor(ui, &mut zone.colors_override, global_colors, &salt);
}

/// Paint a multi-stop gradient as a horizontal bar (sorted copy).
fn stops_bar(ui: &mut egui::Ui, stops: &[(f32, [u8; 3])]) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 18.0), egui::Sense::hover());
    let painter = ui.painter();
    let mut sorted = stops.to_vec();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    const STEPS: usize = 48;
    let step_w = rect.width() / STEPS as f32;
    for i in 0..STEPS {
        let t = i as f32 / (STEPS - 1) as f32;
        let c = argb_core::engine::palette_color(&sorted, t);
        let x0 = rect.left() + i as f32 * step_w;
        painter.rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(x0, rect.top()),
                egui::pos2(x0 + step_w + 0.5, rect.bottom()),
            ),
            CornerRadius::ZERO,
            Color32::from_rgb(c[0] as u8, c[1] as u8, c[2] as u8),
        );
    }
}

/// Effect picker for one zone: follow global, any builtin, or a ★ custom.
fn zone_effect_combo(ui: &mut egui::Ui, zone: &mut ZoneConfig, custom_names: &[String], salt: &str) {
    let label = if let Some(name) = &zone.custom_effect {
        format!("★ {name}")
    } else {
        match zone.effect_override {
            None => "Follow global effect".to_string(),
            Some(m) => m.label().to_string(),
        }
    };
    egui::ComboBox::from_id_salt(format!("mode_{salt}"))
        .selected_text(label)
        .show_ui(ui, |ui| {
            let follow = zone.effect_override.is_none() && zone.custom_effect.is_none();
            if ui.selectable_label(follow, "Follow global effect").clicked() {
                zone.effect_override = None;
                zone.custom_effect = None;
            }
            for mode in EffectsMode::ALL {
                let selected = zone.custom_effect.is_none() && zone.effect_override == Some(mode);
                if ui
                    .selectable_label(selected, mode.label())
                    .on_hover_text(mode.describe())
                    .clicked()
                {
                    zone.effect_override = Some(mode);
                    zone.custom_effect = None;
                }
            }
            for name in custom_names {
                let selected = zone.custom_effect.as_deref() == Some(name.as_str());
                if ui.selectable_label(selected, format!("★ {name}")).clicked() {
                    zone.custom_effect = Some(name.clone());
                    zone.effect_override = None;
                }
            }
        })
        .response
        .on_hover_text("This zone's own effect — builtin, one of your ★ Effect Lab creations, or follow the global one.");
}

fn colors_override_editor(
    ui: &mut egui::Ui,
    over: &mut Option<ColorConfig>,
    global: ColorConfig,
    salt: &str,
) {
    let mut custom = over.is_some();
    if ui
        .checkbox(&mut custom, "Custom colors for this zone")
        .on_hover_text("Override the global cold/warm/hot colors just for this zone.")
        .changed()
    {
        *over = if custom { Some(global) } else { None };
    }
    if let Some(colors) = over {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Cold").small());
            ui.color_edit_button_srgb(&mut colors.cold_color)
                .on_hover_text("Zone color at the bottom of the temperature range.");
            ui.label(RichText::new("Warm").small());
            ui.color_edit_button_srgb(&mut colors.warm_color)
                .on_hover_text("Zone color at the middle of the temperature range.");
            ui.label(RichText::new("Hot").small());
            ui.color_edit_button_srgb(&mut colors.hot_color)
                .on_hover_text("Zone color at the top of the temperature range.");
            let _ = salt;
        });
    }
}

// ---------------------------------------------------------------------------
// 🌡 Thermal Response Curves
// ---------------------------------------------------------------------------

pub fn curves_tab(app: &mut App, ui: &mut egui::Ui) {
    theme::section_title(
        ui,
        "🌡 Thermal Response Curves",
        "Choose the temperature window each sensor sweeps across, and the colors the lights travel through.",
    );
    // Sliders scale with the window instead of overflowing it.
    ui.spacing_mut().slider_width = (ui.available_width() * 0.4).clamp(140.0, 240.0);

    let s = &mut app.settings;
    theme::card_frame().show(ui, |ui| {
        ui.label(RichText::new("CPU Response Range").strong());
        ui.label(
            RichText::new("Most CPUs idle around 40–50 °C and peak near 85–95 °C under load.")
                .small()
                .color(theme::TEXT_DIM),
        );
        ui.add(egui::Slider::new(&mut s.cpu_temp_min, 20.0..=90.0).text("Min").suffix(" °C"))
            .on_hover_text("Below this temperature the CPU zones sit fully on the Cold color.");
        ui.add(egui::Slider::new(&mut s.cpu_temp_max, 40.0..=110.0).text("Max").suffix(" °C"))
            .on_hover_text("At or above this temperature the CPU zones burn fully on the Hot color.");
        if s.cpu_temp_max <= s.cpu_temp_min + 1.0 {
            s.cpu_temp_max = s.cpu_temp_min + 1.0;
        }
    });
    ui.add_space(8.0);
    theme::card_frame().show(ui, |ui| {
        ui.label(RichText::new("GPU Response Range").strong());
        ui.label(
            RichText::new("GPUs usually idle cooler than CPUs — a lower window feels livelier.")
                .small()
                .color(theme::TEXT_DIM),
        );
        ui.add(egui::Slider::new(&mut s.gpu_temp_min, 20.0..=90.0).text("Min").suffix(" °C"))
            .on_hover_text("Below this temperature the GPU zones sit fully on the Cold color.");
        ui.add(egui::Slider::new(&mut s.gpu_temp_max, 40.0..=110.0).text("Max").suffix(" °C"))
            .on_hover_text("At or above this temperature the GPU zones burn fully on the Hot color.");
        if s.gpu_temp_max <= s.gpu_temp_min + 1.0 {
            s.gpu_temp_max = s.gpu_temp_min + 1.0;
        }
    });

    ui.add_space(8.0);
    theme::card_frame().show(ui, |ui| {
        ui.label(RichText::new("Color Journey").strong());
        ui.label(
            RichText::new("The gradient every thermal effect travels: cold → warm → hot.")
                .small()
                .color(theme::TEXT_DIM),
        );
        ui.add_space(4.0);

        let mut multi = !s.global_stops.is_empty();
        if ui
            .checkbox(&mut multi, "🌈 Custom multi-color journey")
            .on_hover_text("Up to 8 color stops instead of the classic 3 — place any color anywhere along the temperature range.")
            .changed()
        {
            s.global_stops = if multi { s.colors.stops() } else { Vec::new() };
        }

        if multi {
            let mut remove: Option<usize> = None;
            let count = s.global_stops.len();
            for (i, stop) in s.global_stops.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(&mut stop.1)
                        .on_hover_text("The color at this point of the journey.");
                    let mut pct = stop.0 * 100.0;
                    if ui
                        .add(egui::Slider::new(&mut pct, 0.0..=100.0).custom_formatter(|v, _| format!("{v:.0}%")))
                        .on_hover_text("Where along the cold→hot journey this color sits.")
                        .changed()
                    {
                        stop.0 = pct / 100.0;
                    }
                    if count > 2 && ui.button("✖").on_hover_text("Remove this color stop.").clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                s.global_stops.remove(i);
            }
            if s.global_stops.len() < 8 && ui.button("＋ Add color stop").clicked() {
                let last = s.global_stops.last().copied().unwrap_or((0.5, [255, 255, 255]));
                s.global_stops.push(((last.0 + 1.0) / 2.0, last.1));
            }
            ui.add_space(4.0);
            stops_bar(ui, &s.global_stops);
        } else {
            ui.horizontal(|ui| {
                ui.label("Cold");
                ui.color_edit_button_srgb(&mut s.colors.cold_color)
                    .on_hover_text("Color when the system is chilling at the bottom of the range.");
                ui.add_space(10.0);
                ui.label("Warm");
                ui.color_edit_button_srgb(&mut s.colors.warm_color)
                    .on_hover_text("Color at the midpoint of the temperature range.");
                ui.add_space(10.0);
                ui.label("Hot");
                ui.color_edit_button_srgb(&mut s.colors.hot_color)
                    .on_hover_text("Color when temperatures hit the top of the range.");
            });
            ui.add_space(6.0);
            theme::gradient_bar(ui, &s.colors, 18.0);
        }
        ui.add_space(8.0);

        ui.label(RichText::new("Global Effect").strong());
        let selected_label = if let Some(name) = &s.global_custom_effect {
            format!("★ {name}")
        } else {
            s.effects_mode.label().to_string()
        };
        let custom_names: Vec<String> = s.custom_effects.iter().map(|f| f.name.clone()).collect();
        egui::ComboBox::from_id_salt("global_mode")
            .selected_text(selected_label)
            .show_ui(ui, |ui| {
                for mode in EffectsMode::ALL {
                    let selected = s.global_custom_effect.is_none() && s.effects_mode == mode;
                    if ui
                        .selectable_label(selected, mode.label())
                        .on_hover_text(mode.describe())
                        .clicked()
                    {
                        s.effects_mode = mode;
                        s.global_custom_effect = None;
                    }
                }
                for name in &custom_names {
                    let selected = s.global_custom_effect.as_deref() == Some(name.as_str());
                    if ui.selectable_label(selected, format!("★ {name}")).clicked() {
                        s.global_custom_effect = Some(name.clone());
                    }
                }
            })
            .response
            .on_hover_text("The animation style used by every zone that hasn't set its own override. ★ = your Effect Lab creations.");
        let describe = if s.global_custom_effect.is_some() {
            "One of your own — tweak it any time in the Effect Lab."
        } else {
            s.effects_mode.describe()
        };
        ui.label(RichText::new(describe).small().color(theme::TEXT_DIM));
    });

    ui.add_space(8.0);
    idle_effect_card(app, ui);
    ui.add_space(8.0);
    effect_tuning_card(app, ui);
}

/// 😴 Idle Effect: a calmer look that kicks in while temps rest in a range.
fn idle_effect_card(app: &mut App, ui: &mut egui::Ui) {
    let s = &mut app.settings;
    theme::card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("😴 Idle Effect").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut s.idle_enabled, "Enabled")
                    .on_hover_text("When a zone's temperature sits inside the range below, show the idle effect instead of its normal one — e.g. calm Stealth-style breathing while the PC is chilling.");
            });
        });
        ui.label(
            RichText::new(
                "Once a zone's source settles into this range, the idle look kicks in; the \
                 moment it leaves, the normal effect returns. The range is in the source's own \
                 units — °C for temperatures, % for loads.",
            )
            .small()
            .color(theme::TEXT_DIM),
        );
        if !s.idle_enabled {
            return;
        }
        ui.add_space(4.0);
        ui.add(
            egui::Slider::new(&mut s.idle_temp_min, 0.0..=100.0)
                .text("From")
                .suffix(" °C"),
        )
        .on_hover_text("The bottom of the idle range.");
        ui.add(
            egui::Slider::new(&mut s.idle_temp_max, 5.0..=110.0)
                .text("To")
                .suffix(" °C"),
        )
        .on_hover_text("The top of the idle range. Tip: GPUs idle cooler than CPUs — start from 0 °C if you want GPU zones to idle too.");
        if s.idle_temp_max <= s.idle_temp_min + 1.0 {
            s.idle_temp_max = s.idle_temp_min + 1.0;
        }

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Idle look");
            let selected = if let Some(name) = &s.idle_custom_effect {
                format!("★ {name}")
            } else {
                s.idle_effect.label().to_string()
            };
            let custom_names: Vec<String> = s.custom_effects.iter().map(|f| f.name.clone()).collect();
            egui::ComboBox::from_id_salt("idle_effect")
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    for mode in EffectsMode::ALL {
                        let is = s.idle_custom_effect.is_none() && s.idle_effect == mode;
                        if ui
                            .selectable_label(is, mode.label())
                            .on_hover_text(mode.describe())
                            .clicked()
                        {
                            s.idle_effect = mode;
                            s.idle_custom_effect = None;
                        }
                    }
                    for name in &custom_names {
                        let is = s.idle_custom_effect.as_deref() == Some(name.as_str());
                        if ui.selectable_label(is, format!("★ {name}")).clicked() {
                            s.idle_custom_effect = Some(name.clone());
                        }
                    }
                })
                .response
                .on_hover_text("Any builtin effect or one of your ★ Effect Lab creations.");
        });

        ui.add_space(4.0);
        let mut own_colors = s.idle_colors.is_some();
        if ui
            .checkbox(&mut own_colors, "Custom colors for the idle look")
            .on_hover_text("Give idle its own cold/warm/hot colors — e.g. dim blues while resting, without touching your main gradient.")
            .changed()
        {
            s.idle_colors = if own_colors { Some(s.colors) } else { None };
        }
        if let Some(colors) = &mut s.idle_colors {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Cold").small());
                ui.color_edit_button_srgb(&mut colors.cold_color);
                ui.label(RichText::new("Warm").small());
                ui.color_edit_button_srgb(&mut colors.warm_color);
                ui.label(RichText::new("Hot").small());
                ui.color_edit_button_srgb(&mut colors.hot_color);
            });
        }

        let mut own_pace = s.idle_tuning.is_some();
        if ui
            .checkbox(&mut own_pace, "Custom speed && intensity for the idle look")
            .on_hover_text("Idle usually wants to be slower and softer than the normal effect — tune it here without touching the effect's global tuning.")
            .changed()
        {
            s.idle_tuning = if own_pace {
                Some(argb_core::settings::EffectTuning {
                    speed: 0.5,
                    intensity: 0.35,
                    ..s.tuning(s.idle_effect)
                })
            } else {
                None
            };
        }
        if let Some(tuning) = &mut s.idle_tuning {
            ui.add(
                egui::Slider::new(&mut tuning.speed, 0.25..=3.0)
                    .custom_formatter(|v, _| format!("{v:.2}×"))
                    .text("Idle speed"),
            )
            .on_hover_text("How fast the idle animation moves. 0.5× makes a lovely calm resting state.");
            ui.add(
                egui::Slider::new(&mut tuning.intensity, 0.0..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                    .text("Idle intensity"),
            )
            .on_hover_text("How pronounced the idle animation is — lower = softer, dreamier.");
        }
    });
}

fn effect_tuning_card(app: &mut App, ui: &mut egui::Ui) {
    let s = &mut app.settings;
    if s.global_custom_effect.is_some() {
        theme::card_frame().show(ui, |ui| {
            ui.label(RichText::new("🎛 Effect Tuning").strong());
            ui.label(
                RichText::new("Custom effects carry their own speed and style — edit them in the 🧪 Effect Lab tab.")
                    .small()
                    .color(theme::TEXT_DIM),
            );
        });
        return;
    }
    let mode = s.effects_mode;
    theme::card_frame().show(ui, |ui| {
        ui.label(RichText::new(format!("🎛 Effect Tuning · {}", mode.label())).strong());
        ui.label(
            RichText::new(
                "Each effect remembers its own tuning. It applies wherever the effect is used — \
                 including zones that picked it as an override. Select an effect above to tune it.",
            )
            .small()
            .color(theme::TEXT_DIM),
        );
        ui.add_space(4.0);

        let mut tuning = s.tuning(mode);
        let mut changed = false;

        changed |= ui
            .add(
                egui::Slider::new(&mut tuning.speed, 0.25..=3.0)
                    .custom_formatter(|v, _| format!("{v:.2}×"))
                    .text("Speed"),
            )
            .on_hover_text("Animation speed multiplier. 1.00× is the effect's natural pace.")
            .changed();

        changed |= ui
            .add(
                egui::Slider::new(&mut tuning.intensity, 0.0..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                    .text(mode.intensity_label()),
            )
            .on_hover_text(
                "How pronounced the motion is — color spread, shimmer depth, tail length, \
                 flicker strength… each effect interprets it its own way. 50% is the stock look.",
            )
            .changed();

        if let Some(detail_label) = mode.detail_label() {
            changed |= ui
                .add(
                    egui::Slider::new(&mut tuning.detail, 0.0..=1.0)
                        .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                        .text(detail_label),
                )
                .on_hover_text(
                    "Brightness resolution of the trail: 100% is perfectly smooth; \
                     lower values quantize it into visible retro steps (the classic 0-9 look).",
                )
                .changed();
        }

        let variants = mode.variant_labels();
        if !variants.is_empty() {
            let current = variants
                .get(tuning.variant as usize)
                .copied()
                .unwrap_or(variants[0]);
            ui.horizontal(|ui| {
                ui.label("Style");
                egui::ComboBox::from_id_salt("effect_variant")
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for (i, label) in variants.iter().enumerate() {
                            if ui
                                .selectable_label(tuning.variant == i as u32, *label)
                                .clicked()
                            {
                                tuning.variant = i as u32;
                                changed = true;
                            }
                        }
                    })
                    .response
                    .on_hover_text("Animation variant — a distinct style of this effect.");
            });
        }

        ui.add_space(4.0);
        if ui
            .button("↩ Reset this effect's tuning")
            .on_hover_text("Back to the stock speed, intensity and style for this effect only.")
            .clicked()
        {
            s.effect_tuning.remove(&mode);
            changed = false;
        } else if changed {
            s.effect_tuning.insert(mode, tuning.clamped(mode));
        }
    });
}

// ---------------------------------------------------------------------------
// ⚙ Advanced Settings
// ---------------------------------------------------------------------------

pub fn advanced_tab(app: &mut App, ui: &mut egui::Ui) {
    theme::section_title(
        ui,
        "⚙ Advanced Settings",
        "Animation performance, smoothing feel, and power safety.",
    );
    ui.spacing_mut().slider_width = (ui.available_width() * 0.4).clamp(140.0, 240.0);

    theme::card_frame().show(ui, |ui| {
        ui.label(RichText::new("Animation Engine").strong());
        ui.add(egui::Slider::new(&mut app.settings.animation_fps, 5..=60).text("FPS"))
            .on_hover_text("How many frames per second the daemon renders. 30 (the default) is silky and safe for USB LED controllers; if you raise it and the LEDs start to flicker erratically, the controller can't keep up — come back down.");
        ui.add(
            egui::Slider::new(&mut app.settings.smoothing_speed, 0.01..=0.5)
                .logarithmic(true)
                .text("Easing Speed"),
        )
        .on_hover_text("Easing Speed: Controls how slowly colors bleed when temperatures spike. Lower = dreamier, slower transitions; higher = snappy response.");
        ui.add(
            egui::Slider::new(&mut app.settings.transition_secs, 0.2..=5.0)
                .custom_formatter(|v, _| format!("{v:.1} s"))
                .text("Transition time"),
        )
        .on_hover_text("How long a look-change (idle kicking in, switching presets or effects) melts into the new one. Longer = slower, more cinematic crossfades.");
    });
    ui.add_space(8.0);

    theme::card_frame().show(ui, |ui| {
        ui.label(RichText::new("Brightness & Safety").strong());
        let lock_resp = ui
            .checkbox(&mut app.settings.safety_power_lock, "🔒 Safety Power Lock (cap at 70%)")
            .on_hover_text("Soft-caps brightness at 70% to keep ARGB header power draw comfortable. Untick only if you know your PSU headroom.");
        if lock_resp.changed() && app.settings.safety_power_lock {
            app.settings.global_brightness = app.settings.global_brightness.min(0.70);
        }
        let cap = if app.settings.safety_power_lock { 0.70 } else { 1.0 };
        ui.add(
            egui::Slider::new(&mut app.settings.global_brightness, 0.0..=cap)
                .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                .text("LED Brightness"),
        )
        .on_hover_text("Master brightness for every zone. The Safety Power Lock soft-caps this at 70%.");
        if !app.settings.safety_power_lock && app.settings.global_brightness > 0.70 {
            ui.label(
                RichText::new("⚠ Above 70% — watch your header power budget.")
                    .small()
                    .color(theme::WARN),
            );
        }
    });
    ui.add_space(8.0);

    theme::card_frame().show(ui, |ui| {
        ui.label(RichText::new("Daemon & Maintenance").strong());
        ui.horizontal(|ui| {
            if ui
                .button("▶ Start Daemon")
                .on_hover_text("Launches thermal_daemon.exe next to this app. Safe to click twice — it refuses to run doubled.")
                .clicked()
            {
                match util::spawn_daemon() {
                    Ok(()) => app.toast("Daemon launched.".to_string(), theme::OK),
                    Err(e) => app.toast(format!("Couldn't start daemon: {e}"), theme::DANGER),
                }
            }
            if ui
                .button("⏹ Stop Daemon")
                .on_hover_text("Terminates the background daemon. Your LEDs keep their last frame.")
                .clicked()
            {
                util::stop_daemon();
                app.toast("Stop signal sent to the daemon.".to_string(), theme::WARN);
            }
            if ui
                .button("📁 Open Settings Folder")
                .on_hover_text("Opens the folder containing settings.json in Explorer.")
                .clicked()
            {
                util::open_settings_folder();
            }
            if ui
                .button("🌐 Help & Project Page")
                .on_hover_text("Opens the ArgbProMaster page on GitHub — full guide, troubleshooting and updates.")
                .clicked()
            {
                util::open_project_page();
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            let fixit = egui::Button::new(
                RichText::new("🔧 Fix My RGB").strong().color(Color32::BLACK),
            )
            .fill(theme::ACCENT)
            .corner_radius(CornerRadius::same(8));
            if ui
                .add(fixit)
                .on_hover_text(
                    "One click back to a known-good state: stock settings, every detected zone \
                     switched on, Thermal Alert applied, daemon running. Then shape it at will.",
                )
                .clicked()
            {
                app.fix_my_rgb();
            }
            if ui
                .add(egui::Button::new(RichText::new("♻ Reset All to Defaults").color(theme::DANGER)))
                .on_hover_text("Restores every APP setting to factory defaults. Your zone setup and custom presets are kept.")
                .clicked()
            {
                let kept = app.settings.custom_presets.clone();
                let kept_zones = app.settings.zones.clone();
                app.settings = Settings::default();
                app.settings.custom_presets = kept;
                app.settings.zones = kept_zones;
                app.toast("Settings reset to defaults (zones and custom presets kept).".to_string(), theme::WARN);
            }
            let restoring = app.restore_running();
            if ui
                .add_enabled(!restoring, egui::Button::new("💡 Restore original lighting"))
                .on_hover_text(
                    "Stops the daemon and switches every device back to its own built-in effect — \
                     the look your LEDs had when first connected to the motherboard (usually the \
                     firmware rainbow). ArgbProMaster stays out of the way until you start it again.",
                )
                .clicked()
            {
                app.restore_hardware_lighting();
                app.toast("Handing your lighting back to the hardware…".to_string(), theme::OK);
            }
        });
    });
}
