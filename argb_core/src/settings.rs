use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::SystemTime;

/// How a zone reacts over time. `SpectrumWave` ignores thermals entirely.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EffectsMode {
    ThermalWave,
    GradientPulse,
    Breathing,
    Solid,
    SpectrumWave,
    EmberFlicker,
    AuroraDrift,
    CometChase,
    ThermalFill,
    MeteorShower,
    LarsonScanner,
    Plasma,
    StarfieldTwinkle,
    RainDrops,
    LightTrail,
    Fireworks,
    WaveCollide,
}

impl EffectsMode {
    pub const ALL: [EffectsMode; 17] = [
        EffectsMode::ThermalWave,
        EffectsMode::ThermalFill,
        EffectsMode::GradientPulse,
        EffectsMode::Breathing,
        EffectsMode::Solid,
        EffectsMode::SpectrumWave,
        EffectsMode::EmberFlicker,
        EffectsMode::AuroraDrift,
        EffectsMode::CometChase,
        EffectsMode::MeteorShower,
        EffectsMode::LarsonScanner,
        EffectsMode::Plasma,
        EffectsMode::StarfieldTwinkle,
        EffectsMode::RainDrops,
        EffectsMode::LightTrail,
        EffectsMode::Fireworks,
        EffectsMode::WaveCollide,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            EffectsMode::ThermalWave => "Thermal Wave",
            EffectsMode::GradientPulse => "Gradient Pulse",
            EffectsMode::Breathing => "Breathing",
            EffectsMode::Solid => "Solid",
            EffectsMode::SpectrumWave => "Spectrum Wave",
            EffectsMode::EmberFlicker => "Ember Flicker",
            EffectsMode::AuroraDrift => "Aurora Drift",
            EffectsMode::CometChase => "Comet Chase",
            EffectsMode::ThermalFill => "Thermal Fill",
            EffectsMode::MeteorShower => "Meteor Shower",
            EffectsMode::LarsonScanner => "Larson Scanner",
            EffectsMode::Plasma => "Plasma",
            EffectsMode::StarfieldTwinkle => "Starfield Twinkle",
            EffectsMode::RainDrops => "Rain Drops",
            EffectsMode::LightTrail => "Light Trail",
            EffectsMode::Fireworks => "Fireworks",
            EffectsMode::WaveCollide => "Wave Collide",
        }
    }

    pub fn describe(&self) -> &'static str {
        match self {
            EffectsMode::ThermalWave => "A flowing lava-sea wave that speeds up and shifts color as temperatures climb.",
            EffectsMode::GradientPulse => "A gentle spatial gradient that stretches toward the hot color and pulses softly.",
            EffectsMode::Breathing => "The whole zone slowly breathes in the current thermal color.",
            EffectsMode::Solid => "A steady solid color mapped to the current temperature.",
            EffectsMode::SpectrumWave => "A classic smooth rainbow wave, completely independent of thermals.",
            EffectsMode::EmberFlicker => "Every LED flickers like a glowing coal — livelier as temperatures rise.",
            EffectsMode::AuroraDrift => "Slow drifting curtains of blended color, like northern lights.",
            EffectsMode::CometChase => "A bright comet sweeps the strip with a fading tail, faster when hot.",
            EffectsMode::ThermalFill => "The strip fills up like a thermometer — 0 to 100% along the LEDs with the full gradient.",
            EffectsMode::MeteorShower => "Several meteors streak along the strip with fading trails.",
            EffectsMode::LarsonScanner => "A glowing eye sweeps back and forth — pure sci-fi.",
            EffectsMode::Plasma => "Slowly swirling interference of your three colors, like liquid light.",
            EffectsMode::StarfieldTwinkle => "A dark sky where stars twinkle in and out, colored by temperature.",
            EffectsMode::RainDrops => "Drops splash onto the strip and ripple outward before fading.",
            EffectsMode::LightTrail => "A glowing trail orbits the dark strip in endless circles — smooth gradient tail on both sides, colored by your scheme.",
            EffectsMode::Fireworks => "Rockets shoot up the strip and burst into fading sparkles — more the hotter it gets.",
            EffectsMode::WaveCollide => "Two pulses race in from the ends and splash bright where they collide.",
        }
    }

    /// Discrete style variants for this effect ("animation variants" in the UI).
    /// Empty slice = the effect has a single style.
    pub fn variant_labels(&self) -> &'static [&'static str] {
        match self {
            EffectsMode::ThermalWave => &["Lava Sea", "Classic Wave"],
            EffectsMode::Breathing => &["Calm", "Heartbeat"],
            EffectsMode::SpectrumWave => &["Vivid", "Pastel"],
            EffectsMode::EmberFlicker => &["Glowing Coals", "Wildfire"],
            EffectsMode::AuroraDrift => &["Curtains", "Rolling Waves"],
            EffectsMode::CometChase => &["Single Comet", "Twin Comets"],
            EffectsMode::ThermalFill => &["Rising Fill", "Center Bloom"],
            EffectsMode::MeteorShower => &["Steady", "Chaotic"],
            EffectsMode::LarsonScanner => &["Classic", "Dual Eye"],
            EffectsMode::Plasma => &["Smooth", "Electric"],
            EffectsMode::StarfieldTwinkle => &["Calm Stars", "Shooting Stars"],
            EffectsMode::RainDrops => &["Drizzle", "Storm"],
            EffectsMode::LightTrail => &["Single Trail", "Twin Trails"],
            EffectsMode::Fireworks => &["Classic", "Grand Finale"],
            EffectsMode::WaveCollide => &["Center Splash", "Ping Pong"],
            EffectsMode::GradientPulse | EffectsMode::Solid => &[],
        }
    }

    /// Label for the Intensity slider when an effect gives it a more
    /// specific meaning.
    pub fn intensity_label(&self) -> &'static str {
        match self {
            EffectsMode::LightTrail => "Trail length",
            _ => "Intensity",
        }
    }

    /// Effects that use the extra Detail knob expose it under this label.
    pub fn detail_label(&self) -> Option<&'static str> {
        match self {
            EffectsMode::LightTrail => Some("Resolution"),
            _ => None,
        }
    }
}

