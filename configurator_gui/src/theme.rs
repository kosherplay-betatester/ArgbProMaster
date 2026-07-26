//! Dark "gamer utility" visual theme plus small reusable UI helpers.

use eframe::egui::{
    self, style::Selection, Color32, CornerRadius, FontFamily, FontId, Margin, RichText, Stroke,
    TextStyle, Visuals,
};

pub const BG: Color32 = Color32::from_rgb(13, 14, 22);
pub const PANEL: Color32 = Color32::from_rgb(19, 21, 33);
pub const CARD: Color32 = Color32::from_rgb(26, 29, 45);
pub const CARD_EDGE: Color32 = Color32::from_rgb(40, 44, 66);
pub const ACCENT: Color32 = Color32::from_rgb(33, 212, 253);
pub const ACCENT_2: Color32 = Color32::from_rgb(183, 33, 255);
pub const TEXT: Color32 = Color32::from_rgb(218, 222, 240);
pub const TEXT_DIM: Color32 = Color32::from_rgb(140, 146, 178);
pub const OK: Color32 = Color32::from_rgb(80, 220, 140);
pub const WARN: Color32 = Color32::from_rgb(255, 180, 60);
pub const DANGER: Color32 = Color32::from_rgb(255, 92, 92);

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.text_styles = [
        (TextStyle::Heading, FontId::new(23.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(14.5, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(13.5, FontFamily::Monospace)),
    ]
    .into();

    let mut v = Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = BG;
    v.faint_bg_color = Color32::from_rgb(24, 26, 40);
    v.selection = Selection {
        bg_fill: ACCENT.gamma_multiply(0.35),
        stroke: Stroke::new(1.0, ACCENT),
    };
    v.slider_trailing_fill = true;
    v.override_text_color = Some(TEXT);

    let cr = CornerRadius::same(7);
    v.widgets.noninteractive.bg_fill = CARD;
    v.widgets.noninteractive.weak_bg_fill = CARD;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, CARD_EDGE);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.noninteractive.corner_radius = cr;

    v.widgets.inactive.bg_fill = Color32::from_rgb(37, 41, 62);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(37, 41, 62);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, CARD_EDGE);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.inactive.corner_radius = cr;

    v.widgets.hovered.bg_fill = Color32::from_rgb(47, 53, 80);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(47, 53, 80);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT.gamma_multiply(0.7));
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    v.widgets.hovered.corner_radius = cr;

    v.widgets.active.bg_fill = ACCENT.gamma_multiply(0.45);
    v.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.45);
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    v.widgets.active.corner_radius = cr;

    v.widgets.open.bg_fill = Color32::from_rgb(42, 47, 72);
    v.widgets.open.weak_bg_fill = Color32::from_rgb(42, 47, 72);
    v.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT.gamma_multiply(0.5));
    v.widgets.open.corner_radius = cr;

    style.visuals = v;
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 7.0);
    style.spacing.slider_width = 210.0;
    style.spacing.interact_size.y = 30.0;
    ctx.set_style(style);
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, CARD_EDGE))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(14))
}

pub fn section_title(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.add_space(2.0);
    ui.label(RichText::new(title).heading().color(TEXT));
    ui.label(RichText::new(subtitle).small().color(TEXT_DIM));
    ui.add_space(8.0);
}

/// A small colored pill label ("CPU", "Disabled", …).
pub fn chip(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.18))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.8)))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).small().strong().color(color));
        });
}

/// Horizontal cold→warm→hot gradient bar sampled from the actual engine math.
pub fn gradient_bar(ui: &mut egui::Ui, colors: &argb_core::settings::ColorConfig, height: f32) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(5), BG);
    let inner = rect.shrink(2.0);
    const STEPS: usize = 48;
    let step_w = inner.width() / STEPS as f32;
    for i in 0..STEPS {
        let t = i as f32 / (STEPS - 1) as f32;
        let c = argb_core::engine::thermal_color(colors, t);
        let color = Color32::from_rgb(c[0] as u8, c[1] as u8, c[2] as u8);
        let x0 = inner.left() + i as f32 * step_w;
        let r = egui::Rect::from_min_max(
            egui::pos2(x0, inner.top()),
            egui::pos2(x0 + step_w + 0.5, inner.bottom()),
        );
        painter.rect_filled(r, CornerRadius::ZERO, color);
    }
}
