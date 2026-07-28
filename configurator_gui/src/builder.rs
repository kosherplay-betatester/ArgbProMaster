//! 🧪 Effect Lab — build custom effects from simple blocks (colors, motion,
//! sparkle, thermal binding) with a live preview. Saved effects join every
//! effect menu in the app, marked with ★.

use crate::app::App;
use crate::theme;
use argb_core::engine;
use argb_core::settings::{CustomEffect, MotionKind, OverlayKind, ThermalBind};
use eframe::egui::{self, Color32, CornerRadius, RichText, Sense};

pub fn builder_tab(app: &mut App, ui: &mut egui::Ui) {
    theme::section_title(
        ui,
        "🧪 Effect Lab",
        "Invent your own effect from building blocks and watch it live below. \
         Saved effects appear in every effect menu with a ★ — assign them to any zone.",
    );

    library_card(app, ui);
    ui.add_space(8.0);
    editor_card(app, ui);
}

fn library_card(app: &mut App, ui: &mut egui::Ui) {
    if app.settings.custom_effects.is_empty() {
        return;
    }
    theme::card_frame().show(ui, |ui| {
        ui.label(RichText::new("★ Your effects").strong());
        enum Action {
            Load(usize),
            Global(usize),
            Delete(usize),
        }
        let mut action: Option<Action> = None;
        for (i, fx) in app.settings.custom_effects.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&fx.name).strong());
                theme::chip(ui, fx.motion.label(), theme::ACCENT_2);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🗑").on_hover_text("Delete this effect (zones using it fall back to the global effect).").clicked() {
                        action = Some(Action::Delete(i));
                    }
                    if ui.button("✏ Edit").on_hover_text("Load this effect into the editor below.").clicked() {
                        action = Some(Action::Load(i));
                    }
                    if ui.button("▶ Use everywhere").on_hover_text("Make this the global effect right now.").clicked() {
                        action = Some(Action::Global(i));
                    }
                });
            });
        }
        match action {
            Some(Action::Load(i)) => {
                app.effect_draft = app.settings.custom_effects[i].clone();
                app.toast("Loaded into the editor — scroll down to tweak it.".to_string(), theme::OK);
            }
            Some(Action::Global(i)) => {
                let name = app.settings.custom_effects[i].name.clone();
                app.settings.global_custom_effect = Some(name.clone());
                app.toast(format!("“{name}” is now the global effect — hit Save to make it live."), theme::OK);
            }
            Some(Action::Delete(i)) => {
                let fx = app.settings.custom_effects.remove(i);
                if app.settings.global_custom_effect.as_deref() == Some(fx.name.as_str()) {
                    app.settings.global_custom_effect = None;
                }
                for zone in app.settings.zones.iter_mut() {
                    if zone.custom_effect.as_deref() == Some(fx.name.as_str()) {
                        zone.custom_effect = None;
                    }
                }
                app.toast(format!("“{}” deleted.", fx.name), theme::WARN);
            }
            None => {}
        }
    });
}

