//! Zone discovery and matching: turns whatever OpenRGB exposes — motherboard
//! ARGB headers (active or empty), RAM sticks, GPU zones, strips, keyboards —
//! into a flat list the user can assign effects to, and keeps the user's saved
//! [`ZoneConfig`] list in sync with what is actually plugged in.

use crate::openrgb::{ControllerInfo, OpenRgbClient};
use crate::settings::{TargetSource, ZoneConfig};

pub const DEVICE_TYPE_MOTHERBOARD: i32 = 0;
pub const DEVICE_TYPE_DRAM: i32 = 1;

/// One controllable zone as reported by the OpenRGB server right now.
#[derive(Clone, Debug, PartialEq)]
pub struct DetectedZone {
    pub device_idx: u32,
    pub device_name: String,
    pub device_type: i32,
    /// Empty = the whole device is driven at once (devices without zones).
    pub zone_name: String,
    /// -1 for whole-device entries.
    pub zone_idx: i32,
    pub leds: u32,
    pub resizable: bool,
    pub max_leds: u32,
}

impl DetectedZone {
    /// Human-friendly default label: "Aura Addressable 2" or the device name.
    pub fn friendly_name(&self) -> String {
        if self.zone_name.is_empty() {
            self.device_name.clone()
        } else {
            self.zone_name.clone()
        }
    }
}

fn zones_of(device_idx: u32, info: &ControllerInfo) -> Vec<DetectedZone> {
    if info.zones.is_empty() {
        return vec![DetectedZone {
            device_idx,
            device_name: info.name.clone(),
            device_type: info.controller_type,
            zone_name: String::new(),
            zone_idx: -1,
            leds: info.num_leds,
            resizable: false,
            max_leds: info.num_leds,
        }];
    }
    info.zones
        .iter()
        .enumerate()
        .map(|(i, z)| DetectedZone {
            device_idx,
            device_name: info.name.clone(),
            device_type: info.controller_type,
            zone_name: z.name.trim().to_string(),
            zone_idx: i as i32,
            leds: z.leds_count,
            resizable: z.leds_min != z.leds_max,
            max_leds: z.leds_max,
        })
        .collect()
}

