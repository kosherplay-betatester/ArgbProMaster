# ArgbProMaster 🌡✨

[![CI](https://github.com/kosherplay-betatester/ArgbProMaster/actions/workflows/ci.yml/badge.svg)](https://github.com/kosherplay-betatester/ArgbProMaster/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**Thermal-reactive ARGB lighting for Windows — your lights follow your temperatures.**

ArgbProMaster turns any lighting OpenRGB can control — motherboard ARGB headers,
RAM sticks, GPU zones, LED strips, keyboards — into a live thermal display.
CPU heating up in a game? Watch your underglow slide from cyan to crimson.
Everything is rendered by a tiny background daemon and configured in a friendly
dark-themed GUI whose live preview is **pixel-identical** to what your LEDs do.

---

## Feature tour

- 🔌 **Auto-detects every zone** OpenRGB exposes — including *empty* ARGB
  headers, shown as "💤 set a LED count to bring it to life". Give each zone its
  own thermal source (CPU or GPU), effect, colors and friendly name.
- 🎬 **14 built-in effects**, all temperature-aware:
  Thermal Wave (an organic lava-sea flow), **Thermal Fill** (the strip fills
  0–100% like a thermometer), Ember Flicker, Aurora Drift, Comet Chase, Meteor
  Shower, Larson Scanner, Plasma, Starfield Twinkle, Rain Drops, Gradient
  Pulse, Breathing, Spectrum Wave and Solid.
- 🎛 **Per-effect tuning** — every effect remembers its own Speed, Intensity and
  Style variant (Lava Sea vs Classic Wave, Calm vs Heartbeat breathing, Single
  vs Twin comets, Glowing Coals vs Wildfire…).
- 🧪 **Effect Lab** — invent your own effects from blocks: a multi-stop color
  palette, six motion types (flow / fill / chase / flicker / breathe / still),
  sparkle overlays, and a "how it listens to temperature" binding. Saved
  effects appear in every effect menu marked with ★.
- 🎨 **9 one-click presets** — Thermal Alert, Molten Ember, Aurora Borealis,
  Comet Storm, Cyberpunk 2077, Vaporwave Sunset, Ice & Fire, Stealth Dark,
  Smooth Spectrum Wave — plus named custom presets that snapshot your entire
  setup.
- 🖥 **Live preview with a thermal simulator** — drag mock CPU/GPU sliders to
  see a 95 °C meltdown without heating your PC, or tick **📡 Follow real
  temperatures** and the preview animates in perfect lockstep with your LEDs.
- 😴 **Idle Effect** — pick a temperature range (say 35–50 °C) and a calmer
  look: the moment a zone's sensor cools into the range, the idle effect kicks
  in (any builtin or your own ★ creation); heat back up and the normal effect
  returns. Configured in the Thermal Curves tab.
- 🔔 **Tray quick menu** — switch effects on the fly, without opening the window.
- 🚑 **Built-in setup assistant** — the app detects missing requirements and
  offers to fix them itself: one click installs OpenRGB or MSI Afterburner
  (via winget) and starts OpenRGB with the right flags automatically.
- 🔧 **Fix My RGB** — a panic button (⚙ Advanced) that returns everything to a
  known-good state: stock settings, every detected zone switched on, Thermal
  Alert live, daemon running. Its sibling "💡 Restore original lighting" hands
  every device back to its own built-in firmware effect instead.
- 🪶 **Tiny and honest** — native Rust, no Electron. Measured on a Ryzen
  7950X3D: the daemon streams 168 LEDs at 30 FPS using **0.024% CPU and
  4.7 MB RAM**. Identical frames are deduplicated (a static look costs zero
  traffic), settings hot-reload atomically, and any crash logs its own cause.

---

## Installation

**The short version:** get ArgbProMaster (Step 3), run `configurator_gui.exe`,
and let the **built-in setup assistant** do the rest — if OpenRGB or MSI
Afterburner is missing or not running, a banner at the top of the window
offers to **install and configure them for you automatically** (one click,
uses winget). The steps below are the manual path if you prefer doing it
yourself or something goes sideways.

### Step 1 — Install OpenRGB (required)

ArgbProMaster drives your LEDs *through* [OpenRGB](https://openrgb.org), the
open-source RGB control hub.

1. Download OpenRGB from <https://openrgb.org> and install it.
2. On Windows, also install its **PawnIO** driver when prompted (needed for
   RAM / SMBus devices): <https://pawnio.eu>
3. **Run OpenRGB as administrator** — otherwise RAM sticks and some sensors
   are invisible.
4. Enable its **SDK server**: either launch OpenRGB with the `--server` flag,
   or open OpenRGB → *SDK Server* tab → **Start Server**. Port stays at the
   default `6742`.

> ✅ Quick check: OpenRGB's SDK Server tab should say the server is running.

### Step 2 — Install MSI Afterburner (recommended)

The daemon reads CPU/GPU temperatures from
[MSI Afterburner](https://www.msi.com/Landing/afterburner)'s shared memory —
install it and let it run in the background. Without it, ArgbProMaster still
animates with the last-known temperatures (it degrades gracefully).

### Step 3 — Get ArgbProMaster

**Option A — download a release** (when available): grab the latest zip from
the [Releases page](https://github.com/kosherplay-betatester/ArgbProMaster/releases),
extract it anywhere (e.g. `C:\Apps\ArgbProMaster`). Keep both exes together.

**Option B — build from source:**

```powershell
# 1. Install Rust (one-time): https://rustup.rs
# 2. Clone and build:
git clone https://github.com/kosherplay-betatester/ArgbProMaster.git
cd ArgbProMaster
cargo build --release
```

Binaries land in `target\release\` — copy `configurator_gui.exe` **and**
`thermal_daemon.exe` into one folder together (the GUI launches the daemon
from beside itself).

---

## First run — 2 minutes

1. Start **OpenRGB** (admin, SDK server on — see Step 1).
2. Double-click **`configurator_gui.exe`**. The app scans for your hardware
   automatically on launch.
3. Open the **🔌 Zones & Ports** tab: every detected port and device is listed,
   grouped by hardware. Tick **Enabled** on the zones you want, and set each
   one's LED count if it shows "💤 empty port".
4. Pick a look in **🎨 Presets & Themes** (Thermal Alert is a great start), and
   watch the **live preview** on the right — drag the mock temperature sliders
   to see how it will react.
5. Press **🚀 Run in Background**. Settings are saved, the daemon starts, the
   window slips into the system tray — and your lights are now thermal-reactive.

Change anything later and press **💾 Save** — the daemon restyles your rig
within a second, live.

### Start with Windows (optional but nice)

- **Daemon:** press `Win+R`, type `shell:startup`, Enter — and drop a shortcut
  to `thermal_daemon.exe` into that folder.
- **OpenRGB:** create a Task Scheduler entry so it starts elevated without a
  UAC prompt: Task Scheduler → *Create Task* → tick **Run with highest
  privileges** → Trigger: *At log on* → Action: your `OpenRGB.exe` with
  arguments `--server --startminimized`.

---

## Using the app

| Tab | What it does |
|---|---|
| 🎨 **Presets & Themes** | One-click looks + save/load your own named presets. |
| 🔌 **Zones & Ports** | Everything OpenRGB detected. Per zone: enable, LED count, CPU/GPU source, its own effect (builtin or ★ custom), its own colors. |
| 🌡 **Thermal Curves** | The temperature window per sensor (e.g. 40–85 °C), the global cold→warm→hot colors, the global effect, the 🎛 per-effect tuning card (Speed / Intensity / Style), and the 😴 Idle Effect (range + look shown while temps rest). |
| 🧪 **Effect Lab** | Build, edit and manage your own ★ effects with a live preview. |
| ⚙ **Advanced** | Animation FPS, easing, brightness with a 70% Safety Power Lock, daemon start/stop, and the safety net: **🔧 Fix My RGB** (one click back to a known-good working state), **♻ Reset All to Defaults** (settings only — zones and custom presets survive), **💡 Restore original lighting** (hand every device back to its built-in firmware effect). |

**Tray icon:** left-click opens the window; right-click gives you 🎨 *Quick
Effect* (instant switching), *Quit (lights keep running)* and *⏹ Stop lights &
quit*.

---

## Troubleshooting

| Symptom | Fix |
|---|---|
| **Anything feels broken** | ⚙ Advanced → **🔧 Fix My RGB** — stock settings, zones on, daemon running, Thermal Alert live. |
| **No zones found** | OpenRGB isn't running, or its SDK server is off (Step 1.4). Click "🔄 Detect zones" after fixing. |
| **RAM sticks missing** | OpenRGB needs to run **as administrator** with the PawnIO driver installed. |
| **LEDs frozen / dark after closing everything** | Devices sit in "Direct" mode showing the last streamed frame. Start the daemon again, or set a hardware mode in OpenRGB and use its "Save to device". |
| **Colors don't react to temperature** | Is MSI Afterburner running? The daemon reads temps from its shared memory. |
| **Erratic flashing** | Usually the slow SMBus bus (RAM zones) choking the frame pipeline — disable RAM zones or lower the FPS in ⚙ Advanced. |
| **Effects fight / flicker between looks** | Two things are driving the LEDs. Don't run OpenRGB effect plugins or other RGB software at the same time as the daemon. |
| Something else | Check `%APPDATA%\ArgbProMaster\daemon.log` and open an issue with it attached. |

Diagnostics: `cargo run -p thermal_daemon --example probe` dumps every
controller/zone OpenRGB reports; `--example paint` paints each motherboard
zone a distinct color to identify which physical port is which.

---

## Architecture

| Crate | Role |
|---|---|
| `argb_core` | Settings schema (with automatic migration of older files), the deterministic animation engine, zone discovery/matching, and a minimal OpenRGB SDK network client. Shared by both binaries so the GUI preview matches the LEDs exactly. |
| `thermal_daemon` | Invisible background renderer: reads temperatures, applies exponential smoothing, renders frames, streams them over the local OpenRGB SDK socket (port 6742). Single-instance, auto-reconnects, re-asserts Direct mode if something steals it. |
| `configurator_gui` | The egui control panel: presets, zones, curves, Effect Lab, live preview, tray. |

Settings: `%APPDATA%\ArgbProMaster\settings.json` (hot-reloaded by the daemon).
Log: `%APPDATA%\ArgbProMaster\daemon.log`.

## Contributing

PRs welcome — see [CONTRIBUTING.md](CONTRIBUTING.md). The golden rule: the
engine stays deterministic (no RNG), so the preview and the LEDs always agree.

## License

MIT — see [LICENSE](LICENSE).