/// Per-effect animation tuning. Defaults reproduce each effect's stock look.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct EffectTuning {
    /// Time multiplier for the whole animation, 0.25–3.0.
    pub speed: f32,
    /// How pronounced the effect's motion/contrast is, 0–1.
    pub intensity: f32,
    /// Index into `EffectsMode::variant_labels`.
    pub variant: u32,
    /// Extra per-effect knob (see `EffectsMode::detail_label`), 0–1.
    /// Light Trail: brightness resolution — 1.0 is perfectly smooth,
    /// lower values quantize the trail into visible retro steps.
    pub detail: f32,
}

impl Default for EffectTuning {
    fn default() -> Self {
        EffectTuning { speed: 1.0, intensity: 0.5, variant: 0, detail: 1.0 }
    }
}

impl EffectTuning {
    pub fn clamped(mut self, mode: EffectsMode) -> Self {
        self.speed = self.speed.clamp(0.25, 3.0);
        self.intensity = self.intensity.clamp(0.0, 1.0);
        self.detail = self.detail.clamp(0.0, 1.0);
        let variants = mode.variant_labels().len().max(1) as u32;
        self.variant = self.variant.min(variants - 1);
        self
    }
}

/// Which system component / metric drives a zone's animation.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetSource {
    #[serde(rename = "CPU")]
    Cpu,
    #[serde(rename = "GPU")]
    Gpu,
    #[serde(rename = "CPU Load")]
    CpuLoad,
    #[serde(rename = "GPU Load")]
    GpuLoad,
    #[serde(rename = "RAM")]
    Ram,
    #[serde(rename = "FPS")]
    Fps,
}

impl TargetSource {
    pub const ALL: [TargetSource; 6] = [
        TargetSource::Cpu,
        TargetSource::Gpu,
        TargetSource::CpuLoad,
        TargetSource::GpuLoad,
        TargetSource::Ram,
        TargetSource::Fps,
    ];

    /// Stable slot in the engine's source-value arrays.
    pub fn index(self) -> usize {
        match self {
            TargetSource::Cpu => 0,
            TargetSource::Gpu => 1,
            TargetSource::CpuLoad => 2,
            TargetSource::GpuLoad => 3,
            TargetSource::Ram => 4,
            TargetSource::Fps => 5,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            TargetSource::Cpu => "CPU °C",
            TargetSource::Gpu => "GPU °C",
            TargetSource::CpuLoad => "CPU Load",
            TargetSource::GpuLoad => "GPU Load",
            TargetSource::Ram => "RAM Use",
            TargetSource::Fps => "FPS",
        }
    }