/// Enumerate every zone of every device on the server — including empty,
/// resizable ARGB headers reporting 0 LEDs (shown so the user can activate
/// them by setting a LED count).
pub fn detect(client: &mut OpenRgbClient) -> std::io::Result<Vec<DetectedZone>> {
    let count = client.controller_count()?;
    let mut out = Vec::new();
    for device in 0..count {
        match client.controller_data(device) {
            Ok(info) => out.extend(zones_of(device, &info)),
            // A device disappearing mid-enumeration is not fatal.
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}

/// Does a saved config entry refer to this detected zone?
///
/// Besides exact (device name, zone name) identity there are two legacy
/// wildcard forms produced by the v1 settings migration:
/// * `zone_name = "*2"` — any motherboard zone whose name ends in "2"
/// * empty device and zone names with `device_type = DRAM` — every RAM stick
pub fn matches(cfg: &ZoneConfig, det: &DetectedZone) -> bool {
    if !cfg.device_name.is_empty() {
        return cfg.device_name == det.device_name && cfg.zone_name == det.zone_name;
    }
    if let Some(suffix) = cfg.zone_name.strip_prefix('*') {
        return det.device_type == DEVICE_TYPE_MOTHERBOARD && det.zone_name.ends_with(suffix);
    }
    cfg.device_type == DEVICE_TYPE_DRAM && det.device_type == DEVICE_TYPE_DRAM
}

fn concrete_from(cfg: &ZoneConfig, det: &DetectedZone) -> ZoneConfig {
    ZoneConfig {
        device_name: det.device_name.clone(),
        device_type: det.device_type,
        zone_name: det.zone_name.clone(),
        display_name: if cfg.display_name.is_empty() {
            det.friendly_name()
        } else {
            cfg.display_name.clone()
        },
        last_seen_leds: det.leds,
        resizable: det.resizable,
        max_leds: det.max_leds,
        ..cfg.clone()
    }
}

/// Reconcile the saved zone list with a fresh detection:
/// * legacy wildcard entries become concrete per-zone entries (keeping their
///   enabled state, targets and overrides)
/// * zones never seen before are appended, disabled, ready to switch on
/// * detection metadata (LED counts, resizability) is refreshed
/// * entries whose hardware is currently absent are kept, never deleted
pub fn merge(zone_configs: &mut Vec<ZoneConfig>, detected: &[DetectedZone]) {
    let mut next: Vec<ZoneConfig> = Vec::with_capacity(zone_configs.len() + detected.len());

    for cfg in zone_configs.iter() {
        let hits: Vec<&DetectedZone> = detected.iter().filter(|d| matches(cfg, d)).collect();
        if hits.is_empty() {
            next.push(cfg.clone()); // hardware absent right now — keep as saved
        } else {
            for det in hits {
                let concrete = concrete_from(cfg, det);
                if !next
                    .iter()
                    .any(|c| c.device_name == concrete.device_name && c.zone_name == concrete.zone_name)
                {
                    next.push(concrete);
                }
            }
        }
    }

    for det in detected {
        if !next
            .iter()
            .any(|c| c.device_name == det.device_name && c.zone_name == det.zone_name)
        {
            next.push(ZoneConfig {
                device_name: det.device_name.clone(),
                device_type: det.device_type,
                zone_name: det.zone_name.clone(),
                display_name: det.friendly_name(),
                enabled: false,
                led_count: 0,
                target_source: TargetSource::Cpu,
                effect_override: None,
                custom_effect: None,
                colors_override: None,
                last_seen_leds: det.leds,
                resizable: det.resizable,
                max_leds: det.max_leds,
            });
        }
    }

    *zone_configs = next;
}

/// Emoji for a device type, used by the GUI to make the zone list scannable.
pub fn device_emoji(device_type: i32) -> &'static str {
    match device_type {
        0 => "🔌", // motherboard ARGB headers
        1 => "🧠", // DRAM
        2 => "🎮", // GPU
        3 => "❄",  // cooler
        4 => "💡", // LED strip
        5 => "⌨",  // keyboard
        6 => "🖱", // mouse
        8 => "🎧", // headset
        _ => "💡",
    }
}

/// Plain-language description of a device type for tooltips.
pub fn device_kind_label(device_type: i32) -> &'static str {
    match device_type {
        0 => "Motherboard ARGB header",
        1 => "RAM stick",
        2 => "Graphics card",
        3 => "CPU cooler",
        4 => "LED strip",
        5 => "Keyboard",
        6 => "Mouse",
        8 => "Headset",
        _ => "RGB device",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn det(device: &str, dtype: i32, zone: &str, leds: u32) -> DetectedZone {
        DetectedZone {
            device_idx: 0,
            device_name: device.to_string(),
            device_type: dtype,
            zone_name: zone.to_string(),
            zone_idx: 0,
            leds,
            resizable: dtype == DEVICE_TYPE_MOTHERBOARD,
            max_leds: 120,
        }
    }

    fn legacy_port(suffix: &str, enabled: bool, leds: u32) -> ZoneConfig {
        ZoneConfig {
            zone_name: format!("*{suffix}"),
            enabled,
            led_count: leds,
            ..ZoneConfig::default()
        }
    }

    #[test]
    fn legacy_wildcards_concretize_and_keep_settings() {
        let mut cfgs = vec![legacy_port("2", true, 72)];
        let detected = vec![
            det("ASUS PRIME X670-P WIFI", 0, "Aura Addressable 1", 0),
            det("ASUS PRIME X670-P WIFI", 0, "Aura Addressable 2", 72),
        ];
        merge(&mut cfgs, &detected);
        let hit = cfgs
            .iter()
            .find(|c| c.zone_name == "Aura Addressable 2")
            .expect("wildcard resolved");
        assert!(hit.enabled);
        assert_eq!(hit.led_count, 72);
        assert_eq!(hit.device_name, "ASUS PRIME X670-P WIFI");
        // The empty header shows up too, disabled and ready to activate.
        let empty = cfgs
            .iter()
            .find(|c| c.zone_name == "Aura Addressable 1")
            .expect("inactive header listed");
        assert!(!empty.enabled);
        assert_eq!(empty.last_seen_leds, 0);
        assert!(empty.resizable);
    }

    #[test]
    fn dram_wildcard_expands_to_every_stick() {
        let mut cfgs = vec![ZoneConfig {
            device_type: DEVICE_TYPE_DRAM,
            enabled: true,
            ..ZoneConfig::default()
        }];
        let detected = vec![
            det("Corsair Vengeance RGB DDR5", 1, "DRAM", 10),
            det("Corsair Vengeance RGB DDR5 #2", 1, "DRAM", 10),
        ];
        merge(&mut cfgs, &detected);
        assert_eq!(cfgs.iter().filter(|c| c.device_type == 1 && c.enabled).count(), 2);
    }

    #[test]
    fn absent_hardware_is_kept_not_deleted() {
        let mut cfgs = vec![ZoneConfig {
            device_name: "Unplugged Keyboard".into(),
            zone_name: "Keys".into(),
            enabled: true,
            ..ZoneConfig::default()
        }];
        merge(&mut cfgs, &[det("GPU", 2, "GPU zone", 12)]);
        assert!(cfgs.iter().any(|c| c.device_name == "Unplugged Keyboard" && c.enabled));
        assert!(cfgs.iter().any(|c| c.device_name == "GPU" && !c.enabled));
    }

    #[test]
    fn repeated_merge_is_stable() {
        let detected = vec![det("Board", 0, "Header 1", 30), det("GPU", 2, "GPU zone", 12)];
        let mut cfgs = Vec::new();
        merge(&mut cfgs, &detected);
        let first = cfgs.clone();
        merge(&mut cfgs, &detected);
        assert_eq!(first, cfgs);
    }
}
