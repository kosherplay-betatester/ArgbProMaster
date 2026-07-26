# ArgbProMaster 🌡✨

**Thermal-reactive ARGB lighting for Windows — your lights follow your temperatures.**

ArgbProMaster turns any OpenRGB-controllable lighting — motherboard ARGB headers,
RAM sticks, GPU zones, strips, keyboards — into a live thermal display. CPU heating
up in a game? Watch your underglow slide from cyan to crimson. All rendered
smoothly by a tiny background daemon, configured in a friendly dark-themed GUI
with a live preview that is pixel-identical to what your LEDs will do.

## Highlights

- 🔌 **Auto-detects every zone** OpenRGB exposes — including *empty* ARGB headers
  (shown as "set a LED count to bring it to life"). Assign each zone its own
  thermal source (CPU/GPU), effect and colors.
- 🎬 **14 built-in effects**, all temperature-aware: Thermal Wave (lava-sea flow),
  Thermal Fill (a 0–100% thermometer along the strip), Ember Flicker, Aurora
  Drift, Comet Chase, Meteor Shower, Larson Scanner, Plasma, Starfield Twinkle,
  Rain Drops, Gradient Pulse, Breathing, Spectrum Wave, Solid.
- 🎛 **Per-effect tuning** — speed, intensity and style variants (Lava Sea vs
  Classic Wave, Heartbeat breathing, Twin Comets…), each effect remembers its own.
- 🧪 **Effect Lab** — build your own effects from blocks: a multi-stop color
  palette, motion (flow / fill / chase / flicker / breathe / still), sparkle
  overlays and temperature binding. Saved effects appear in every menu with a ★.
- 🎨 **9 one-click presets** (Thermal Alert, Molten Ember, Aurora Borealis, Comet
  Storm, Cyberpunk, Vaporwave, Ice & Fire, Stealth Dark, Spectrum) + named custom
  presets snapshotting your whole setup.
- 🖥 **Live preview** with mock temperature sliders — test a 95 °C meltdown
  without heating your PC.
- 🔔 **Tray quick menu** — switch effects on the fly without opening the window.
- 🪶 **Tiny daemon** — no Electron, no services, a few MB of RAM, hot-reloads
  settings the instant you save.

## Requirements

1. **[OpenRGB](https://openrgb.org)** running with its **SDK server** enabled
   (launch with `--server`, or press "Start Server" in its SDK Server tab).
   Run OpenRGB as administrator so it can control SMBus devices like RAM.
2. **[MSI Afterburner](https://www.msi.com/Landing/afterburner)** *(optional but
   recommended)* — the daemon reads CPU/GPU temperatures from its shared memory.
   Without it, animations keep running with the last-known temperatures.
3. Windows 10/11.

## Quick start

1. Build (or grab a release): `cargo build --release` — binaries land in
   `target\release\`. Keep `configurator_gui.exe` and `thermal_daemon.exe`
   together in one folder.
2. Start OpenRGB (admin, SDK server on).
3. Run `configurator_gui.exe`. It auto-detects your zones on launch — flip the
   ones you want on in **🔌 Zones & Ports**.
4. Pick a preset (or build something in the **🧪 Effect Lab**), watch the live
   preview, then press **🚀 Run in Background**. Done — the daemon keeps your
   lights thermal-reactive and starts hidden in the tray.

To start everything with Windows: add a shortcut to `thermal_daemon.exe` to your
Startup folder (`Win+R` → `shell:startup`), and set up OpenRGB to start with
`--server --startminimized` (a Task Scheduler entry with "Run with highest
privileges" avoids a UAC prompt at every boot).

## Architecture

| Crate | Role |
|---|---|
| `argb_core` | Settings schema (+v1 migration), the animation engine, zone discovery/matching, and a minimal OpenRGB SDK network client. Shared by both binaries so the GUI preview matches the LEDs exactly. |
| `thermal_daemon` | Invisible background renderer: reads temperatures, renders frames, streams them to OpenRGB over the local SDK socket. Single-instance, auto-reconnects, hot-reloads `settings.json`. |
| `configurator_gui` | The dark-themed egui control panel: presets, zone assignment, thermal curves, effect tuning, Effect Lab, live preview, tray. |

Settings live in `%APPDATA%\ArgbProMaster\settings.json`; the daemon logs a few
lifecycle lines to `%APPDATA%\ArgbProMaster\daemon.log`.

Diagnostics: `cargo run -p thermal_daemon --example probe` dumps every
controller/zone OpenRGB reports; `--example paint` paints each motherboard zone
a distinct color to identify physical wiring.

## License

MIT — see [LICENSE](LICENSE).
