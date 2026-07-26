//! Diagnostic: paint every motherboard ARGB zone a distinct solid color and
//! leave it applied, so the user can report which physical strips lit up.
//!
//!   Aura Mainboard     -> WHITE
//!   Aura Addressable 1 -> GREEN (temporarily resized to 72)
//!   Aura Addressable 2 -> RED
//!   Aura Addressable 3 -> BLUE
//!
//! Run: cargo run --release -p thermal_daemon --example paint

use argb_core::openrgb;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn main() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6742);
    let mut c = openrgb::OpenRgbClient::connect(addr, "ArgbProMaster paint test").expect("connect");
    println!("protocol {}", c.protocol);

    let count = c.controller_count().expect("count");
    for dev in 0..count {
        let info = match c.controller_data(dev) {
            Ok(i) => i,
            Err(e) => {
                println!("[{dev}] data failed: {e}");
                continue;
            }
        };
        if info.controller_type != openrgb::DEVICE_TYPE_MOTHERBOARD {
            continue;
        }
        println!("[{dev}] motherboard: {}", info.name);
        let mode_names: Vec<&str> = info.modes.iter().map(|m| m.name.as_str()).collect();
        println!("  modes: {mode_names:?}");
        let direct = c.enter_direct_mode(dev, &info).expect("mode switch");
        println!(
            "  mode switch: {}",
            if direct { "UPDATEMODE -> Direct (reaches hardware)" } else { "SETCUSTOMMODE fallback (no Direct mode!)" }
        );

        for (zi, z) in info.zones.iter().enumerate() {
            let (color, leds): ([u8; 3], u32) = match z.name.as_str() {
                "Aura Mainboard" => ([255, 255, 255], z.leds_count),
                "Aura Addressable 1" => ([0, 255, 0], 72),
                "Aura Addressable 2" => ([255, 0, 0], z.leds_count.max(72)),
                "Aura Addressable 3" => ([0, 0, 255], z.leds_count.max(96)),
                _ => continue,
            };
            if z.leds_count != leds {
                println!("  resizing zone {zi} '{}' {} -> {leds}", z.name, z.leds_count);
                c.resize_zone(dev, zi as u32, leds).expect("resize");
            }
            let frame = vec![color; leds as usize];
            c.update_zone(dev, zi as u32, &frame).expect("update");
            println!("  zone {zi} '{}' -> RGB {:?} on {leds} LEDs", z.name, color);
        }
    }
    // Give the server a moment to flush everything to the hardware.
    std::thread::sleep(std::time::Duration::from_millis(1500));
    println!("done - colors left applied");
}
