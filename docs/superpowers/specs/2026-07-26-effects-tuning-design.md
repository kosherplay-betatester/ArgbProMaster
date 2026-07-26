# Effects Expansion & Per-Effect Tuning — Design

Date: 2026-07-26 · Status: approved by default choices (user AFK; recommended options taken)

## Goal

The user asked for: an apply-default-settings button, more effects, and per-effect
control of colors, speed and animation variants.

Findings from code exploration:
- **Reset button already exists** (Advanced tab → "♻ Reset All to Defaults", keeps
  custom presets). No work needed; surface it to the user.
- **Per-port effect + color overrides already exist** (Ports tab). No work needed.
- Missing: more effects, and per-effect speed/intensity/variant tuning.

## Scope

1. **Three new `EffectsMode` variants** (total 8):
   - `EmberFlicker` — per-LED glowing-coals flicker via deterministic hash noise
     (no RNG — GUI preview must render identically to the daemon). Variants:
     "Glowing Coals" / "Wildfire" (adds bright sparks).
   - `AuroraDrift` — very slow counter-drifting color curtains. Variants:
     "Curtains" / "Rolling Waves" (extra layer, a bit faster).
   - `CometChase` — bright head with exponential-fade tail sweeping the strip;
     faster when hot. Variants: "Single Comet" / "Twin Comets" (opposite directions).
2. **`EffectTuning { speed, intensity, variant }`** stored per effect in
   `Settings.effect_tuning: BTreeMap<EffectsMode, EffectTuning>`:
   - `speed` 0.25–3.0× (default 1.0) multiplies each effect's time terms.
   - `intensity` 0–1 (default 0.5) drives ripple depth / shimmer / tail length /
     flicker depth per effect. Defaults must reproduce the current look exactly.
   - `variant` selects the discrete style; existing effects gain variants too:
     Thermal Wave "Lava Sea"/"Classic Wave", Breathing "Calm"/"Heartbeat",
     Spectrum "Vivid"/"Pastel". Solid and Gradient Pulse have none.
3. **GUI**: new "🎛 Effect Tuning" card in the Thermal Curves tab under the Global
   Effect picker — edits the tuning of the currently selected global effect
   (Speed slider, Intensity slider, Style combo). Tuning applies wherever that
   effect is used, including port overrides.

## Compatibility & data flow

- `Settings` and `PresetData` gain the map with `#[serde(default)]`— old
  settings.json and old custom presets parse unchanged (empty map = defaults).
- `engine::render_zone` gains a `tuning: EffectTuning` parameter;
  `render_configured_zone` looks tuning up from `Settings` itself, so the GUI
  preview needs no changes. The daemon's local `zone_frame` passes
  `settings.tuning(mode)`.
- `Settings::normalize()` clamps all stored tunings (speed/intensity ranges,
  variant < variant count).

## Testing

- Engine: existing ALL-modes length/black-at-zero-brightness test auto-covers the
  new modes; add determinism test (same inputs → identical frames) and
  variant/speed-sensitivity tests.
- Settings: serde roundtrip with tuning; old-JSON-without-field still parses;
  normalize clamps out-of-range tuning.

## Out of scope (YAGNI)

- Per-effect color palettes (zone color overrides already cover color control).
- Per-port tuning overrides (tuning is per-effect, global).
- New preset cards using the new effects.