    pub fn describe(&self) -> &'static str {
        match self {
            TargetSource::Cpu => "CPU temperature — the classic thermal display.",
            TargetSource::Gpu => "GPU temperature — great for game load.",
            TargetSource::CpuLoad => "CPU usage in % — reacts instantly to work, not heat.",
            TargetSource::GpuLoad => "GPU usage in % — lights up the moment a game starts rendering.",
            TargetSource::Ram => "RAM usage in % of installed memory.",
            TargetSource::Fps => "Framerate — cold when it stutters, hot when it flies.",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorConfig {
    pub cold_color: [u8; 3],
    pub warm_color: [u8; 3],
    pub hot_color: [u8; 3],
}

impl Default for ColorConfig {
    fn default() -> Self {
        // "Thermal Alert" palette: Cold = Cyan, Warm = Purple, Hot = Crimson.
        ColorConfig {
            cold_color: [0, 255, 200],
            warm_color: [180, 0, 255],
            hot_color: [255, 10, 10],
        }
    }
}

impl ColorConfig {
    /// The classic 3-color journey as gradient stops.
    pub fn stops(&self) -> Vec<(f32, [u8; 3])> {
        vec![(0.0, self.cold_color), (0.5, self.warm_color), (1.0, self.hot_color)]
    }
}

/// How a custom effect moves along the strip.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionKind {
    /// The palette flows continuously along the strip.
    Flow,
    /// The strip fills like a bar/thermometer.
    Fill,
    /// A bright head with a fading tail sweeps the strip.
    Chase,
    /// Every LED flickers organically.
    Flicker,
    /// The whole strip breathes in and out.
    Breathe,
    /// A static gradient, no motion.
    Still,
}

impl MotionKind {
    pub const ALL: [MotionKind; 6] = [
        MotionKind::Flow,
        MotionKind::Fill,
        MotionKind::Chase,
        MotionKind::Flicker,
        MotionKind::Breathe,
        MotionKind::Still,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            MotionKind::Flow => "Flowing wave",
            MotionKind::Fill => "Filling bar",
            MotionKind::Chase => "Chase with trail",
            MotionKind::Flicker => "Organic flicker",
            MotionKind::Breathe => "Breathing",
            MotionKind::Still => "Still gradient",
        }
    }
}

/// An optional animated layer on top of the base motion.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayKind {
    None,
    Shimmer,
    Pulse,
    Sparks,
}

impl OverlayKind {
    pub const ALL: [OverlayKind; 4] =
        [OverlayKind::None, OverlayKind::Shimmer, OverlayKind::Pulse, OverlayKind::Sparks];
    pub fn label(&self) -> &'static str {
        match self {
            OverlayKind::None => "None",
            OverlayKind::Shimmer => "Shimmer",
            OverlayKind::Pulse => "Soft pulse",
            OverlayKind::Sparks => "Sparks",
        }
    }
}

/// How the custom effect listens to the zone's temperature.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThermalBind {
    /// Palette position follows temperature (cold end → hot end).
    PalettePosition,
    /// Animation speed rises with temperature.
    Speed,
    /// Fill level = temperature (great with the Filling bar motion).
    FillLevel,
    /// Ignore temperature entirely.
    None,
}

impl ThermalBind {
    pub const ALL: [ThermalBind; 4] = [
        ThermalBind::PalettePosition,
        ThermalBind::Speed,
        ThermalBind::FillLevel,
        ThermalBind::None,
    ];
    pub fn label(&self) -> &'static str {
        match self {
            ThermalBind::PalettePosition => "Colors follow temperature",
            ThermalBind::Speed => "Speed follows temperature",
            ThermalBind::FillLevel => "Fill level = temperature",
            ThermalBind::None => "Ignore temperature",
        }
    }
}

/// A user-built effect from the Effect Lab: a multi-stop palette plus motion,
/// overlay and thermal-binding blocks. Rendered by `engine::render_custom`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct CustomEffect {
    pub name: String,
    /// (position 0..1, color) stops, kept sorted by position; 2..=8 stops.
    pub palette: Vec<(f32, [u8; 3])>,
    pub motion: MotionKind,
    pub reverse: bool,
    pub speed: f32,
    /// Pattern size: wave density, tail length or edge softness (0..1).
    pub scale: f32,
    pub overlay: OverlayKind,
    pub overlay_strength: f32,
    pub thermal: ThermalBind,
}

