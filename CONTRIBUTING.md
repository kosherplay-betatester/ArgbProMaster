# Contributing to ArgbProMaster

Thanks for wanting to help! A few ground rules keep the project pleasant:

- **Build & test before a PR:** `cargo build --release && cargo test --release`
  (toolchain is pinned by `rust-toolchain.toml`).
- **The engine must stay deterministic.** Effects use time-seeded hash noise,
  never `rand` or wall-clock randomness — the GUI preview and the daemon must
  render bit-identical frames from the same inputs. Tests enforce this; keep
  them passing and add the same coverage for new effects.
- **New effects** go into `argb_core/src/engine.rs` + the `EffectsMode` enum
  (label, description, variant labels). The GUI picks them up automatically.
- **User-facing text**: plain language, a tooltip on every control, no jargon.
  This app is deliberately beginner-friendly.
- **Settings compatibility**: never break existing `settings.json` files — new
  fields get `#[serde(default)]`, format changes get a migration (see
  `migrate_v1` in `argb_core/src/settings.rs`) and a test.

Bug reports: please attach `%APPDATA%\ArgbProMaster\daemon.log` and your
OpenRGB version. LED-not-lighting issues are usually OpenRGB SDK-server or
Direct-mode questions — the README's Requirements section covers the common ones.
