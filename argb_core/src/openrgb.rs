//! Minimal synchronous OpenRGB SDK network client (default port 6742).
//!
//! Wire format: every packet is a 16-byte header
//! `"ORGB" | device_index: u32 LE | packet_id: u32 LE | data_len: u32 LE`
//! followed by `data_len` payload bytes.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

const MAGIC: [u8; 4] = *b"ORGB";

// Packet ids (NetworkProtocol.h)
const REQUEST_CONTROLLER_COUNT: u32 = 0;
const REQUEST_CONTROLLER_DATA: u32 = 1;
const REQUEST_PROTOCOL_VERSION: u32 = 40;
const SET_CLIENT_NAME: u32 = 50;
const DEVICE_LIST_UPDATED: u32 = 100;
const RGBCONTROLLER_RESIZEZONE: u32 = 1000;
const RGBCONTROLLER_UPDATELEDS: u32 = 1050;
const RGBCONTROLLER_UPDATEZONELEDS: u32 = 1051;
const RGBCONTROLLER_SETCUSTOMMODE: u32 = 1100;
const RGBCONTROLLER_UPDATEMODE: u32 = 1101;

/// Highest protocol revision this client speaks. Capped at 3 to keep the
/// controller-data parser free of the segment blocks added in protocol 4.
const CLIENT_PROTOCOL: u32 = 3;

pub const DEVICE_TYPE_MOTHERBOARD: i32 = 0;
pub const DEVICE_TYPE_DRAM: i32 = 1;
pub const ZONE_TYPE_LINEAR: i32 = 1;

#[derive(Clone, Debug)]
pub struct ZoneInfo {
    pub name: String,
    pub zone_type: i32,
    pub leds_min: u32,
    pub leds_max: u32,
    pub leds_count: u32,
}

