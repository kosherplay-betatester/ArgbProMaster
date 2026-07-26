//! ArgbProMaster thermal daemon — invisible background renderer.
//!
//! Reads CPU/GPU temperatures from MSI Afterburner shared memory, applies
//! exponential smoothing, renders animation frames with `argb_core::engine`
//! and streams them to OpenRGB over the local SDK socket (port 6742).
//! `settings.json` is re-read whenever its modification timestamp changes.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
mod afterburner;

use argb_core::engine;
use argb_core::openrgb::{self, OpenRgbClient};
use argb_core::settings::Settings;
use argb_core::win::{SingleInstance, DAEMON_MUTEX_NAME};
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

/// Append one line to `%APPDATA%\ArgbProMaster\daemon.log`. The daemon has no
/// console in release builds, so this file is its only observable signal —
/// deliberately tiny: a handful of lifecycle events, self-truncating.
fn note(msg: &str) {
    #[cfg(debug_assertions)]
    eprintln!("[thermal_daemon] {msg}");
    let path = argb_core::settings::settings_dir().join("daemon.log");
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::metadata(&path).map(|m| m.len() > 512 * 1024).unwrap_or(false) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(f, "[{secs}] {msg}");
    }
}

const OPENRGB_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6742);
const RECONNECT_DELAY: Duration = Duration::from_secs(5);
const MAHM_RETRY_DELAY: Duration = Duration::from_secs(5);
/// How often to verify our devices are still in Direct mode.
const MODE_GUARD_POLL: Duration = Duration::from_secs(10);

/// One enabled zone resolved to its live location on the OpenRGB server.
#[derive(Debug)]
struct ResolvedZone {
    device: u32,
    /// -1 = drive the whole device with UPDATELEDS.
    zone_idx: i32,
    leds: u32,
    /// Index into `settings.zones`.
    cfg: usize,
    /// Last frame actually sent — identical frames are skipped entirely, so a
    /// static look costs zero traffic (RTSS-style: do nothing unless needed).
    last_frame: Vec<[u8; 3]>,
}

/// Where the configured zones actually live on the OpenRGB server.
#[derive(Debug, Default)]
struct RenderMap {
    zones: Vec<ResolvedZone>,
    /// (device, direct mode index) for every device we drive, so the frame
    /// loop can detect when the OpenRGB GUI (or firmware) steals the mode
    /// back and quietly re-assert Direct.
    direct_modes: Vec<(u32, u32)>,
}

impl RenderMap {
    fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }
}

