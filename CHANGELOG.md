# Changelog

## 1.4.1 — Stress-Proof (2026-07-30)
- **No more flicker under stress tests — proven by a 150 s all-core burn
  with zero connection losses.** An all-core CPU burn (FurMark,
  Cinebench…) starves the OpenRGB server of CPU. Two failure modes
  followed: the daemon's periodic "are you still in Direct mode?"
  round-trip timed out (slow ≠ dead) and forced a reconnect that jolted
  every LED ~every 17 s — and even without questions, a long burn let
  the LED stream slowly outpace the starved server until a socket
  buffer filled and a write timed out (TCP hides congestion until the
  moment it's total). The fix is proactive: the daemon watches
  whole-machine CPU load straight from the kernel (sensor apps freeze
  stale under exactly this load), and while the machine is pegged it
  eases the frame rate (production stays far under the starved server's
  drain rate) and pauses mode checks — announcing both transitions in
  daemon.log. Backpressure detection, 20 s timeouts and a protocol-
  handshake retry back it up. Frame-rate changes are invisible: every
  effect is clock-driven, so nothing speeds up, slows down or jumps.
- The CPU/GPU Response Range cards are visible in BOTH 🎯 scopes again —
  they're shared by every zone and journey, and now say so.
- Still featherweight: 0.02% CPU / 5.3 MB RAM measured while streaming.

## 1.4 — Total Control (2026-07-30)
- **🎯 Per-zone everything**: a scope selector in 🌡 Thermal Curves edits
  either the whole rig (🌐 All zones) or one port/device at a time. Each
  zone can now carry its own multi-stop Color Journey, its own effect,
  **⤾ reverse direction**, its own **speed & intensity**, and a **fully
  independent 😴 idle setup** (own range, effect, colors and pace — or opt
  out of idle entirely). One-click 📢 buttons bring every zone back to the
  global look.
- **Idle multi-color journey**: the idle look can travel its own custom
  gradient (up to 8 stops) — globally and per zone.
- **8 new effects (25 total)**: DNA Helix, Lightning Storm, Candle Flame,
  Ocean Tide, Pendulum Wave, Stardust, Digital Rain, Kaleidoscope — each
  with two style variants and full tuning.
- **6 new presets (15 total)**: 💻 Matrix Code, 🌊 Deep Ocean,
  🕯 Candlelight, ⛈ Thunderstorm, ✨ Galaxy Dust, 🧬 Double Helix.
- All of it crossfades and slew-limits exactly like before — the
  photosensitive-safety guarantees are untouched.
- **Smarter installer**: upgrades now close a running daemon/configurator by
  themselves (no more locked-file errors) and guarantee the daemon comes
  back afterwards. Every checkbox tells you what's already on the machine —
  "OpenRGB: already installed", "start-with-Windows: already set up from a
  previous install" — and anything already in place starts unticked, so an
  upgrade breezes through in seconds.

## 1.3 — Always Smooth, Seizure-Safe (2026-07-28)
- **One-click installer** (`ArgbProMaster-Setup.exe`): installs the app,
  OpenRGB and MSI Afterburner (winget), configures OpenRGB's start-with-
  Windows scheduled task (admin + SDK server, no login UAC), daemon
  autostart, shortcuts, uninstaller — full A-to-Z setup.
- **Jump-free thermal clocks**: animations accelerate with heat but can
  never teleport under temperature spikes (integrated per-sensor phase
  replaces `time × speed(temp)`).
- **Photosensitive-safety slew limiter**: output stage caps per-LED change
  (full black↔white ≥ 0.35 s in linear light) — strobing is physically
  impossible regardless of upstream behavior.
- Apply & Save now offers to snapshot the whole setup as a named preset.
- README rewritten as a complete install/use/help guide.

## 1.2.1 (unreleased polish)
- **Gamma-correct color pipeline**: all gradient lookups and look-change
  crossfades now blend in linear light instead of raw sRGB — gradient
  midpoints are luminous instead of muddy, and slow transitions read
  perceptually even from end to end.
- "Transition time" slider (0.2–5 s, default 1.5 s) for look-change
  crossfades.
- Stability hardening: Fix My RGB only auto-enables motherboard ARGB (HID)
  zones; GPU (I2C) and RAM (SMBus) zones are strictly opt-in and capped at
  2 updates/sec — slow buses can no longer flood OpenRGB into reconnect
  loops. The daemon self-reports unsustainable frame rates in daemon.log.
- In-app link to the project page and help.

## 1.2 — Deep Customization (2026-07-27)
- Multi-stop Color Journey: up to 8 gradient stops with live preview.
- Idle Effect: own effect, own colors, own speed/intensity, with boundary
  hysteresis (2-unit exit margin, 3 s dwell) — no strobing at range edges.
- New effects: Light Trail (orbiting pulse with Trail length & Resolution),
  Fireworks, Wave Collide — 17 effects total.
- Seamless crossfades on every look change; six component sources per zone
  (CPU/GPU °C, CPU/GPU load, RAM, FPS); tooltips everywhere.

## 1.1 — Component Selector (2026-07-27)
- Zones can follow CPU °C, GPU °C, CPU load, GPU load, RAM use, or FPS.
- Live preview follows real sensors; Thermal Alert became a chase-style wave.

## 1.0 (2026-07-26)
- First release: auto-detected zones for any OpenRGB hardware, 14 effects,
  per-effect tuning, Effect Lab custom-effect builder, presets, live
  pixel-identical preview, tray quick menu, setup assistant (winget
  auto-install of OpenRGB / MSI Afterburner), Fix My RGB, restore original
  lighting, atomic hot-reloaded settings, panic-logging daemon measured at
  0.024% CPU / 4.7 MB RAM.