impl Default for CustomEffect {
    fn default() -> Self {
        CustomEffect {
            name: String::new(),
            palette: vec![(0.0, [0, 255, 200]), (0.5, [180, 0, 255]), (1.0, [255, 10, 10])],
            motion: MotionKind::Flow,
            reverse: false,
            speed: 1.0,
            scale: 0.5,
            overlay: OverlayKind::None,
            overlay_strength: 0.5,
            thermal: ThermalBind::PalettePosition,
        }
    }
}

impl CustomEffect {
    /// Clamp and sort everything into a renderable state.
    pub fn sanitize(&mut self) {
        self.speed = self.speed.clamp(0.25, 3.0);
        self.scale = self.scale.clamp(0.0, 1.0);
        self.overlay_strength = self.overlay_strength.clamp(0.0, 1.0);
        if self.palette.is_empty() {
            self.palette = CustomEffect::default().palette;
        }
        self.palette.truncate(8);
        for stop in self.palette.iter_mut() {
            stop.0 = stop.0.clamp(0.0, 1.0);
        }
        self.palette
            .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    }
}

/// One controllable zone, matched against live hardware by
/// (`device_name`, `zone_name`). Discovered zones are appended by
/// `zones::merge`; legacy v1 files migrate into wildcard entries that
/// concretize on first detection.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ZoneConfig {
    pub device_name: String,
    pub device_type: i32,
    /// Empty = whole device; `"*2"` style = legacy motherboard suffix match.
    pub zone_name: String,
    /// User-editable friendly label shown in the GUI.
    pub display_name: String,
    pub enabled: bool,
    /// 0 = use whatever LED count the server reports.
    pub led_count: u32,
    pub target_source: TargetSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_override: Option<EffectsMode>,
    /// A custom (Effect Lab) effect by name; wins over `effect_override`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_effect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors_override: Option<ColorConfig>,
    /// Detection bookkeeping, refreshed on every scan so the GUI can render
    /// meaningfully even before the next detection.
    pub last_seen_leds: u32,
    pub resizable: bool,
    pub max_leds: u32,
}

impl Default for ZoneConfig {
    fn default() -> Self {
        ZoneConfig {
            device_name: String::new(),
            device_type: 0,
            zone_name: String::new(),
            display_name: String::new(),
            enabled: false,
            led_count: 0,
            target_source: TargetSource::Cpu,
            effect_override: None,
            custom_effect: None,
            colors_override: None,
            last_seen_leds: 0,
            resizable: false,
            max_leds: 0,
        }
    }
}

impl ZoneConfig {
    /// The LED count to render: the user's choice, else what the server saw.
    pub fn effective_leds(&self) -> u32 {
        if self.led_count > 0 {
            if self.max_leds > 0 {
                self.led_count.min(self.max_leds)
            } else {
                self.led_count
            }
        } else {
            self.last_seen_leds
        }
    }
}

/// Everything a preset snapshots — the full "look" of the rig.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PresetData {
    pub global_brightness: f32,
    pub animation_fps: u32,
    pub smoothing_speed: f32,
    pub effects_mode: EffectsMode,
    pub cpu_temp_min: f32,
    pub cpu_temp_max: f32,
    pub gpu_temp_min: f32,
    pub gpu_temp_max: f32,
    pub colors: ColorConfig,
    #[serde(default)]
    pub zones: Vec<ZoneConfig>,
    /// Missing in snapshots saved before per-effect tuning existed.
    #[serde(default)]
    pub effect_tuning: BTreeMap<EffectsMode, EffectTuning>,
    #[serde(default)]
    pub global_custom_effect: Option<String>,
    #[serde(default)]
    pub idle_enabled: bool,
    #[serde(default = "default_idle_min")]
    pub idle_temp_min: f32,
    #[serde(default = "default_idle_max")]
    pub idle_temp_max: f32,
    #[serde(default = "default_idle_effect")]
    pub idle_effect: EffectsMode,
    #[serde(default)]
    pub idle_custom_effect: Option<String>,
    #[serde(default)]
    pub idle_colors: Option<ColorConfig>,
    #[serde(default)]
    pub idle_tuning: Option<EffectTuning>,
    #[serde(default)]
    pub global_stops: Vec<(f32, [u8; 3])>,
}

fn default_idle_min() -> f32 {
    35.0
}
fn default_idle_max() -> f32 {
    50.0
}
fn default_idle_effect() -> EffectsMode {
    EffectsMode::Breathing
}

