//! The animation renderer. Pure math, no I/O — shared verbatim by the GUI
//! preview and the daemon so what you see is exactly what the LEDs get.

use crate::settings::{
    ColorConfig, CustomEffect, EffectTuning, EffectsMode, MotionKind, OverlayKind, Settings,
    ThermalBind, ZoneConfig,
};

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)]
}

fn to_f(c: [u8; 3]) -> [f32; 3] {
    [c[0] as f32, c[1] as f32, c[2] as f32]
}

/// Map a normalized temperature (0 = cold, 1 = hot) onto the three-stop
/// cold -> warm -> hot gradient.
pub fn thermal_color(colors: &ColorConfig, t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    let cold = to_f(colors.cold_color);
    let warm = to_f(colors.warm_color);
    let hot = to_f(colors.hot_color);
    if t <= 0.5 {
        lerp3(cold, warm, t * 2.0)
    } else {
        lerp3(warm, hot, (t - 0.5) * 2.0)
    }
}

/// Classic HSV -> RGB, h in degrees, s/v in 0..1, output channels 0..255.
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(360.0);
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [(r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0]
}

/// Convert a raw temperature into its 0..1 position on the response curve.
pub fn normalize_temp(temp: f32, min: f32, max: f32) -> f32 {
    if max <= min {
        return 0.0;
    }
    ((temp - min) / (max - min)).clamp(0.0, 1.0)
}

fn finish(rgb: [f32; 3], brightness: f32) -> [u8; 3] {
    let b = brightness.clamp(0.0, 1.0);
    [
        (rgb[0] * b).clamp(0.0, 255.0) as u8,
        (rgb[1] * b).clamp(0.0, 255.0) as u8,
        (rgb[2] * b).clamp(0.0, 255.0) as u8,
    ]
}

/// Deterministic pseudo-random 0..1 — hash noise instead of an RNG so the GUI
/// preview and the daemon render bit-identical frames from the same inputs.
fn hash01(x: f32) -> f32 {
    (x.sin() * 43758.547).fract().abs()
}

/// Per-LED value noise over time: piecewise-random levels crossfaded with
/// smoothstep, so flicker wanders instead of strobing.
fn flicker(i: usize, ft: f32) -> f32 {
    let cell = ft.floor();
    let seed = i as f32 * 12.9898;
    let a = hash01(seed + cell * 78.233);
    let b = hash01(seed + (cell + 1.0) * 78.233);
    let u = ft.fract();
    lerp(a, b, u * u * (3.0 - 2.0 * u))
}

