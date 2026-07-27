//! Built-in one-click lighting presets. Each preset rewrites the relevant
//! parts of [`Settings`] and stamps its own name into `active_preset`.

use crate::settings::{ColorConfig, EffectTuning, EffectsMode, Settings};

pub struct BuiltinPreset {
    pub name: &'static str,
    pub emoji: &'static str,
    pub tagline: &'static str,
    /// Three swatch colors shown on the preset card (cold / warm / hot feel).
    pub swatch: [[u8; 3]; 3],
}

pub const BUILTIN_PRESETS: [BuiltinPreset; 9] = [
    BuiltinPreset {
        name: "Thermal Alert",
        emoji: "🔥",
        tagline: "Cold = Cyan, Warm = Purple, Hot = Crimson Red.",
        swatch: [[0, 255, 200], [180, 0, 255], [255, 10, 10]],
    },
    BuiltinPreset {
        name: "Cyberpunk 2077",
        emoji: "🏙",
        tagline: "Neon Yellow & Magenta dual-tone wave.",
        swatch: [[250, 255, 0], [255, 0, 200], [255, 40, 120]],
    },
    BuiltinPreset {
        name: "Vaporwave Sunset",
        emoji: "🌴",
        tagline: "Pastel Pink, Purple and Teal gradient.",
        swatch: [[255, 150, 200], [170, 120, 255], [95, 235, 225]],
    },
    BuiltinPreset {
        name: "Ice & Fire",
        emoji: "❄",
        tagline: "Glacier blue when cool — bursts into fiery orange as heat builds.",
        swatch: [[110, 195, 255], [255, 140, 0], [255, 30, 0]],
    },
    BuiltinPreset {
        name: "Stealth Dark",
        emoji: "🌙",
        tagline: "Dim ambient accent lighting at 15% brightness.",
        swatch: [[45, 25, 80], [80, 45, 130], [120, 70, 180]],
    },
    BuiltinPreset {
        name: "Smooth Spectrum Wave",
        emoji: "🌈",
        tagline: "Classic rainbow wave, independent of thermals.",
        swatch: [[255, 0, 0], [0, 255, 60], [0, 120, 255]],
    },
    BuiltinPreset {
        name: "Molten Ember",
        emoji: "🌋",
        tagline: "A bed of glowing coals with white-hot sparks as heat builds.",
        swatch: [[120, 25, 0], [255, 110, 0], [255, 220, 120]],
    },
    BuiltinPreset {
        name: "Aurora Borealis",
        emoji: "🌌",
        tagline: "Slow northern-lights curtains drifting green to violet.",
        swatch: [[0, 255, 120], [0, 180, 200], [170, 60, 255]],
    },
    BuiltinPreset {
        name: "Comet Storm",
        emoji: "☄",
        tagline: "Twin comets race the strips, faster as temperatures climb.",
        swatch: [[80, 160, 255], [200, 120, 255], [255, 80, 40]],
    },
];

/// Every preset must fully determine the look, so the baseline also restores
/// the default brightness — otherwise Stealth Dark's 15% silently persists
/// through every later preset and the lights appear dead while a bright
/// preset claims to be active. Presets that dim (Stealth Dark) override
/// brightness again after this reset.
fn reset_baseline(s: &mut Settings) {
    s.global_brightness = crate::settings::DEFAULT_BRIGHTNESS;
    for zone in s.zones.iter_mut() {
        zone.effect_override = None;
        zone.colors_override = None;
    }
}