impl PresetData {
    pub fn capture(s: &Settings) -> Self {
        PresetData {
            global_brightness: s.global_brightness,
            animation_fps: s.animation_fps,
            smoothing_speed: s.smoothing_speed,
            effects_mode: s.effects_mode,
            cpu_temp_min: s.cpu_temp_min,
            cpu_temp_max: s.cpu_temp_max,
            gpu_temp_min: s.gpu_temp_min,
            gpu_temp_max: s.gpu_temp_max,
            colors: s.colors,
            zones: s.zones.clone(),
            effect_tuning: s.effect_tuning.clone(),
            global_custom_effect: s.global_custom_effect.clone(),
            idle_enabled: s.idle_enabled,
            idle_temp_min: s.idle_temp_min,
            idle_temp_max: s.idle_temp_max,
            idle_effect: s.idle_effect,
            idle_custom_effect: s.idle_custom_effect.clone(),
            idle_colors: s.idle_colors,
            idle_tuning: s.idle_tuning,
            global_stops: s.global_stops.clone(),
        }
    }

    pub fn apply(&self, s: &mut Settings) {
        s.global_brightness = self.global_brightness;
        s.animation_fps = self.animation_fps;
        s.smoothing_speed = self.smoothing_speed;
        s.effects_mode = self.effects_mode;
        s.cpu_temp_min = self.cpu_temp_min;
        s.cpu_temp_max = self.cpu_temp_max;
        s.gpu_temp_min = self.gpu_temp_min;
        s.gpu_temp_max = self.gpu_temp_max;
        s.colors = self.colors;
        s.zones = self.zones.clone();
        s.effect_tuning = self.effect_tuning.clone();
        s.global_custom_effect = self.global_custom_effect.clone();
        s.idle_enabled = self.idle_enabled;
        s.idle_temp_min = self.idle_temp_min;
        s.idle_temp_max = self.idle_temp_max;
        s.idle_effect = self.idle_effect;
        s.idle_custom_effect = self.idle_custom_effect.clone();
        s.idle_colors = self.idle_colors;
        s.idle_tuning = self.idle_tuning;
        s.global_stops = self.global_stops.clone();
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CustomPreset {
    pub name: String,
    pub data: PresetData,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub active_preset: String,
    pub global_brightness: f32,
    pub animation_fps: u32,
    pub smoothing_speed: f32,
    pub effects_mode: EffectsMode,
    pub cpu_temp_min: f32,
    pub cpu_temp_max: f32,
    pub gpu_temp_min: f32,
    pub gpu_temp_max: f32,
    pub colors: ColorConfig,
    pub zones: Vec<ZoneConfig>,
    /// When true, the brightness slider is soft-capped at 70%.
    pub safety_power_lock: bool,
    pub custom_presets: Vec<CustomPreset>,
    /// Per-effect speed/intensity/variant tuning. Effects without an entry
    /// (including every effect in pre-tuning settings files) use defaults.
    pub effect_tuning: BTreeMap<EffectsMode, EffectTuning>,
    /// The Effect Lab library: user-built effects, selectable anywhere.
    pub custom_effects: Vec<CustomEffect>,
    /// A custom effect used as the global effect (wins over `effects_mode`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_custom_effect: Option<String>,
    /// Idle mode: when a zone's temperature sits inside [idle_temp_min,
    /// idle_temp_max] °C, it shows the idle effect instead of its normal one.
    pub idle_enabled: bool,
    pub idle_temp_min: f32,
    pub idle_temp_max: f32,
    pub idle_effect: EffectsMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_custom_effect: Option<String>,
    /// The idle look's own colors; None = the zone's normal colors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_colors: Option<ColorConfig>,
    /// The idle look's own speed/intensity; None = the effect's global tuning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_tuning: Option<EffectTuning>,
    /// Multi-stop Color Journey (2..=8 stops). Empty = use the classic
    /// 3-color `colors` above. Zones with color overrides keep their own.
    pub global_stops: Vec<(f32, [u8; 3])>,
}

/// Stock brightness, shared by `Settings::default` and the preset baseline
/// so they cannot drift apart. Distinct from the 0.70 safety-lock cap in
/// `normalize`, which is only coincidentally the same value.
pub const DEFAULT_BRIGHTNESS: f32 = 0.70;

impl Default for Settings {
    fn default() -> Self {
        Settings {
            active_preset: "Thermal Alert".to_string(),
            global_brightness: DEFAULT_BRIGHTNESS,
            // 30 FPS is the sweet spot for USB LED controllers: silky to the
            // eye, comfortably within what their links sustain. Higher rates
            // can overwhelm slower controllers into erratic flicker.
            animation_fps: 30,
            smoothing_speed: 0.02,
            effects_mode: EffectsMode::ThermalWave,
            cpu_temp_min: 40.0,
            cpu_temp_max: 85.0,
            gpu_temp_min: 30.0,
            gpu_temp_max: 75.0,
            colors: ColorConfig::default(),
            zones: Vec::new(),
            safety_power_lock: true,
            custom_presets: Vec::new(),
            effect_tuning: BTreeMap::new(),
            custom_effects: Vec::new(),
            global_custom_effect: None,
            idle_enabled: false,
            idle_temp_min: 35.0,
            idle_temp_max: 50.0,
            idle_effect: EffectsMode::Breathing,
            idle_custom_effect: None,
            idle_colors: None,
            idle_tuning: None,
            global_stops: Vec::new(),
        }
    }
}

impl Settings {
    /// Keep derived / invariant fields coherent before persisting or rendering.
    pub fn normalize(&mut self) {
        if self.cpu_temp_max <= self.cpu_temp_min + 1.0 {
            self.cpu_temp_max = self.cpu_temp_min + 1.0;
        }
        if self.gpu_temp_max <= self.gpu_temp_min + 1.0 {
            self.gpu_temp_max = self.gpu_temp_min + 1.0;
        }
        let cap = if self.safety_power_lock { 0.70 } else { 1.0 };
        self.global_brightness = self.global_brightness.clamp(0.0, cap);
        self.animation_fps = self.animation_fps.clamp(1, 60);
        self.smoothing_speed = self.smoothing_speed.clamp(0.005, 1.0);
        for (mode, tuning) in self.effect_tuning.iter_mut() {
            *tuning = tuning.clamped(*mode);
        }
        for fx in self.custom_effects.iter_mut() {
            fx.sanitize();
        }
        self.idle_temp_min = self.idle_temp_min.clamp(0.0, 109.0);
        if self.idle_temp_max <= self.idle_temp_min + 1.0 {
            self.idle_temp_max = self.idle_temp_min + 1.0;
        }
        self.idle_temp_max = self.idle_temp_max.clamp(1.0, 110.0);
        if let Some(t) = self.idle_tuning {
            self.idle_tuning = Some(t.clamped(self.idle_effect));
        }
        // Multi-stop journey: sorted, clamped, 2..=8 stops (or empty = off).
        self.global_stops.truncate(8);
        for stop in self.global_stops.iter_mut() {
            stop.0 = stop.0.clamp(0.0, 1.0);
        }
        self.global_stops
            .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        if self.global_stops.len() == 1 {
            self.global_stops.clear();
        }
    }

    /// The gradient every zone WITHOUT a color override travels: the custom
    /// multi-stop journey when configured, else the classic 3 colors.
    pub fn journey_stops(&self) -> Vec<(f32, [u8; 3])> {
        if self.global_stops.len() >= 2 {
            self.global_stops.clone()
        } else {
            self.colors.stops()
        }
    }

    /// Look up a custom effect by name.
    pub fn custom_effect(&self, name: &str) -> Option<&CustomEffect> {
        self.custom_effects.iter().find(|f| f.name == name)
    }

    /// Tuning for an effect, falling back to defaults when never customized.
    pub fn tuning(&self, mode: EffectsMode) -> EffectTuning {
        self.effect_tuning
            .get(&mode)
            .copied()
            .unwrap_or_default()
            .clamped(mode)
    }

    pub fn load_or_default() -> Settings {
        match std::fs::read_to_string(settings_path()) {
            Ok(text) => Settings::from_json(&text).unwrap_or_default(),
            Err(_) => Settings::default(),
        }
    }

    /// Parse a settings JSON document, transparently migrating v1 files
    /// (hardcoded port_2/port_3/ram) into the dynamic zone list.
    pub fn from_json(text: &str) -> Option<Settings> {
        let mut value: serde_json::Value = serde_json::from_str(text).ok()?;
        migrate_v1(&mut value);
        let mut s: Settings = serde_json::from_value(value).ok()?;
        s.normalize();
        Some(s)
    }

    /// Load the settings file, writing the defaults to disk if it is missing
    /// or unreadable (used by the daemon so it always has a file to watch).
    pub fn load_or_create() -> Settings {
        let path = settings_path();
        if !path.exists() {
            let s = Settings::default();
            let _ = s.save();
            return s;
        }
        Settings::load_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let mut copy = self.clone();
        copy.normalize();
        let path = settings_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(&copy)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        // Write-then-rename so the hot-reloading daemon can never observe a
        // half-written file (a torn read used to reset it to defaults).
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &path)
    }
}

/// Rewrite a v1 settings document (fixed `ports` object) into the v2 shape
/// (dynamic `zones` array). Port 2/3 become motherboard suffix wildcards that
/// `zones::merge` concretizes on first detection; RAM becomes an every-DRAM
/// wildcard. Applies to the top level and to every custom preset snapshot.
fn migrate_v1(value: &mut serde_json::Value) {
    use serde_json::{json, Map, Value};

    fn ports_to_zones(obj: &mut Map<String, Value>) {
        if obj.contains_key("zones") || !obj.contains_key("ports") {
            return;
        }
        let ports = obj.remove("ports").unwrap_or(Value::Null);
        let mut zones = Vec::new();
        let grab = |p: &Value, key: &str| p.get(key).cloned().unwrap_or(Value::Null);
        for (key, wildcard) in [("port_2", "*2"), ("port_3", "*3")] {
            if let Some(p) = ports.get(key) {
                zones.push(json!({
                    "device_name": "",
                    "device_type": 0,
                    "zone_name": wildcard,
                    "display_name": grab(p, "description"),
                    "enabled": grab(p, "enabled"),
                    "led_count": grab(p, "led_count"),
                    "target_source": grab(p, "target_source"),
                    "effect_override": grab(p, "mode_override"),
                    "colors_override": grab(p, "colors_override"),
                }));
            }
        }
        if let Some(r) = ports.get("ram") {
            zones.push(json!({
                "device_name": "",
                "device_type": 1,
                "zone_name": "",
                "display_name": grab(r, "description"),
                "enabled": grab(r, "enabled"),
                "led_count": 0,
                "target_source": grab(r, "target_source"),
                "effect_override": grab(r, "mode_override"),
                "colors_override": grab(r, "colors_override"),
            }));
        }
        obj.insert("zones".to_string(), Value::Array(zones));
    }

    if let Some(obj) = value.as_object_mut() {
        ports_to_zones(obj);
        if let Some(presets) = obj.get_mut("custom_presets").and_then(|v| v.as_array_mut()) {
            for preset in presets {
                if let Some(data) = preset.get_mut("data").and_then(|d| d.as_object_mut()) {
                    ports_to_zones(data);
                }
            }
        }
    }
}

/// `%APPDATA%\ArgbProMaster` on Windows, next to the executable otherwise.
pub fn settings_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata).join("ArgbProMaster");
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn settings_path() -> PathBuf {
    settings_dir().join("settings.json")
}