/// Render one frame for a zone.
///
/// * `time` — seconds since the animation started.
/// * `temp` — smoothed, normalized temperature in 0..1.
/// * `brightness` — global brightness multiplier in 0..1.
/// * `tuning` — per-effect speed/intensity/variant; defaults keep stock looks.
pub fn render_zone(
    mode: EffectsMode,
    colors: &ColorConfig,
    led_count: usize,
    time: f64,
    temp: f32,
    brightness: f32,
    tuning: EffectTuning,
) -> Vec<[u8; 3]> {
    if led_count == 0 {
        return Vec::new();
    }
    let temp = temp.clamp(0.0, 1.0);
    let tuning = tuning.clamped(mode);
    let t = time as f32 * tuning.speed;
    let k = tuning.intensity;
    let n = led_count as f32;
    let tau = std::f32::consts::TAU;
    let mut out = Vec::with_capacity(led_count);

    match mode {
        EffectsMode::Solid => {
            let px = finish(thermal_color(colors, temp), brightness);
            out.resize(led_count, px);
        }
        EffectsMode::Breathing => {
            // Breathe a little faster as things heat up.
            let freq = 0.9 + temp * 1.6;
            // Intensity controls how deep each breath dips (stock 0.30 floor).
            let floor = (1.0 - 1.4 * k).clamp(0.05, 1.0);
            let breath = match tuning.variant {
                // Heartbeat: a strong lub and a softer dub per cycle.
                1 => {
                    let p = (t * freq * 0.5).fract();
                    let lub = (-((p - 0.10) / 0.045).powi(2)).exp();
                    let dub = 0.55 * (-((p - 0.32) / 0.055).powi(2)).exp();
                    floor + (1.0 - floor) * (lub + dub).min(1.0)
                }
                _ => floor + (1.0 - floor) * (0.5 + 0.5 * (t * freq).sin()),
            };
            let px = finish(thermal_color(colors, temp), brightness * breath);
            out.resize(led_count, px);
        }
        EffectsMode::ThermalWave => {
            // Temperature sets the overall flow rate: calm idle, lively hot.
            // Intensity 0.5 reproduces the stock ±0.18 color spread and shimmer.
            let spread = 0.06 + 0.24 * k;
            let shimmer = 0.36 * k;
            match tuning.variant {
                // Classic Wave: one gentle traveling sine.
                1 => {
                    let speed = 0.5 + temp * 2.0;
                    for i in 0..led_count {
                        let wave = ((i as f32 / n) * tau * 1.5 - t * speed).sin();
                        let ti = (temp + wave * spread).clamp(0.0, 1.0);
                        let level = (1.0 - shimmer) + shimmer * (0.5 + 0.5 * wave);
                        out.push(finish(thermal_color(colors, ti), brightness * level));
                    }
                }
                // Lava Sea: three layered traveling waves with incommensurate
                // frequencies, the middle one flowing against the others, so
                // crests emerge and dissolve instead of repeating.
                _ => {
                    let drift = 0.35 + temp * 1.4;
                    for i in 0..led_count {
                        let x = i as f32 / n;
                        let swell = (x * tau - t * drift).sin();
                        let cross = (x * 2.3 * tau + t * drift * 0.61).sin();
                        let ripple = (x * 4.7 * tau - t * drift * 1.37).sin();
                        let blend = 0.5 * swell + 0.33 * cross + 0.17 * ripple; // ~ -1..1
                        let ti = (temp + blend * spread).clamp(0.0, 1.0);
                        let mix = 0.5 + 0.5 * (0.8 * swell + 0.2 * ripple);
                        let level = (1.0 - shimmer) + shimmer * mix;
                        out.push(finish(thermal_color(colors, ti), brightness * level));
                    }
                }
            }
        }
        EffectsMode::GradientPulse => {
            // Spatial gradient that reaches further into the hot colors as the
            // temperature rises, with a soft global pulse layered on top.
            // Intensity sets pulse depth (stock 0.25 at k = 0.5).
            let depth = 0.5 * k;
            let pulse = (1.0 - depth) + depth * (t * 2.0).sin();
            for i in 0..led_count {
                let g = if led_count > 1 { i as f32 / (n - 1.0) } else { 0.0 };
                let ti = g * temp;
                out.push(finish(thermal_color(colors, ti), brightness * pulse));
            }
        }
        EffectsMode::SpectrumWave => {
            // Rainbow wave, deliberately independent of thermals.
            // Intensity sets how many rainbows span the strip (stock 1 at 0.5).
            let density = 2.0 * k;
            let sat = if tuning.variant == 1 { 0.55 } else { 1.0 };
            for i in 0..led_count {
                let hue = (i as f32 / n * density + t * 0.12).fract() * 360.0;
                out.push(finish(hsv_to_rgb(hue, sat, 1.0), brightness));
            }
        }
        EffectsMode::EmberFlicker => {
            // Each LED wanders like a glowing coal; hotter = livelier embers.
            let rate = 1.5 + temp * 4.0;
            let depth = 0.25 + 0.60 * k;
            let ft = t * rate;
            for i in 0..led_count {
                let glow = flicker(i, ft);
                let mut ti = (temp + (glow - 0.55) * (0.10 + 0.25 * k)).clamp(0.0, 1.0);
                let mut level = (1.0 - depth) + depth * glow;
                // Wildfire: rare white-hot sparks pop out of the bed of coals.
                if tuning.variant == 1 && hash01(i as f32 * 7.31 + ft.floor() * 3.97) > 0.96 {
                    level = 1.0;
                    ti = (ti + 0.30).min(1.0);
                }
                out.push(finish(thermal_color(colors, ti), brightness * level));
            }
        }
        EffectsMode::AuroraDrift => {
            // Slow counter-drifting curtains — the calmest effect of the set.
            let drift = (0.10 + temp * 0.25) * if tuning.variant == 1 { 1.6 } else { 1.0 };
            let spread = 0.10 + 0.25 * k;
            for i in 0..led_count {
                let x = i as f32 / n;
                let w1 = (x * tau * 0.8 - t * drift).sin();
                let w2 = (x * tau * 1.7 + t * drift * 0.63).sin();
                let mut blend = 0.6 * w1 + 0.4 * w2;
                if tuning.variant == 1 {
                    blend += 0.25 * (x * tau * 3.1 - t * drift * 1.9).sin();
                }
                let ti = (temp + blend * spread).clamp(0.0, 1.0);
                let level = 0.70 + 0.30 * (0.5 + 0.5 * w1);
                out.push(finish(thermal_color(colors, ti), brightness * level));
            }
        }
        EffectsMode::CometChase => {
            // A bright head sweeping the strip with an exponential tail.
            // Intensity sets tail length; temperature sets sweep rate.
            let cycles = 0.25 + temp * 0.55;
            let tail = 0.06 + 0.30 * k;
            let head = (t * cycles).fract();
            for i in 0..led_count {
                let x = i as f32 / n;
                let d = (head - x).rem_euclid(1.0);
                let mut glow = (-d / tail).exp();
                // Twin Comets: a mirror comet sweeps the opposite direction.
                if tuning.variant == 1 {
                    let head2 = 1.0 - head;
                    let d2 = (x - head2).rem_euclid(1.0);
                    glow = glow.max((-d2 / tail).exp());
                }
                let level = 0.06 + 0.94 * glow;
                out.push(finish(thermal_color(colors, temp), brightness * level));
            }
        }
        EffectsMode::ThermalFill => {
            // A thermometer along the strip: the lit portion is the current
            // temperature (0–100%), painted with the full cold→hot gradient
            // so the head always shows "how hot" at a glance.
            let fill = temp.max(0.02);
            let soft = 0.02 + 0.10 * k;
            let head_pulse = 0.85 + 0.15 * (t * 2.0).sin();
            for i in 0..led_count {
                let x = if led_count > 1 { i as f32 / (n - 1.0) } else { 0.0 };
                // Center Bloom fills outward from the middle of the strip.
                let pos = if tuning.variant == 1 { (x - 0.5).abs() * 2.0 } else { x };
                let lit = ((fill - pos) / soft + 0.5).clamp(0.0, 1.0);
                let lit = lit * lit * (3.0 - 2.0 * lit);
                let ti = (pos / fill).clamp(0.0, 1.0) * temp;
                let head = (1.0 - ((pos - fill).abs() / soft).min(1.0)) * head_pulse;
                let level = 0.04 + 0.96 * lit.max(head * 0.9);
                out.push(finish(thermal_color(colors, ti), brightness * level));
            }
        }
        EffectsMode::MeteorShower => {
            // Several meteors, each with its own offset and (in Chaotic) its
            // own pace, streaking with exponential trails.
            let meteors = 2 + (k * 4.0) as u32;
            let rate = 0.15 + temp * 0.35;
            let tail = 0.05 + 0.12 * k;
            for i in 0..led_count {
                let x = i as f32 / n;
                let mut glow: f32 = 0.0;
                for m in 0..meteors {
                    let seed = hash01(m as f32 * 91.17);
                    let pace = if tuning.variant == 1 { 0.6 + seed } else { 1.0 };
                    let head = (t * rate * pace + seed).fract();
                    let d = (head - x).rem_euclid(1.0);
                    glow = glow.max((-d / tail).exp());
                }
                let level = 0.05 + 0.95 * glow.min(1.0);
                out.push(finish(thermal_color(colors, temp), brightness * level));
            }
        }
        EffectsMode::LarsonScanner => {
            // A glowing eye bouncing end to end; faster and wider when hot.
            let rate = 0.35 + temp * 0.5;
            let width = 0.03 + 0.09 * k;
            let p = (t * rate).fract();
            let pos = 1.0 - (1.0 - 2.0 * p).abs();
            for i in 0..led_count {
                let x = if led_count > 1 { i as f32 / (n - 1.0) } else { 0.0 };
                let mut glow = (-((x - pos) / width).powi(2)).exp();
                // Dual Eye: a mirrored second scanner.
                if tuning.variant == 1 {
                    glow = glow.max((-((x - (1.0 - pos)) / width).powi(2)).exp());
                }
                let level = 0.05 + 0.95 * glow;
                out.push(finish(thermal_color(colors, temp), brightness * level));
            }
        }
        EffectsMode::Plasma => {
            // Two interfering waves swirl the gradient like liquid light.
            let spread = 0.15 + 0.35 * k;
            for i in 0..led_count {
                let x = i as f32 / n;
                let mut swirl = (x * 5.1 + t * 0.7).sin() * (x * 2.3 - t * 1.13).sin();
                if tuning.variant == 1 {
                    swirl += 0.35 * (x * 9.7 + t * 2.3).sin();
                }
                let ti = (temp + swirl * spread).clamp(0.0, 1.0);
                let level = 0.85 + 0.15 * (x * 3.7 + t * 0.91).sin().abs();
                out.push(finish(thermal_color(colors, ti), brightness * level));
            }
        }
        EffectsMode::StarfieldTwinkle => {
            // Sparse stars twinkling over a near-dark sky.
            let density = 0.12 + 0.28 * k;
            let twinkle_rate = 0.8 + temp * 1.2;
            for i in 0..led_count {
                let is_star = hash01(i as f32 * 17.77) < density;
                let mut level = 0.05;
                if is_star {
                    let tw = flicker(i, t * twinkle_rate + hash01(i as f32 * 3.3) * 7.0);
                    level += 0.95 * tw * tw;
                }
                // Shooting Stars: one comet occasionally crosses the field.
                if tuning.variant == 1 {
                    let head = (t * 0.22).fract();
                    let d = (head - i as f32 / n).rem_euclid(1.0);
                    level = level.max(0.05 + 0.95 * (-d / 0.05).exp());
                }
                out.push(finish(thermal_color(colors, temp), brightness * level.min(1.0)));
            }
        }
        EffectsMode::RainDrops => {
            // Drops land at pseudo-random spots and ripple outward, fading.
            let drops = if tuning.variant == 1 { 6 } else { 3 };
            let rate = if tuning.variant == 1 { 0.7 } else { 0.4 } + temp * 0.4;
            for i in 0..led_count {
                let x = i as f32 / n;
                let mut glow: f32 = 0.0;
                for d in 0..drops {
                    let seed = hash01(d as f32 * 7.31);
                    let cycle = (t * rate + seed).fract();
                    let generation = (t * rate + seed).floor();
                    let center = hash01(d as f32 * 31.7 + generation * 13.1);
                    let radius = cycle * (0.15 + 0.20 * k);
                    let ring = (-((x - center).abs() - radius).abs() * 60.0).exp();
                    glow = glow.max(ring * (1.0 - cycle));
                }
                let level = 0.07 + 0.93 * glow.min(1.0);
                out.push(finish(thermal_color(colors, temp), brightness * level));
            }
        }
    }
    out
}

