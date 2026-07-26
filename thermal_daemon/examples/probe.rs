//! Diagnostic: connect to OpenRGB with the daemon's own client code and dump
//! every controller/zone exactly as the daemon would see it.
//! Run: cargo run --release -p thermal_daemon --example probe

use argb_core::openrgb;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

fn main() {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6742);
    let mut c = match openrgb::OpenRgbClient::connect(addr, "ArgbProMaster probe") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("connect failed: {e}");
            std::process::exit(1);
        }
    };
    println!("negotiated protocol: {}", c.protocol);
    let count = match c.controller_count() {
        Ok(n) => n,
        Err(e) => {
            eprintln!("controller_count failed: {e}");
            std::process::exit(1);
        }
    };
    println!("controllers: {count}");
    for dev in 0..count {
        match c.controller_data(dev) {
            Ok(info) => {
                println!(
                    "[{dev}] type={} name={:?} leds={} zones={}",
                    info.controller_type,
                    info.name,
                    info.num_leds,
                    info.zones.len()
                );
                for (i, z) in info.zones.iter().enumerate() {
                    println!(
                        "    zone[{i}] {:?} type={} leds {}..{} count={}",
                        z.name, z.zone_type, z.leds_min, z.leds_max, z.leds_count
                    );
                }
            }
            Err(e) => println!("[{dev}] controller_data FAILED: {e}"),
        }
    }
}