/// One device mode, kept verbatim so it can be echoed back in UPDATEMODE.
/// Merely SETCUSTOMMODE-ing only flips the server's in-memory active mode —
/// the hardware keeps playing its onboard effect until UPDATEMODE pushes the
/// switch down to the device.
#[derive(Clone, Debug, Default)]
pub struct ModeInfo {
    pub name: String,
    pub value: i32,
    pub flags: u32,
    pub speed_min: u32,
    pub speed_max: u32,
    pub brightness_min: u32,
    pub brightness_max: u32,
    pub colors_min: u32,
    pub colors_max: u32,
    pub speed: u32,
    pub brightness: u32,
    pub direction: u32,
    pub color_mode: u32,
    pub colors: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct ControllerInfo {
    pub controller_type: i32,
    /// Parsed for diagnostics and tests; device matching keys off the type.
    #[allow(dead_code)]
    pub name: String,
    /// Index of the mode the device is currently in. Anything (the OpenRGB
    /// GUI, another SDK client) can change this behind our back.
    pub active_mode: i32,
    pub modes: Vec<ModeInfo>,
    pub zones: Vec<ZoneInfo>,
    pub num_leds: u32,
}

impl ControllerInfo {
    /// Index and description of the device's direct-control mode. Mirrors the
    /// server's own fallback order — "Direct", then "Custom", then "Static" —
    /// with trimmed, case-insensitive matching.
    pub fn direct_mode(&self) -> Option<(u32, &ModeInfo)> {
        for wanted in ["Direct", "Custom", "Static"] {
            let hit = self
                .modes
                .iter()
                .enumerate()
                .find(|(_, m)| m.name.trim().eq_ignore_ascii_case(wanted));
            if let Some((i, m)) = hit {
                return Some((i as u32, m));
            }
        }
        None
    }
}

pub fn encode_header(device: u32, packet_id: u32, len: u32) -> [u8; 16] {
    let mut header = [0u8; 16];
    header[0..4].copy_from_slice(&MAGIC);
    header[4..8].copy_from_slice(&device.to_le_bytes());
    header[8..12].copy_from_slice(&packet_id.to_le_bytes());
    header[12..16].copy_from_slice(&len.to_le_bytes());
    header
}

/// Payload for UPDATEZONELEDS: total_size | zone | count | RGB0 colors.
pub fn encode_zone_update(zone: u32, colors: &[[u8; 3]]) -> Vec<u8> {
    let total = 4 + 4 + 2 + colors.len() * 4;
    let mut buf = Vec::with_capacity(total);
    buf.extend_from_slice(&(total as u32).to_le_bytes());
    buf.extend_from_slice(&zone.to_le_bytes());
    buf.extend_from_slice(&(colors.len() as u16).to_le_bytes());
    for c in colors {
        buf.extend_from_slice(&[c[0], c[1], c[2], 0]);
    }
    buf
}

/// Payload for UPDATEMODE: total_size | mode_index | full mode description.
/// The description mirrors the wire layout parsed from controller data, so a
/// parsed [`ModeInfo`] can be echoed back verbatim to activate that mode on
/// the physical device.
pub fn encode_update_mode(protocol: u32, mode_index: u32, m: &ModeInfo) -> Vec<u8> {
    let mut buf = Vec::with_capacity(96 + m.name.len() + m.colors.len() * 4);
    buf.extend_from_slice(&0u32.to_le_bytes()); // total size, patched below
    buf.extend_from_slice(&(mode_index as i32).to_le_bytes());
    buf.extend_from_slice(&((m.name.len() + 1) as u16).to_le_bytes());
    buf.extend_from_slice(m.name.as_bytes());
    buf.push(0);
    buf.extend_from_slice(&m.value.to_le_bytes());
    buf.extend_from_slice(&m.flags.to_le_bytes());
    buf.extend_from_slice(&m.speed_min.to_le_bytes());
    buf.extend_from_slice(&m.speed_max.to_le_bytes());
    if protocol >= 3 {
        buf.extend_from_slice(&m.brightness_min.to_le_bytes());
        buf.extend_from_slice(&m.brightness_max.to_le_bytes());
    }
    buf.extend_from_slice(&m.colors_min.to_le_bytes());
    buf.extend_from_slice(&m.colors_max.to_le_bytes());
    buf.extend_from_slice(&m.speed.to_le_bytes());
    if protocol >= 3 {
        buf.extend_from_slice(&m.brightness.to_le_bytes());
    }
    buf.extend_from_slice(&m.direction.to_le_bytes());
    buf.extend_from_slice(&m.color_mode.to_le_bytes());
    buf.extend_from_slice(&(m.colors.len() as u16).to_le_bytes());
    for c in &m.colors {
        buf.extend_from_slice(&c.to_le_bytes());
    }
    let total = buf.len() as u32;
    buf[0..4].copy_from_slice(&total.to_le_bytes());
    buf
}

/// Payload for UPDATELEDS: total_size | count | RGB0 colors.
pub fn encode_led_update(colors: &[[u8; 3]]) -> Vec<u8> {
    let total = 4 + 2 + colors.len() * 4;
    let mut buf = Vec::with_capacity(total);
    buf.extend_from_slice(&(total as u32).to_le_bytes());
    buf.extend_from_slice(&(colors.len() as u16).to_le_bytes());
    for c in colors {
        buf.extend_from_slice(&[c[0], c[1], c[2], 0]);
    }
    buf
}

pub struct OpenRgbClient {
    stream: TcpStream,
    pub protocol: u32,
    /// Set when the server pushes DEVICE_LIST_UPDATED between replies.
    pub device_list_dirty: bool,
}

impl OpenRgbClient {
    pub fn connect(addr: SocketAddr, client_name: &str) -> std::io::Result<OpenRgbClient> {
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let mut client = OpenRgbClient {
            stream,
            protocol: 0,
            device_list_dirty: false,
        };

        // Negotiate protocol version; very old servers never answer, so a
        // timeout simply leaves us on protocol 0.
        client.send(0, REQUEST_PROTOCOL_VERSION, &CLIENT_PROTOCOL.to_le_bytes())?;
        if let Ok((_, payload)) = client.recv_expect(REQUEST_PROTOCOL_VERSION) {
            if payload.len() >= 4 {
                let server = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                client.protocol = server.min(CLIENT_PROTOCOL);
            }
        }

        let mut name = client_name.as_bytes().to_vec();
        name.push(0);
        client.send(0, SET_CLIENT_NAME, &name)?;
        Ok(client)
    }