/// Multi-stop palette lookup: linear blend between neighbouring stops.
pub fn palette_color(palette: &[(f32, [u8; 3])], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    match palette {
        [] => [255.0, 255.0, 255.0],
        [only] => to_f(only.1),
        _ => {
            if t <= palette[0].0 {
                return to_f(palette[0].1);
            }
            for pair in palette.windows(2) {
                let (p0, c0) = pair[0];
                let (p1, c1) = pair[1];
                if t <= p1 {
                    let span = (p1 - p0).max(1e-6);
                    return lerp3(to_f(c0), to_f(c1), (t - p0) / span);
                }
            }
            to_f(palette[palette.len() - 1].1)
        }
    }
}

/// Render one frame of an Effect Lab custom effect. Deterministic, like the
/// builtin effects, so the GUI preview matches the LEDs exactly.
pub fn render_custom(
    fx: &CustomEffect,
    led_count: usize,
    time: f64,
    temp: f32,
    brightness: f32,
) -> Vec<[u8; 3]> {
    if led_count == 0 {
        return Vec::new();
    }
    let temp = temp.clamp(0.0, 1.0);
    let speed_boost = if fx.thermal == ThermalBind::Speed { 0.5 + temp * 1.5 } else { 1.0 };
    let t = time as f32 * fx.speed.clamp(0.25, 3.0) * speed_boost;
    let n = led_count as f32;
    let scale = fx.scale.clamp(0.0, 1.0);
    let tau = std::f32::consts::TAU;
    let mut out = Vec::with_capacity(led_count);

    for i in 0..led_count {
        let mut x = if led_count > 1 { i as f32 / (n - 1.0) } else { 0.0 };
        if fx.reverse {
            x = 1.0 - x;
        }

        // Base motion: a palette position (0..1) and a brightness level.
        let (motion_pos, mut level) = match fx.motion {
            MotionKind::Flow => {
                let density = 0.5 + 2.5 * scale;
                ((x * density + t * 0.15).fract(), 1.0)
            }
            MotionKind::Fill => {
                let fill = if fx.thermal == ThermalBind::FillLevel {
                    temp.max(0.02)
                } else {
                    // Unbound fill slowly sweeps so it still looks alive.
                    0.5 + 0.5 * (t * 0.4).sin()
                };
                let soft = 0.02 + 0.10 * scale;
                let lit = ((fill - x) / soft + 0.5).clamp(0.0, 1.0);
                let lit = lit * lit * (3.0 - 2.0 * lit);
                ((x / fill).clamp(0.0, 1.0), 0.04 + 0.96 * lit)
            }
            MotionKind::Chase => {
                let head = (t * 0.35).fract();
                let tail = 0.05 + 0.30 * scale;
                let d = (head - x).rem_euclid(1.0);
                let glow = (-d / tail).exp();
                (d.min(1.0), 0.05 + 0.95 * glow)
            }
            MotionKind::Flicker => {
                let glow = flicker(i, t * 2.2);
                (glow, 0.35 + 0.65 * glow)
            }
            MotionKind::Breathe => {
                let breath = 0.5 + 0.5 * (t * 0.9).sin();
                (x, 0.15 + 0.85 * breath)
            }
            MotionKind::Still => (x, 1.0),
        };

        // Thermal binding: colors follow temperature, with the motion adding
        // life around the temperature point.
        let p = match fx.thermal {
            ThermalBind::PalettePosition => (temp + (motion_pos - 0.5) * 0.35).clamp(0.0, 1.0),
            _ => motion_pos,
        };

        // Overlay layer.
        match fx.overlay {
            OverlayKind::None => {}
            OverlayKind::Shimmer => {
                let s = fx.overlay_strength * 0.35;
                level *= (1.0 - s) + s * (0.5 + 0.5 * (x * tau * 3.0 - t * 1.7).sin());
            }
            OverlayKind::Pulse => {
                let s = fx.overlay_strength * 0.4;
                level *= (1.0 - s) + s * (0.5 + 0.5 * (t * 1.3).sin());
            }
            OverlayKind::Sparks => {
                if hash01(i as f32 * 7.31 + (t * 3.0).floor() * 3.97)
                    > 1.0 - 0.04 * fx.overlay_strength
                {
                    level = 1.0;
                }
            }
        }

        out.push(finish(palette_color(&fx.palette, p), brightness * level.clamp(0.0, 1.0)));
    }
    out
}