fn main() {
    // Refuse to double-render: a second instance exits silently.
    let Some(_guard) = SingleInstance::acquire(DAEMON_MUTEX_NAME) else {
        return;
    };
    // A crash must never be silent: log the panic before dying, so
    // daemon.log always tells crash apart from external kill.
    std::panic::set_hook(Box::new(|info| {
        note(&format!("PANIC: {info}"));
    }));
    note("daemon starting");

    let mut settings = Settings::load_or_create();
    let mut settings_stamp = argb_core::settings::settings_mtime();

    let mut mahm = MahmSource::new();
    // Report the thermal sources once, so the log shows what was detected on
    // this machine (works with any CPU/GPU vendor Afterburner supports).
    let mut thermal_reported = false;

    // Smoothed "working" temperatures, seeded at the cool end of the curves.
    let mut cpu_work = settings.cpu_temp_min;
    let mut gpu_work = settings.gpu_temp_min;
    let mut cpu_target = cpu_work;
    let mut gpu_target = gpu_work;

    let animation_start = Instant::now();

    loop {
        // ---- connect to OpenRGB (retry forever; server may not be up yet) --
        let mut client = match OpenRgbClient::connect(OPENRGB_ADDR, "ArgbProMaster") {
            Ok(c) => c,
            Err(_) => {
                std::thread::sleep(RECONNECT_DELAY);
                continue;
            }
        };
        let mut map = match discover(&mut client, &settings) {
            Ok(m) => m,
            Err(_) => {
                std::thread::sleep(RECONNECT_DELAY);
                continue;
            }
        };
        note(&format!(
            "connected to OpenRGB (protocol {}): {} zone(s) live",
            client.protocol,
            map.zones.len()
        ));

        // ---- frame loop ---------------------------------------------------
        let mut last_discover = Instant::now();
        let mut last_mode_guard = Instant::now();
        'frames: loop {
            let frame_start = Instant::now();

            // Anyone (the OpenRGB GUI, another SDK client) can flip a device
            // out of Direct mode behind our back — the stream then becomes
            // invisible while every write still succeeds. Poll and reclaim.
            if !map.direct_modes.is_empty() && last_mode_guard.elapsed() >= MODE_GUARD_POLL {
                last_mode_guard = Instant::now();
                let mut lost_connection = false;
                for entry in map.direct_modes.iter_mut() {
                    let (device, expected) = *entry;
                    match client.controller_data(device) {
                        Ok(info) if info.active_mode != expected as i32 => {
                            if let Some((idx, mode)) = info.direct_mode() {
                                let mode = mode.clone();
                                if client.update_mode(device, idx, &mode).is_err() {
                                    lost_connection = true;
                                    break;
                                }
                                *entry = (device, idx);
                                note(&format!(
                                    "device {device} was switched to mode {} externally — re-asserted Direct",
                                    info.active_mode
                                ));
                            }
                        }
                        Ok(_) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {}
                        Err(_) => {
                            lost_connection = true;
                            break;
                        }
                    }
                }
                if lost_connection {
                    note("OpenRGB connection lost — reconnecting");
                    break 'frames;
                }
            }

            // If we connected while the server was still detecting hardware
            // (e.g. OpenRGB launched after us), the first discovery comes back
            // empty — and the server's DEVICE_LIST_UPDATED push is only seen
            // during request/response traffic, so keep re-scanning until the
            // devices appear.
            if map.is_empty() && last_discover.elapsed() >= RECONNECT_DELAY {
                last_discover = Instant::now();
                match discover(&mut client, &settings) {
                    Ok(m) => map = m,
                    Err(_) => break 'frames,
                }
            }

            // Hot-reload settings on mtime change. A torn or unreadable file
            // (mid-write) keeps the current settings instead of nuking them
            // to defaults — the next mtime change retries.
            let stamp = argb_core::settings::settings_mtime();
            if stamp != settings_stamp {
                settings_stamp = stamp;
                let parsed = std::fs::read_to_string(argb_core::settings::settings_path())
                    .ok()
                    .and_then(|text| Settings::from_json(&text));
                match parsed {
                    Some(fresh) => {
                        let remap = fresh.zones != settings.zones;
                        settings = fresh;
                        note(&format!("settings reloaded (preset \"{}\")", settings.active_preset));
                        if remap {
                            match discover(&mut client, &settings) {
                                Ok(m) => map = m,
                                Err(_) => break 'frames,
                            }
                        }
                    }
                    None => note("settings.json unreadable (mid-write?) — keeping current settings"),
                }
            }
            if client.device_list_dirty {
                client.device_list_dirty = false;
                match discover(&mut client, &settings) {
                    Ok(m) => map = m,
                    Err(_) => break 'frames,
                }
            }

            // Pull fresh thermal targets when Afterburner is available.
            if let Some(temps) = mahm.read() {
                if !thermal_reported {
                    thermal_reported = true;
                    let show = |t: Option<f32>| match t {
                        Some(v) => format!("found ({v:.0}\u{00B0}C)"),
                        None => "not found".to_string(),
                    };
                    note(&format!(
                        "thermal sources detected via MSI Afterburner: CPU {} / GPU {}",
                        show(temps.cpu),
                        show(temps.gpu)
                    ));
                }
                if let Some(c) = temps.cpu {
                    cpu_target = c;
                }
                if let Some(g) = temps.gpu {
                    gpu_target = g;
                }
            }

            // Exponential smoothing for fluid transitions.
            cpu_work += (cpu_target - cpu_work) * settings.smoothing_speed;
            gpu_work += (gpu_target - gpu_work) * settings.smoothing_speed;

            let cpu_norm = engine::normalize_temp(cpu_work, settings.cpu_temp_min, settings.cpu_temp_max);
            let gpu_norm = engine::normalize_temp(gpu_work, settings.gpu_temp_min, settings.gpu_temp_max);
            let time = animation_start.elapsed().as_secs_f64();

            if send_frame(&mut client, &mut map, &settings, time, cpu_norm, gpu_norm, cpu_work, gpu_work).is_err() {
                note("OpenRGB connection lost — reconnecting");
                break 'frames;
            }

            let frame_time = Duration::from_secs_f32(1.0 / settings.animation_fps.max(1) as f32);
            if let Some(rest) = frame_time.checked_sub(frame_start.elapsed()) {
                std::thread::sleep(rest);
            }
        }

        std::thread::sleep(Duration::from_secs(2));
    }
}

// ---------------------------------------------------------------------------
// Thermal source with automatic reopen
// ---------------------------------------------------------------------------

struct MahmSource {
    reader: Option<afterburner::MahmReader>,
    last_attempt: Option<Instant>,
    stale_reads: u32,
}

impl MahmSource {
    fn new() -> MahmSource {
        MahmSource {
            reader: afterburner::MahmReader::open(),
            last_attempt: Some(Instant::now()),
            stale_reads: 0,
        }
    }