    fn send(&mut self, device: u32, packet_id: u32, data: &[u8]) -> std::io::Result<()> {
        self.stream
            .write_all(&encode_header(device, packet_id, data.len() as u32))?;
        if !data.is_empty() {
            self.stream.write_all(data)?;
        }
        Ok(())
    }

    fn recv(&mut self) -> std::io::Result<(u32, u32, Vec<u8>)> {
        let mut header = [0u8; 16];
        self.stream.read_exact(&mut header)?;
        if header[0..4] != MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "bad OpenRGB packet magic",
            ));
        }
        let device = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);
        let packet_id = u32::from_le_bytes([header[8], header[9], header[10], header[11]]);
        let len = u32::from_le_bytes([header[12], header[13], header[14], header[15]]) as usize;
        if len > 16 * 1024 * 1024 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "OpenRGB payload unreasonably large",
            ));
        }
        let mut payload = vec![0u8; len];
        self.stream.read_exact(&mut payload)?;
        Ok((device, packet_id, payload))
    }

    /// Receive until a packet with the wanted id arrives, absorbing any
    /// asynchronous DEVICE_LIST_UPDATED notifications along the way.
    fn recv_expect(&mut self, wanted: u32) -> std::io::Result<(u32, Vec<u8>)> {
        loop {
            let (device, packet_id, payload) = self.recv()?;
            if packet_id == wanted {
                return Ok((device, payload));
            }
            if packet_id == DEVICE_LIST_UPDATED {
                self.device_list_dirty = true;
            }
        }
    }

    pub fn controller_count(&mut self) -> std::io::Result<u32> {
        self.send(0, REQUEST_CONTROLLER_COUNT, &[])?;
        let (_, payload) = self.recv_expect(REQUEST_CONTROLLER_COUNT)?;
        if payload.len() < 4 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "short controller count reply",
            ));
        }
        Ok(u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]))
    }

    pub fn controller_data(&mut self, device: u32) -> std::io::Result<ControllerInfo> {
        if self.protocol >= 1 {
            self.send(device, REQUEST_CONTROLLER_DATA, &self.protocol.to_le_bytes())?;
        } else {
            self.send(device, REQUEST_CONTROLLER_DATA, &[])?;
        }
        let (_, payload) = self.recv_expect(REQUEST_CONTROLLER_DATA)?;
        parse_controller_data(&payload, self.protocol)
    }

    pub fn set_custom_mode(&mut self, device: u32) -> std::io::Result<()> {
        self.send(device, RGBCONTROLLER_SETCUSTOMMODE, &[])
    }

    /// Activate a mode ON THE HARDWARE. Prefer this (with the device's
    /// "Direct" mode) over [`set_custom_mode`], which never reaches the
    /// device — firmware effects keep playing over anything we stream.
    pub fn update_mode(&mut self, device: u32, mode_index: u32, mode: &ModeInfo) -> std::io::Result<()> {
        let payload = encode_update_mode(self.protocol, mode_index, mode);
        self.send(device, RGBCONTROLLER_UPDATEMODE, &payload)
    }

    /// Switch a device into Direct mode if it has one, else fall back to the
    /// custom-mode request. Returns true when a real hardware switch was sent.
    pub fn enter_direct_mode(&mut self, device: u32, info: &ControllerInfo) -> std::io::Result<bool> {
        match info.direct_mode() {
            Some((index, mode)) => {
                let mode = mode.clone();
                self.update_mode(device, index, &mode)?;
                Ok(true)
            }
            None => {
                self.set_custom_mode(device)?;
                Ok(false)
            }
        }
    }

    pub fn resize_zone(&mut self, device: u32, zone: u32, new_size: u32) -> std::io::Result<()> {
        let mut data = Vec::with_capacity(8);
        data.extend_from_slice(&(zone as i32).to_le_bytes());
        data.extend_from_slice(&(new_size as i32).to_le_bytes());
        self.send(device, RGBCONTROLLER_RESIZEZONE, &data)
    }

    pub fn update_zone(&mut self, device: u32, zone: u32, colors: &[[u8; 3]]) -> std::io::Result<()> {
        self.send(device, RGBCONTROLLER_UPDATEZONELEDS, &encode_zone_update(zone, colors))
    }

    pub fn update_leds(&mut self, device: u32, colors: &[[u8; 3]]) -> std::io::Result<()> {
        self.send(device, RGBCONTROLLER_UPDATELEDS, &encode_led_update(colors))
    }
}