/// All source metrics in one bundle, indexed by `TargetSource::index()`:
/// `norm` = 0..1 for coloring, `raw` = natural units (°C, %, frames/s) for
/// the idle-range check.
#[derive(Clone, Copy, Debug, Default)]
pub struct SourceValues {
    pub norm: [f32; 6],
    pub raw: [f32; 6],
}

/// Render a configured zone: resolves idle mode, custom vs builtin effect,
/// per-zone colors and its chosen component source. Disabled zones render
/// black. This is THE render entry point shared by daemon and GUI preview.
pub fn render_zone_config(
    settings: &Settings,
    zone: &ZoneConfig,
    led_count: usize,
    time: f64,
    sources: &SourceValues,
) -> Vec<[u8; 3]> {
    if !zone.enabled {
        return vec![[0, 0, 0]; led_count];
    }
    let idx = zone.target_source.index();
    let temp = sources.norm[idx];

    // Idle mode: while this zone's source rests inside the chosen range (in
    // its natural units — °C or %), show the calmer idle effect instead.
    if settings.idle_enabled {
        let raw = sources.raw[idx];
        if raw >= settings.idle_temp_min && raw <= settings.idle_temp_max {
            if let Some(fx) = settings
                .idle_custom_effect
                .as_deref()
                .and_then(|name| settings.custom_effect(name))
            {
                return render_custom(fx, led_count, time, temp, settings.global_brightness);
            }
            let mode = settings.idle_effect;
            let colors = zone.colors_override.unwrap_or(settings.colors);
            let tuning = settings.tuning(mode);
            return render_zone(mode, &colors, led_count, time, temp, settings.global_brightness, tuning);
        }
    }

    // Custom effect resolution: zone-level name, else the global custom
    // effect (only when the zone has no builtin override), else builtins.
    let custom_name = zone.custom_effect.as_deref().or(if zone.effect_override.is_none() {
        settings.global_custom_effect.as_deref()
    } else {
        None
    });
    if let Some(fx) = custom_name.and_then(|name| settings.custom_effect(name)) {
        return render_custom(fx, led_count, time, temp, settings.global_brightness);
    }

    let mode = zone.effect_override.unwrap_or(settings.effects_mode);
    let colors = zone.colors_override.unwrap_or(settings.colors);
    let tuning = settings.tuning(mode);
    render_zone(mode, &colors, led_count, time, temp, settings.global_brightness, tuning)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ColorConfig;

    #[test]
    fn thermal_color_hits_the_three_stops() {
        let c = ColorConfig::default();
        assert_eq!(thermal_color(&c, 0.0), [0.0, 255.0, 200.0]);
        assert_eq!(thermal_color(&c, 0.5), [180.0, 0.0, 255.0]);
        assert_eq!(thermal_color(&c, 1.0), [255.0, 10.0, 10.0]);
    }

    #[test]
    fn render_zone_len_matches_and_zero_brightness_is_black() {
        let c = ColorConfig::default();
        for mode in EffectsMode::ALL {
            for variant in 0..mode.variant_labels().len().max(1) as u32 {
                let tuning = EffectTuning { variant, ..EffectTuning::default() };
                let frame = render_zone(mode, &c, 72, 1.25, 0.5, 0.0, tuning);
                assert_eq!(frame.len(), 72);
                assert!(frame.iter().all(|px| *px == [0, 0, 0]), "{mode:?} v{variant}");
            }
        }
        let empty = render_zone(EffectsMode::Solid, &c, 0, 0.0, 0.0, 1.0, EffectTuning::default());
        assert!(empty.is_empty());
    }

    #[test]
    fn render_zone_is_deterministic() {
        let c = ColorConfig::default();
        for mode in EffectsMode::ALL {
            let t = EffectTuning::default();
            let a = render_zone(mode, &c, 48, 3.7, 0.4, 0.8, t);
            let b = render_zone(mode, &c, 48, 3.7, 0.4, 0.8, t);
            assert_eq!(a, b, "{mode:?} must render identically for equal inputs");
        }
    }

    #[test]
    fn default_tuning_reproduces_stock_thermal_wave() {
        // Intensity 0.5 must map to the stock ±0.18 spread / 0.18 shimmer.
        let t = EffectTuning::default();
        assert!((0.06 + 0.24 * t.intensity - 0.18).abs() < 1e-6);
        assert!((0.36 * t.intensity - 0.18).abs() < 1e-6);
    }

    #[test]
    fn variants_and_speed_change_the_frame() {
        let c = ColorConfig::default();
        for mode in [
            EffectsMode::ThermalWave,
            EffectsMode::Breathing,
            EffectsMode::EmberFlicker,
            EffectsMode::AuroraDrift,
            EffectsMode::CometChase,
            EffectsMode::ThermalFill,
            EffectsMode::MeteorShower,
            EffectsMode::LarsonScanner,
            EffectsMode::Plasma,
            EffectsMode::StarfieldTwinkle,
            EffectsMode::RainDrops,
        ] {
            let base = render_zone(mode, &c, 60, 2.0, 0.5, 1.0, EffectTuning::default());
            let v1 = EffectTuning { variant: 1, ..EffectTuning::default() };
            assert_ne!(
                base,
                render_zone(mode, &c, 60, 2.0, 0.5, 1.0, v1),
                "{mode:?} variant 1 should look different"
            );
            let fast = EffectTuning { speed: 3.0, ..EffectTuning::default() };
            assert_ne!(
                base,
                render_zone(mode, &c, 60, 2.0, 0.5, 1.0, fast),
                "{mode:?} speed should shift the animation"
            );
        }
    }

    #[test]
    fn normalize_temp_clamps() {
        assert_eq!(normalize_temp(20.0, 40.0, 85.0), 0.0);
        assert_eq!(normalize_temp(100.0, 40.0, 85.0), 1.0);
        let mid = normalize_temp(62.5, 40.0, 85.0);
        assert!((mid - 0.5).abs() < 1e-4);
        assert_eq!(normalize_temp(50.0, 80.0, 80.0), 0.0);
    }

    #[test]
    fn palette_color_interpolates_stops() {
        let p = vec![(0.0, [0u8, 0, 0]), (0.5, [100, 100, 100]), (1.0, [200, 200, 200])];
        assert_eq!(palette_color(&p, 0.0), [0.0, 0.0, 0.0]);
        assert_eq!(palette_color(&p, 1.0), [200.0, 200.0, 200.0]);
        let mid = palette_color(&p, 0.25);
        assert!((mid[0] - 50.0).abs() < 1.0);
        // Degenerate palettes never panic.
        assert_eq!(palette_color(&[], 0.5), [255.0, 255.0, 255.0]);
        assert_eq!(palette_color(&[(0.3, [9, 9, 9])], 0.9), [9.0, 9.0, 9.0]);
    }

    #[test]
    fn render_custom_covers_every_motion_and_is_deterministic() {
        use crate::settings::{CustomEffect, MotionKind, OverlayKind, ThermalBind};
        for motion in MotionKind::ALL {
            for overlay in OverlayKind::ALL {
                for thermal in ThermalBind::ALL {
                    let fx = CustomEffect {
                        name: "t".into(),
                        motion,
                        overlay,
                        thermal,
                        ..CustomEffect::default()
                    };
                    let a = render_custom(&fx, 40, 2.5, 0.4, 0.8);
                    let b = render_custom(&fx, 40, 2.5, 0.4, 0.8);
                    assert_eq!(a.len(), 40);
                    assert_eq!(a, b, "{motion:?}/{overlay:?}/{thermal:?}");
                    // Zero brightness must always be black.
                    let dark = render_custom(&fx, 40, 2.5, 0.4, 0.0);
                    assert!(dark.iter().all(|px| *px == [0, 0, 0]));
                }
            }
        }
        assert!(render_custom(&CustomEffect::default(), 0, 0.0, 0.0, 1.0).is_empty());
    }

    #[test]
    fn zone_config_custom_effect_wins_and_falls_back() {
        use crate::settings::{CustomEffect, Settings, ZoneConfig};
        let mut s = Settings::default();
        s.custom_effects.push(CustomEffect {
            name: "Mine".into(),
            palette: vec![(0.0, [1, 2, 3]), (1.0, [1, 2, 3])],
            motion: crate::settings::MotionKind::Still,
            thermal: crate::settings::ThermalBind::None,
            ..CustomEffect::default()
        });
        let mut zone = ZoneConfig { enabled: true, custom_effect: Some("Mine".into()), ..ZoneConfig::default() };
        let sv = SourceValues { norm: [0.2; 6], raw: [45.0, 35.0, 20.0, 20.0, 20.0, 60.0] };
        let frame = render_zone_config(&s, &zone, 4, 0.0, &sv);
        // Full-brightness custom palette color scaled by global brightness 0.70.
        assert_eq!(frame[0], [0, 1, 2]);
        // Unknown names fall back to the builtin path instead of crashing.
        zone.custom_effect = Some("Ghost".into());
        let fallback = render_zone_config(&s, &zone, 4, 0.0, &sv);
        assert_eq!(fallback.len(), 4);
    }

    #[test]
    fn idle_effect_kicks_in_inside_the_range_only() {
        use crate::settings::{Settings, ZoneConfig};
        let mut s = Settings::default();
        s.effects_mode = EffectsMode::Solid;
        s.idle_enabled = true;
        s.idle_temp_min = 35.0;
        s.idle_temp_max = 50.0;
        s.idle_effect = EffectsMode::Breathing; // dips brightness — distinguishable
        let zone = ZoneConfig { enabled: true, ..ZoneConfig::default() };
        // CPU at 42°C (inside range) → idle Breathing, not Solid: pick a time
        // where the breath is clearly dimmer than solid.
        let inside = SourceValues { norm: [0.3; 6], raw: [42.0, 60.0, 0.0, 0.0, 0.0, 0.0] };
        let outside = SourceValues { norm: [0.3; 6], raw: [70.0, 60.0, 0.0, 0.0, 0.0, 0.0] };
        let idle = render_zone_config(&s, &zone, 4, 4.9, &inside);
        let solid = render_zone(EffectsMode::Solid, &s.colors, 4, 4.9, 0.3, s.global_brightness, EffectTuning::default());
        assert_ne!(idle, solid, "inside the idle range the idle effect must render");
        // CPU at 70°C (outside) → the normal effect again.
        let normal = render_zone_config(&s, &zone, 4, 4.9, &outside);
        assert_eq!(normal, solid, "outside the range the normal effect returns");
        // Disabled idle → normal even inside the range.
        s.idle_enabled = false;
        let off = render_zone_config(&s, &zone, 4, 4.9, &inside);
        assert_eq!(off, solid);
    }

    #[test]
    fn hsv_primaries() {
        assert_eq!(hsv_to_rgb(0.0, 1.0, 1.0), [255.0, 0.0, 0.0]);
        assert_eq!(hsv_to_rgb(120.0, 1.0, 1.0), [0.0, 255.0, 0.0]);
        assert_eq!(hsv_to_rgb(240.0, 1.0, 1.0), [0.0, 0.0, 255.0]);
    }
}