    fn read(&mut self) -> Option<afterburner::Temps> {
        if self.reader.is_none() {
            let due = self
                .last_attempt
                .map(|t| t.elapsed() >= MAHM_RETRY_DELAY)
                .unwrap_or(true);
            if due {
                self.last_attempt = Some(Instant::now());
                self.reader = afterburner::MahmReader::open();
            }
        }
        let reader = self.reader.as_ref()?;
        match reader.read_temps() {
            Some(t) => {
                self.stale_reads = 0;
                Some(t)
            }
            None => {
                // Repeatedly-stale memory usually means Afterburner exited;
                // drop the mapping so we periodically try a clean reopen.
                self.stale_reads += 1;
                if self.stale_reads > 50 {
                    self.stale_reads = 0;
                    self.reader = None;
                    self.last_attempt = Some(Instant::now());
                }
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Device discovery / zone mapping
// ---------------------------------------------------------------------------

fn discover(client: &mut OpenRgbClient, settings: &Settings) -> std::io::Result<RenderMap> {
    let detected = argb_core::zones::detect(client)?;
    let mut map = RenderMap::default();
    let mut claimed: Vec<bool> = vec![false; detected.len()];

    for (cfg_idx, cfg) in settings.zones.iter().enumerate() {
        if !cfg.enabled {
            continue;
        }
        for (det_idx, det) in detected.iter().enumerate() {
            if claimed[det_idx] || !argb_core::zones::matches(cfg, det) {
                continue;
            }
            claimed[det_idx] = true;

            // Resize resizable headers to the user's LED count so empty ports
            // ("0 LEDs") light up once a count is configured.
            let wanted = cfg.effective_leds();
            let mut leds = det.leds;
            if wanted > 0 && det.resizable && wanted != det.leds && det.zone_idx >= 0 {
                let fits = det.max_leds == 0 || wanted <= det.max_leds;
                if fits {
                    client.resize_zone(det.device_idx, det.zone_idx as u32, wanted)?;
                    leds = wanted;
                }
            } else if wanted > 0 && !det.resizable {
                leds = wanted.min(det.leds.max(1));
            }
            if leds == 0 {
                continue; // nothing to render until the user sets a LED count
            }
            map.zones.push(ResolvedZone {
                device: det.device_idx,
                zone_idx: det.zone_idx,
                leds,
                cfg: cfg_idx,
                last_frame: Vec::new(),
            });
        }
    }

    // Must be a real UPDATEMODE to "Direct": firmware keeps playing its
    // onboard effect over anything we stream until the switch reaches the
    // hardware. One switch per involved device.
    let mut devices: Vec<u32> = map.zones.iter().map(|z| z.device).collect();
    devices.sort_unstable();
    devices.dedup();
    for device in devices {
        let info = match client.controller_data(device) {
            Ok(i) => i,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => continue,
            Err(e) => return Err(e),
        };
        switch_to_direct(client, device, &info, &mut map)?;
    }

    Ok(map)
}

/// Push a device into Direct mode and remember which mode index that is, so
/// the frame loop can detect and undo outside mode changes.
fn switch_to_direct(
    client: &mut OpenRgbClient,
    device: u32,
    info: &openrgb::ControllerInfo,
    map: &mut RenderMap,
) -> std::io::Result<()> {
    match info.direct_mode() {
        Some((idx, mode)) => {
            let mode = mode.clone();
            client.update_mode(device, idx, &mode)?;
            map.direct_modes.push((device, idx));
        }
        None => {
            client.set_custom_mode(device)?;
            note(&format!(
                "WARN: \"{}\" has no Direct/Custom/Static mode — streamed colors may stay invisible",
                info.name
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Frame rendering
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn send_frame(
    client: &mut OpenRgbClient,
    map: &mut RenderMap,
    settings: &Settings,
    time: f64,
    cpu_norm: f32,
    gpu_norm: f32,
    cpu_c: f32,
    gpu_c: f32,
) -> std::io::Result<()> {
    for zone in map.zones.iter_mut() {
        let Some(cfg) = settings.zones.get(zone.cfg) else {
            continue; // settings shrank since resolve; remap follows shortly
        };
        let frame = engine::render_zone_config(
            settings, cfg, zone.leds as usize, time, cpu_norm, gpu_norm, cpu_c, gpu_c,
        );
        if frame == zone.last_frame {
            continue; // nothing changed — skip the write entirely
        }
        if zone.zone_idx >= 0 {
            client.update_zone(zone.device, zone.zone_idx as u32, &frame)?;
        } else {
            client.update_leds(zone.device, &frame)?;
        }
        zone.last_frame = frame;
    }
    Ok(())
}

// Keep unused-import warnings away on non-Windows check builds where the
// afterburner module is compiled out.
#[cfg(not(windows))]
mod afterburner {
    #[derive(Clone, Copy, Debug, Default)]
    pub struct Temps {
        pub cpu: Option<f32>,
        pub gpu: Option<f32>,
    }
    pub struct MahmReader;
    impl MahmReader {
        pub fn open() -> Option<MahmReader> {
            None
        }
        pub fn read_temps(&self) -> Option<Temps> {
            None
        }
    }
}