fn editor_card(app: &mut App, ui: &mut egui::Ui) {
    theme::card_frame().show(ui, |ui| {
        ui.label(RichText::new("Effect editor").strong());
        ui.spacing_mut().slider_width = (ui.available_width() * 0.35).clamp(120.0, 220.0);
        ui.add_space(4.0);

        let fx = &mut app.effect_draft;
        ui.horizontal(|ui| {
            ui.label("Name");
            ui.add(
                egui::TextEdit::singleline(&mut fx.name)
                    .hint_text("e.g. Dragon Fire…")
                    .desired_width(220.0),
            )
            .on_hover_text("Saving with an existing name updates that effect.");
        });
        ui.add_space(6.0);

        // ----- Colors -------------------------------------------------------
        ui.label(RichText::new("🎨 Colors").strong());
        ui.label(
            RichText::new("The color journey, left to right. Click a swatch to change it, drag its position, add up to 8 stops.")
                .small()
                .color(theme::TEXT_DIM),
        );
        let mut remove: Option<usize> = None;
        let stops = fx.palette.len();
        for (i, stop) in fx.palette.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.color_edit_button_srgb(&mut stop.1);
                let mut pct = stop.0 * 100.0;
                if ui
                    .add(egui::Slider::new(&mut pct, 0.0..=100.0).custom_formatter(|v, _| format!("{v:.0}%")))
                    .on_hover_text("Where along the journey this color sits.")
                    .changed()
                {
                    stop.0 = pct / 100.0;
                }
                if stops > 2 && ui.button("✖").on_hover_text("Remove this color stop.").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            fx.palette.remove(i);
        }
        if fx.palette.len() < 8 && ui.button("＋ Add color stop").clicked() {
            let last = fx.palette.last().map(|s| *s).unwrap_or((0.5, [255, 255, 255]));
            fx.palette.push(((last.0 + 1.0) / 2.0, last.1));
        }
        palette_bar(ui, &fx.palette);
        ui.add_space(8.0);

        // ----- Motion & feel ------------------------------------------------
        ui.label(RichText::new("🌀 Motion").strong());
        egui::Grid::new("lab_grid").num_columns(2).spacing([16.0, 8.0]).show(ui, |ui| {
            ui.label("Movement");
            egui::ComboBox::from_id_salt("lab_motion")
                .selected_text(fx.motion.label())
                .show_ui(ui, |ui| {
                    for m in MotionKind::ALL {
                        ui.selectable_value(&mut fx.motion, m, m.label());
                    }
                })
                .response
                .on_hover_text("How the colors move along the strip.");
            ui.end_row();

            ui.label("Speed");
            ui.add(
                egui::Slider::new(&mut fx.speed, 0.25..=3.0)
                    .custom_formatter(|v, _| format!("{v:.2}×")),
            )
            .on_hover_text("Overall pace of the animation.");
            ui.end_row();

            ui.label("Pattern size");
            ui.add(
                egui::Slider::new(&mut fx.scale, 0.0..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
            )
            .on_hover_text("Wave density, tail length or edge softness — depends on the movement.");
            ui.end_row();

            ui.label("Direction");
            ui.checkbox(&mut fx.reverse, "Reverse")
                .on_hover_text("Flip the effect to run the other way along the strip.");
            ui.end_row();

            ui.label("Sparkle layer");
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("lab_overlay")
                    .selected_text(fx.overlay.label())
                    .show_ui(ui, |ui| {
                        for o in OverlayKind::ALL {
                            ui.selectable_value(&mut fx.overlay, o, o.label());
                        }
                    })
                    .response
                    .on_hover_text("An extra animated layer on top of the base motion.");
                if fx.overlay != OverlayKind::None {
                    ui.add(
                        egui::Slider::new(&mut fx.overlay_strength, 0.0..=1.0)
                            .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                    );
                }
            });
            ui.end_row();

            ui.label("Temperature");
            egui::ComboBox::from_id_salt("lab_thermal")
                .selected_text(fx.thermal.label())
                .show_ui(ui, |ui| {
                    for t in ThermalBind::ALL {
                        ui.selectable_value(&mut fx.thermal, t, t.label());
                    }
                })
                .response
                .on_hover_text("How this effect reacts to the zone's CPU/GPU temperature.");
            ui.end_row();
        });
        ui.add_space(8.0);

        // ----- Live preview -------------------------------------------------
        ui.label(RichText::new("🔎 Live preview").strong());
        ui.label(
            RichText::new("Uses the thermal simulator sliders on the right — drag them to test hot and cold.")
                .small()
                .color(theme::TEXT_DIM),
        );
        let time = ui.input(|i| i.time);
        let s = &app.settings;
        let temp = engine::normalize_temp(app.sim_cpu_smooth, s.cpu_temp_min, s.cpu_temp_max);
        let mut draft = app.effect_draft.clone();
        draft.sanitize();
        let frame = engine::render_custom(&draft, 60, time, time as f32, temp, s.global_brightness);
        strip(ui, &frame);
        ui.add_space(8.0);

        // ----- Save ---------------------------------------------------------
        ui.horizontal(|ui| {
            let can_save = !app.effect_draft.name.trim().is_empty();
            if ui
                .add_enabled(can_save, egui::Button::new("💾 Save effect"))
                .on_hover_text(if can_save {
                    "Adds it to your library (★ in every effect menu)."
                } else {
                    "Give your effect a name first."
                })
                .clicked()
            {
                let mut fx = app.effect_draft.clone();
                fx.name = fx.name.trim().to_string();
                fx.sanitize();
                let name = fx.name.clone();
                app.settings.custom_effects.retain(|f| f.name != name);
                app.settings.custom_effects.push(fx);
                app.toast(
                    format!("“{name}” saved — assign it in Zones & Ports or “Use everywhere”, then hit Save."),
                    theme::OK,
                );
            }
            if ui
                .button("↩ Start fresh")
                .on_hover_text("Clears the editor back to a simple starting effect.")
                .clicked()
            {
                app.effect_draft = CustomEffect::default();
            }
        });
    });
}

/// Paint the palette as a horizontal gradient bar.
fn palette_bar(ui: &mut egui::Ui, palette: &[(f32, [u8; 3])]) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 12.0), Sense::hover());
    let painter = ui.painter();
    let mut sorted = palette.to_vec();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    const STEPS: usize = 48;
    let step_w = rect.width() / STEPS as f32;
    for i in 0..STEPS {
        let t = i as f32 / (STEPS - 1) as f32;
        let c = engine::palette_color(&sorted, t);
        let x0 = rect.left() + i as f32 * step_w;
        painter.rect_filled(
            egui::Rect::from_min_max(egui::pos2(x0, rect.top()), egui::pos2(x0 + step_w + 0.5, rect.bottom())),
            CornerRadius::ZERO,
            Color32::from_rgb(c[0] as u8, c[1] as u8, c[2] as u8),
        );
    }
}

/// A single LED strip preview row (same look as the side panel).
fn strip(ui: &mut egui::Ui, colors: &[[u8; 3]]) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 22.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(6), Color32::from_rgb(8, 9, 15));
    if colors.is_empty() {
        return;
    }
    let inner = rect.shrink(3.0);
    let cell_w = inner.width() / colors.len() as f32;
    let gap = (cell_w * 0.22).clamp(0.4, 2.0);
    for (i, c) in colors.iter().enumerate() {
        let x0 = inner.left() + i as f32 * cell_w;
        let cell = egui::Rect::from_min_max(
            egui::pos2(x0, inner.top()),
            egui::pos2((x0 + cell_w - gap).max(x0 + 0.5), inner.bottom()),
        );
        painter.rect_filled(
            cell.expand(1.6),
            CornerRadius::same(3),
            Color32::from_rgba_unmultiplied(c[0], c[1], c[2], 42),
        );
        painter.rect_filled(cell, CornerRadius::same(2), Color32::from_rgb(c[0], c[1], c[2]));
    }
}