// ---------------------------------------------------------------------------
// Controller data parsing
// ---------------------------------------------------------------------------

struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ByteReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        ByteReader { data, pos: 0 }
    }

    fn take(&mut self, n: usize) -> std::io::Result<&'a [u8]> {
        if self.pos + n > self.data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "truncated controller data",
            ));
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u16(&mut self) -> std::io::Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> std::io::Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i32(&mut self) -> std::io::Result<i32> {
        Ok(self.u32()? as i32)
    }

    /// Length-prefixed string (u16 length includes the trailing NUL).
    fn string(&mut self) -> std::io::Result<String> {
        let len = self.u16()? as usize;
        let bytes = self.take(len)?;
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        Ok(String::from_utf8_lossy(&bytes[..end]).to_string())
    }
}

fn parse_controller_data(payload: &[u8], protocol: u32) -> std::io::Result<ControllerInfo> {
    let mut r = ByteReader::new(payload);
    let _data_size = r.u32()?;
    let controller_type = r.i32()?;
    let name = r.string()?;
    if protocol >= 1 {
        let _vendor = r.string()?;
    }
    let _description = r.string()?;
    let _version = r.string()?;
    let _serial = r.string()?;
    let _location = r.string()?;

    let num_modes = r.u16()?;
    let active_mode = r.i32()?;
    let mut modes = Vec::with_capacity(num_modes as usize);
    for _ in 0..num_modes {
        let mut m = ModeInfo {
            name: r.string()?,
            value: r.i32()?,
            flags: r.u32()?,
            speed_min: r.u32()?,
            speed_max: r.u32()?,
            ..ModeInfo::default()
        };
        if protocol >= 3 {
            m.brightness_min = r.u32()?;
            m.brightness_max = r.u32()?;
        }
        m.colors_min = r.u32()?;
        m.colors_max = r.u32()?;
        m.speed = r.u32()?;
        if protocol >= 3 {
            m.brightness = r.u32()?;
        }
        m.direction = r.u32()?;
        m.color_mode = r.u32()?;
        let mode_colors = r.u16()? as usize;
        m.colors = Vec::with_capacity(mode_colors);
        for _ in 0..mode_colors {
            m.colors.push(r.u32()?);
        }
        modes.push(m);
    }

    let num_zones = r.u16()?;
    let mut zones = Vec::with_capacity(num_zones as usize);
    for _ in 0..num_zones {
        let zone_name = r.string()?;
        let zone_type = r.i32()?;
        let leds_min = r.u32()?;
        let leds_max = r.u32()?;
        let leds_count = r.u32()?;
        let matrix_len = r.u16()? as usize;
        r.take(matrix_len)?;
        zones.push(ZoneInfo {
            name: zone_name,
            zone_type,
            leds_min,
            leds_max,
            leds_count,
        });
    }

    let num_leds = r.u16()?;
    for _ in 0..num_leds {
        let _led_name = r.string()?;
        let _led_value = r.u32()?;
    }
    // Trailing per-LED color values are not needed; stop parsing here.

    Ok(ControllerInfo {
        controller_type,
        name,
        active_mode,
        modes,
        zones,
        num_leds: num_leds as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_layout() {
        let h = encode_header(2, 1051, 10);
        assert_eq!(&h[0..4], b"ORGB");
        assert_eq!(u32::from_le_bytes([h[4], h[5], h[6], h[7]]), 2);
        assert_eq!(u32::from_le_bytes([h[8], h[9], h[10], h[11]]), 1051);
        assert_eq!(u32::from_le_bytes([h[12], h[13], h[14], h[15]]), 10);
    }

    #[test]
    fn zone_update_payload() {
        let colors = [[255u8, 0, 0], [0, 255, 0]];
        let p = encode_zone_update(3, &colors);
        // total(4) + zone(4) + count(2) + 2 colors * 4 = 18
        assert_eq!(p.len(), 18);
        assert_eq!(u32::from_le_bytes([p[0], p[1], p[2], p[3]]), 18);
        assert_eq!(u32::from_le_bytes([p[4], p[5], p[6], p[7]]), 3);
        assert_eq!(u16::from_le_bytes([p[8], p[9]]), 2);
        assert_eq!(&p[10..14], &[255, 0, 0, 0]);
        assert_eq!(&p[14..18], &[0, 255, 0, 0]);
    }

    #[test]
    fn direct_mode_falls_back_to_custom_and_static() {
        fn mk(names: &[&str]) -> ControllerInfo {
            ControllerInfo {
                controller_type: 0,
                name: "x".into(),
                active_mode: 0,
                modes: names
                    .iter()
                    .map(|n| ModeInfo { name: n.to_string(), ..ModeInfo::default() })
                    .collect(),
                zones: Vec::new(),
                num_leds: 0,
            }
        }
        assert_eq!(mk(&["Rainbow", " Direct "]).direct_mode().unwrap().0, 1);
        assert_eq!(mk(&["Rainbow", "Custom"]).direct_mode().unwrap().0, 1);
        assert_eq!(mk(&["Rainbow", "Static"]).direct_mode().unwrap().0, 1);
        assert!(mk(&["Rainbow", "Off"]).direct_mode().is_none());
        // "Direct" outranks an earlier "Static".
        assert_eq!(mk(&["Static", "Direct"]).direct_mode().unwrap().0, 1);
    }

    #[test]
    fn update_mode_payload_layout() {
        let m = ModeInfo {
            name: "Direct".to_string(),
            value: 0xFFFF,
            flags: 0x20,
            colors_min: 0,
            colors_max: 0,
            color_mode: 1,
            ..ModeInfo::default()
        };
        let p = encode_update_mode(3, 5, &m);
        // total(4) + index(4) + name len(2)+7 + value(4) + flags(4) +
        // speed min/max(8) + brightness min/max(8) + colors min/max(8) +
        // speed(4) + brightness(4) + direction(4) + color_mode(4) + count(2)
        assert_eq!(p.len(), 67);
        assert_eq!(u32::from_le_bytes([p[0], p[1], p[2], p[3]]), 67);
        assert_eq!(i32::from_le_bytes([p[4], p[5], p[6], p[7]]), 5);
        assert_eq!(u16::from_le_bytes([p[8], p[9]]), 7); // "Direct\0"
        assert_eq!(&p[10..16], b"Direct");
        // Protocol 2 drops the three brightness fields (12 bytes).
        assert_eq!(encode_update_mode(2, 5, &m).len(), 55);
    }

    #[test]
    fn parsed_modes_roundtrip_into_direct_lookup() {
        fn put_str(buf: &mut Vec<u8>, s: &str) {
            let bytes = s.as_bytes();
            buf.extend_from_slice(&((bytes.len() + 1) as u16).to_le_bytes());
            buf.extend_from_slice(bytes);
            buf.push(0);
        }
        let mut b = Vec::new();
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0i32.to_le_bytes());
        put_str(&mut b, "Mobo");
        put_str(&mut b, "ASUS");
        put_str(&mut b, "desc");
        put_str(&mut b, "1.0");
        put_str(&mut b, "serial");
        put_str(&mut b, "loc");
        b.extend_from_slice(&2u16.to_le_bytes()); // num_modes
        b.extend_from_slice(&1i32.to_le_bytes()); // active_mode
        for (name, value) in [("Rainbow", 1i32), ("Direct", 0xFFFF)] {
            put_str(&mut b, name);
            b.extend_from_slice(&value.to_le_bytes());
            for _ in 0..11 {
                b.extend_from_slice(&0u32.to_le_bytes()); // flags..color_mode (p3)
            }
            b.extend_from_slice(&0u16.to_le_bytes()); // num_colors
        }
        b.extend_from_slice(&0u16.to_le_bytes()); // num_zones
        b.extend_from_slice(&0u16.to_le_bytes()); // num_leds

        let info = parse_controller_data(&b, 3).unwrap();
        assert_eq!(info.modes.len(), 2);
        assert_eq!(info.active_mode, 1);
        let (idx, direct) = info.direct_mode().expect("Direct mode present");
        assert_eq!(idx, 1);
        assert_eq!(direct.value, 0xFFFF);
    }

    #[test]
    fn led_update_payload() {
        let colors = [[1u8, 2, 3]];
        let p = encode_led_update(&colors);
        assert_eq!(p.len(), 10);
        assert_eq!(u32::from_le_bytes([p[0], p[1], p[2], p[3]]), 10);
        assert_eq!(u16::from_le_bytes([p[4], p[5]]), 1);
        assert_eq!(&p[6..10], &[1, 2, 3, 0]);
    }

    #[test]
    fn parse_minimal_controller_blob() {
        // Hand-build a protocol-3 controller: type 0, name "Mobo", no modes,
        // one linear zone "Addressable 2" with 72 leds, no leds/colors.
        fn put_str(buf: &mut Vec<u8>, s: &str) {
            let bytes = s.as_bytes();
            buf.extend_from_slice(&((bytes.len() + 1) as u16).to_le_bytes());
            buf.extend_from_slice(bytes);
            buf.push(0);
        }
        let mut b = Vec::new();
        b.extend_from_slice(&0u32.to_le_bytes()); // data_size (unused)
        b.extend_from_slice(&0i32.to_le_bytes()); // type = motherboard
        put_str(&mut b, "Mobo");
        put_str(&mut b, "ASUS"); // vendor (protocol >= 1)
        put_str(&mut b, "desc");
        put_str(&mut b, "1.0");
        put_str(&mut b, "serial");
        put_str(&mut b, "loc");
        b.extend_from_slice(&0u16.to_le_bytes()); // num_modes
        b.extend_from_slice(&0i32.to_le_bytes()); // active_mode
        b.extend_from_slice(&1u16.to_le_bytes()); // num_zones
        put_str(&mut b, "Addressable 2");
        b.extend_from_slice(&1i32.to_le_bytes()); // linear
        b.extend_from_slice(&0u32.to_le_bytes()); // leds_min
        b.extend_from_slice(&120u32.to_le_bytes()); // leds_max
        b.extend_from_slice(&72u32.to_le_bytes()); // leds_count
        b.extend_from_slice(&0u16.to_le_bytes()); // matrix_len
        b.extend_from_slice(&0u16.to_le_bytes()); // num_leds

        let info = parse_controller_data(&b, 3).unwrap();
        assert_eq!(info.controller_type, DEVICE_TYPE_MOTHERBOARD);
        assert_eq!(info.name, "Mobo");
        assert_eq!(info.zones.len(), 1);
        assert_eq!(info.zones[0].name, "Addressable 2");
        assert_eq!(info.zones[0].zone_type, ZONE_TYPE_LINEAR);
        assert_eq!(info.zones[0].leds_count, 72);
    }
}