pub fn settings_mtime() -> Option<SystemTime> {
    std::fs::metadata(settings_path()).ok()?.modified().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_community_generic() {
        let s = Settings::default();
        assert_eq!(s.active_preset, "Thermal Alert");
        assert!((s.global_brightness - 0.70).abs() < f32::EPSILON);
        assert_eq!(s.effects_mode, EffectsMode::ThermalWave);
        assert_eq!(s.colors.cold_color, [0, 255, 200]);
        // Fresh installs start with no zones — detection populates them.
        assert!(s.zones.is_empty());
    }

    #[test]
    fn serde_roundtrip() {
        let mut s = Settings::default();
        s.zones.push(ZoneConfig {
            device_name: "Board".into(),
            zone_name: "Header 1".into(),
            enabled: true,
            led_count: 30,
            effect_override: Some(EffectsMode::Solid),
            ..ZoneConfig::default()
        });
        s.custom_presets.push(CustomPreset {
            name: "My Rig".into(),
            data: PresetData::capture(&s),
        });
        let json = serde_json::to_string_pretty(&s).unwrap();
        let back = Settings::from_json(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn v1_settings_files_migrate_to_zones() {
        let v1 = r#"{
            "active_preset": "cpu and gpu",
            "ports": {
                "port_2": {
                    "enabled": true, "led_count": 72,
                    "target_source": "GPU", "description": "Front Fans (3 Strips)"
                },
                "port_3": {
                    "enabled": true, "led_count": 96,
                    "target_source": "CPU", "description": "Bottom Underglow (4 Strips)",
                    "mode_override": "Solid"
                },
                "ram": {
                    "enabled": false, "sticks": 2, "leds_per_stick": 10,
                    "total_led_count": 20, "target_source": "CPU",
                    "description": "Corsair Vengeance RGB DDR5"
                }
            },
            "custom_presets": [{
                "name": "old",
                "data": {
                    "global_brightness": 0.7, "animation_fps": 30, "smoothing_speed": 0.02,
                    "effects_mode": "ThermalWave",
                    "cpu_temp_min": 40.0, "cpu_temp_max": 85.0,
                    "gpu_temp_min": 30.0, "gpu_temp_max": 75.0,
                    "colors": {"cold_color": [0,255,200], "warm_color": [180,0,255], "hot_color": [255,10,10]},
                    "ports": {
                        "port_2": {"enabled": true, "led_count": 72, "target_source": "GPU", "description": "x"},
                        "port_3": {"enabled": true, "led_count": 96, "target_source": "CPU", "description": "y"},
                        "ram": {"enabled": false, "sticks": 2, "leds_per_stick": 10, "total_led_count": 20, "target_source": "CPU", "description": "z"}
                    }
                }
            }]
        }"#;
        let s = Settings::from_json(v1).expect("v1 file must parse");
        assert_eq!(s.zones.len(), 3);
        let p2 = &s.zones[0];
        assert_eq!(p2.zone_name, "*2");
        assert!(p2.enabled);
        assert_eq!(p2.led_count, 72);
        assert_eq!(p2.target_source, TargetSource::Gpu);
        let p3 = &s.zones[1];
        assert_eq!(p3.zone_name, "*3");
        assert_eq!(p3.effect_override, Some(EffectsMode::Solid));
        let ram = &s.zones[2];
        assert_eq!(ram.device_type, 1);
        assert!(!ram.enabled);
        // The stored custom preset migrated too.
        assert_eq!(s.custom_presets[0].data.zones.len(), 3);
    }

    #[test]
    fn target_source_uses_plan_spelling() {
        let json = serde_json::to_string(&TargetSource::Gpu).unwrap();
        assert_eq!(json, "\"GPU\"");
    }

    #[test]
    fn tuning_roundtrips_and_defaults_when_absent() {
        let mut s = Settings::default();
        s.effect_tuning.insert(
            EffectsMode::CometChase,
            EffectTuning { speed: 2.0, intensity: 0.8, variant: 1, ..EffectTuning::default() },
        );
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
        // Effects never customized fall back to stock tuning.
        assert_eq!(back.tuning(EffectsMode::ThermalWave), EffectTuning::default());
        assert_eq!(back.tuning(EffectsMode::CometChase).variant, 1);
    }

    #[test]
    fn pre_tuning_settings_files_still_parse() {
        // A settings.json written before effect_tuning existed.
        let mut old = serde_json::to_value(Settings::default()).unwrap();
        old.as_object_mut().unwrap().remove("effect_tuning");
        let s: Settings = serde_json::from_value(old).unwrap();
        assert!(s.effect_tuning.is_empty());
        // Same for custom preset snapshots.
        let mut preset = serde_json::to_value(PresetData::capture(&s)).unwrap();
        preset.as_object_mut().unwrap().remove("effect_tuning");
        let p: PresetData = serde_json::from_value(preset).unwrap();
        assert!(p.effect_tuning.is_empty());
    }

    #[test]
    fn normalize_clamps_effect_tuning() {
        let mut s = Settings::default();
        s.effect_tuning.insert(
            EffectsMode::Solid,
            EffectTuning { speed: 99.0, intensity: -1.0, variant: 7, ..EffectTuning::default() },
        );
        s.normalize();
        let t = s.effect_tuning[&EffectsMode::Solid];
        assert_eq!(t.speed, 3.0);
        assert_eq!(t.intensity, 0.0);
        assert_eq!(t.variant, 0); // Solid has no variants
    }

    #[test]
    fn normalize_caps_brightness_under_safety_lock() {
        let mut s = Settings::default();
        s.global_brightness = 0.95;
        s.safety_power_lock = true;
        s.normalize();
        assert!((s.global_brightness - 0.70).abs() < f32::EPSILON);
    }

    #[test]
    fn effective_leds_prefers_user_count_capped_by_max() {
        let mut z = ZoneConfig { led_count: 0, last_seen_leds: 30, max_leds: 120, ..ZoneConfig::default() };
        assert_eq!(z.effective_leds(), 30); // auto
        z.led_count = 72;
        assert_eq!(z.effective_leds(), 72);
        z.led_count = 500;
        assert_eq!(z.effective_leds(), 120); // clamped to hardware max
    }
}