/// Apply a built-in preset by name. Returns false if the name is unknown.
pub fn apply_builtin(name: &str, s: &mut Settings) -> bool {
    match name {
        "Thermal Alert" => {
            reset_baseline(s);
            s.effects_mode = EffectsMode::ThermalWave;
            s.colors = ColorConfig {
                cold_color: [0, 255, 200],
                warm_color: [180, 0, 255],
                hot_color: [255, 10, 10],
            };
            // Classic Wave variant with a pronounced crest: one bright wave
            // chasing along the gradient instead of the lava-sea shimmer.
            s.effect_tuning.insert(
                EffectsMode::ThermalWave,
                EffectTuning { speed: 1.2, intensity: 0.7, variant: 1 },
            );
        }
        "Cyberpunk 2077" => {
            reset_baseline(s);
            s.effects_mode = EffectsMode::ThermalWave;
            s.colors = ColorConfig {
                cold_color: [250, 255, 0],
                warm_color: [255, 0, 200],
                hot_color: [255, 40, 120],
            };
        }
        "Vaporwave Sunset" => {
            reset_baseline(s);
            s.effects_mode = EffectsMode::GradientPulse;
            s.colors = ColorConfig {
                cold_color: [255, 150, 200],
                warm_color: [170, 120, 255],
                hot_color: [95, 235, 225],
            };
        }
        "Ice & Fire" => {
            reset_baseline(s);
            // Glacier blue when cool, blazing orange-red when hot — the whole
            // rig tells the temperature story at a glance.
            s.effects_mode = EffectsMode::ThermalWave;
            s.colors = ColorConfig {
                cold_color: [110, 195, 255],
                warm_color: [255, 140, 0],
                hot_color: [255, 10, 0],
            };
        }
        "Stealth Dark" => {
            reset_baseline(s);
            s.effects_mode = EffectsMode::Breathing;
            s.global_brightness = 0.15;
            s.colors = ColorConfig {
                cold_color: [45, 25, 80],
                warm_color: [80, 45, 130],
                hot_color: [120, 70, 180],
            };
        }
        "Smooth Spectrum Wave" => {
            reset_baseline(s);
            s.effects_mode = EffectsMode::SpectrumWave;
        }
        "Molten Ember" => {
            reset_baseline(s);
            s.effects_mode = EffectsMode::EmberFlicker;
            s.colors = ColorConfig {
                cold_color: [120, 25, 0],
                warm_color: [255, 110, 0],
                hot_color: [255, 220, 120],
            };
            // Wildfire variant: rare white-hot sparks over the coals.
            s.effect_tuning.insert(
                EffectsMode::EmberFlicker,
                EffectTuning { speed: 1.0, intensity: 0.65, variant: 1 },
            );
        }
        "Aurora Borealis" => {
            reset_baseline(s);
            s.effects_mode = EffectsMode::AuroraDrift;
            s.colors = ColorConfig {
                cold_color: [0, 255, 120],
                warm_color: [0, 180, 200],
                hot_color: [170, 60, 255],
            };
            s.effect_tuning.insert(
                EffectsMode::AuroraDrift,
                EffectTuning { speed: 1.0, intensity: 0.6, variant: 0 },
            );
        }
        "Comet Storm" => {
            reset_baseline(s);
            s.effects_mode = EffectsMode::CometChase;
            s.colors = ColorConfig {
                cold_color: [80, 160, 255],
                warm_color: [200, 120, 255],
                hot_color: [255, 80, 40],
            };
            s.effect_tuning.insert(
                EffectsMode::CometChase,
                EffectTuning { speed: 1.2, intensity: 0.6, variant: 1 },
            );
        }
        _ => return false,
    }
    s.active_preset = name.to_string();
    s.normalize();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_applies() {
        for p in &BUILTIN_PRESETS {
            let mut s = Settings::default();
            assert!(apply_builtin(p.name, &mut s), "{} did not apply", p.name);
            assert_eq!(s.active_preset, p.name);
        }
        let mut s = Settings::default();
        assert!(!apply_builtin("Nope", &mut s));
    }

    #[test]
    fn presets_clear_zone_overrides() {
        use crate::settings::ZoneConfig;
        let mut s = Settings::default();
        s.zones.push(ZoneConfig {
            device_name: "Board".into(),
            zone_name: "Header 1".into(),
            effect_override: Some(EffectsMode::Solid),
            ..ZoneConfig::default()
        });
        apply_builtin("Ice & Fire", &mut s);
        assert!(s.zones[0].effect_override.is_none());
        assert_eq!(s.colors.cold_color, [110, 195, 255]);
    }

    #[test]
    fn stealth_dark_dims_to_15_percent() {
        let mut s = Settings::default();
        apply_builtin("Stealth Dark", &mut s);
        assert!((s.global_brightness - 0.15).abs() < f32::EPSILON);
    }

    #[test]
    fn presets_restore_brightness_after_stealth_dark() {
        let mut s = Settings::default();
        apply_builtin("Stealth Dark", &mut s);
        apply_builtin("Thermal Alert", &mut s);
        assert!((s.global_brightness - crate::settings::DEFAULT_BRIGHTNESS).abs() < f32::EPSILON);
    }
}
