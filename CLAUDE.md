# ArgbProMaster — project context

Thermal-reactive ARGB lighting for this specific PC. Rust workspace, Windows-only, **not a git repository** (deliberate — don't init one without asking).

## Hardware map

| Zone | Header | LEDs | Default source |
|---|---|---|---|
| Front fans (3× Teucer strips) | ADD_GEN2_2 → OpenRGB zone "Aura Addressable 2" | 72 | GPU |
| Bottom underglow (4× Teucer) | ADD_GEN2_3 → OpenRGB zone "Aura Addressable 3" | 96 | CPU |
| Corsair Vengeance DDR5 (2×10) | SMBus | 20 | CPU |

Board: ASUS Prime X670-P WiFi · CPU: Ryzen 9 7950X3D · GPU: RTX 5070 Ti (Gigabyte Eagle OC ICE).
The board also exposes "Aura Mainboard" (2 LEDs) and "Aura Addressable 1" (unused) — the daemon deliberately leaves both alone.

## Layout

- `argb_core` — settings schema (`%APPDATA%\ArgbProMaster\settings.json`), preset engine, animation renderer (shared by GUI preview and daemon so they render identically), Win32 single-instance mutex.
- `thermal_daemon` — headless loop: MSI Afterburner shared memory (`MAHMSharedMemory`) → exponential smoothing → `argb_core::engine` frames → OpenRGB SDK TCP (127.0.0.1:6742, protocol capped at 3). Hot-reloads settings.json on mtime change. `examples/probe.rs` (dump controllers/zones/modes) and `examples/paint.rs` (paint headers distinct static colors) are diagnostic tools.
- `configurator_gui` — egui 0.31 dark-themed control panel (tabs: presets/ports/curves/advanced, live preview, tray via tray-icon).
- **Deployed app**: `C:\Users\beta\Desktop\ArgbProMaster\` (configurator_gui.exe + thermal_daemon.exe must stay siblings; GUI spawns the daemon from its own folder). Rebuilds land in `target\release\` — copy over after changes.

## Build

- `rust-toolchain.toml` pins Rust 1.88.0 (global Cargo 1.84 can't parse `edition2024` manifests in the eframe tree). `Cargo.lock` additionally pins `idna_adapter 1.2.0` (→ ICU 1.5.x). Either workaround suffices; leave both.
- `cargo build --release` must stay zero-error; tests: `cargo test --workspace` (argb_core 12, thermal_daemon 7).

## Hard-won lessons (do not relearn these)

1. **OpenRGB SETCUSTOMMODE (1100) does NOT reach the hardware.** Only UPDATEMODE (1101) with the device's "Direct" mode actually switches the board out of its firmware effect — without it, every streamed frame is silently invisible while the onboard rainbow plays. Implemented: `ControllerInfo::direct_mode()` (falls back "Direct"→"Custom"→"Static", trimmed/case-insensitive) + `update_mode()` in `thermal_daemon/src/openrgb.rs`.
2. **Anything (OpenRGB GUI, firmware) can steal the mode back.** The daemon polls `active_mode` every 10 s (`MODE_GUARD_POLL`) and re-asserts Direct — see the mode-guard block in the frame loop.
3. The daemon re-runs device discovery while its map is empty (handles connecting before OpenRGB finishes detection).
4. **Windows process I/O counters do not count socket sends as write-ops** (they're "other" ops). Don't use `WriteOperationCount` to detect streaming.
5. Windows Defender's "ClickFix" heuristic blocks PowerShell one-liners that build raw network bytes (`[byte[]]` + socket writes) — use the Rust `probe`/`paint` examples or `netstat` instead of PS socket scripts.
6. Corsair DDR5 DIMMs only appear in OpenRGB when it runs **elevated**. Confirmed 2026-07-23: even with PawnIO failing to load (`PawnIO module initialization aborted, code=-2147023728` / ERROR_NOT_FOUND), an elevated OpenRGB still detects both DIMMs via the AMD SMBus path — PawnIO is not actually required on this board.
7. **A hidden eframe window can never process viewport commands.** eframe only applies queued `ViewportCommand`s while painting a frame; frames are driven by `RedrawRequested`; winit's Windows `request_redraw()` is `RedrawWindow(RDW_INTERNALPAINT)`, which the OS ignores for hidden windows. So after `Visible(false)`, `Visible(true)`/`Close` sit in the queue forever. Tray restore must call Win32 `ShowWindow` on the raw HWND first, and tray quit must show the window before posting `WM_CLOSE` (eframe latches close requests only inside a frame; a `WM_CLOSE` at a hidden window queues a stale Close that fires on the next restore). See `configurator_gui/src/tray.rs`.
8. **OpenRGB quits entirely when its window is closed** (default `minimize_on_close: false`), taking the SDK server down and freezing the lights on the last frame — the daemon then retries silently forever (its log's last line will be `OpenRGB connection lost — reconnecting`). We set `minimize_on_close: true` in `%APPDATA%\OpenRGB\OpenRGB.json` on 2026-07-23 so closing the window just minimizes it to tray.

## Runtime requirements

OpenRGB (installed via winget, 1.0rc3, `C:\Program Files\OpenRGB\OpenRGB.exe`) must run with SDK server on port 6742. Since 2026-07-23 this is handled by the "OpenRGB Server" scheduled task (elevated at logon, so the Corsair DIMMs are detected); don't also enable OpenRGB's own Start-at-Login (would spawn a second, non-elevated instance). MSI Afterburner supplies temps (daemon degrades gracefully without it). Daemon logs lifecycle events to `%APPDATA%\ArgbProMaster\daemon.log` — read this FIRST when lights misbehave (connects, zone mapping, reloads, external mode-change reclaims, warnings).

## Open items

- RAM lighting: DONE 2026-07-23 — daemon.log shows `ram_sticks=2`. OpenRGB runs elevated via the "OpenRGB Server" scheduled task (at logon, RunLevel Highest, `--server --startminimized`, no execution time limit). `schtasks /Run /TN "OpenRGB Server"` restarts it elevated without a UAC prompt.
- Daemon autostart: DONE 2026-07-23 — "ArgbProMaster Daemon.lnk" in `shell:startup`.
- Mode-guard `controller_data` poll happens on the streaming socket; if OpenRGB is slow it momentarily delays a frame — acceptable at 10 s cadence.
- The daemon's connect-retry loop is silent; after the initial "connection lost" line nothing is logged until reconnect succeeds. A periodic "still waiting for OpenRGB" line would make the dead-server state visible in the log.

## Conventions

- Hand-rolled Win32/protocol code over dependency crates; unit tests live beside the code they test; comments explain constraints, not history.
- A project code-reviewer subagent exists at `.claude/agents/code-reviewer.md` — use it after modifying Rust code here.
- Two Claude agents once worked on this repo concurrently (2026-07-23); if files change unexpectedly mid-session, check for a parallel session before overwriting.
