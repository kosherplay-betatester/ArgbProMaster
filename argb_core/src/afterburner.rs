//! Reader for MSI Afterburner's `MAHMSharedMemory` memory-mapped file.
//!
//! Layout (from MAHMSharedMemory.h):
//! ```text
//! header: dwSignature 'MAHM', dwVersion, dwHeaderSize, dwNumEntries, dwEntrySize, ...
//! entry:  szSrcName[260], szSrcUnits[260], szLocalizedSrcName[260],
//!         szLocalizedSrcUnits[260], szRecommendedFormat[260],
//!         data: f32, minLimit: f32, maxLimit: f32, dwFlags, dwGpu, dwSrcId
//! ```
//! Entry `i` lives at `base + dwHeaderSize + i * dwEntrySize`; the `data`
//! float sits at byte offset 1300 inside an entry (5 * 260 name bytes).
//!
//! Shared by the daemon (drives the LEDs) and the GUI (the live preview's
//! "follow real temperatures" mode), so both see identical values.

/// Current CPU / GPU temperatures, when the sources exist.
#[derive(Clone, Copy, Debug, Default)]
pub struct Temps {
    pub cpu: Option<f32>,
    pub gpu: Option<f32>,
}

/// Every system metric a zone can follow, when Afterburner publishes it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Readings {
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    /// Percent 0..100.
    pub cpu_load: Option<f32>,
    /// Percent 0..100.
    pub gpu_load: Option<f32>,
    /// Percent of installed memory 0..100.
    pub ram_pct: Option<f32>,
    pub fps: Option<f32>,
    /// Afterburner's own scale hint for framerate (0 when unknown).
    pub fps_max: f32,
}

#[cfg(windows)]
mod imp {
    use super::Temps;
    use std::ffi::c_void;

    type Handle = *mut c_void;

    const FILE_MAP_READ: u32 = 0x0004;
    const ENTRY_NAME_LEN: usize = 260;
    const ENTRY_DATA_OFFSET: usize = 5 * ENTRY_NAME_LEN; // 1300

    // 'MAHM' both as a C multi-char constant and as little-endian bytes,
    // depending on which way the writer packed it.
    const SIG_MULTICHAR: u32 = 0x4D41_484D;
    const SIG_BYTES_LE: u32 = u32::from_le_bytes(*b"MAHM");
    /// Afterburner sets the signature to 0xDEAD while tearing down.
    const SIG_DEAD: u32 = 0xDEAD;

    #[link(name = "kernel32")]
    extern "system" {
        fn OpenFileMappingW(desired_access: u32, inherit: i32, name: *const u16) -> Handle;
        fn MapViewOfFile(h: Handle, desired_access: u32, off_hi: u32, off_lo: u32, size: usize) -> *mut c_void;
        fn UnmapViewOfFile(base: *const c_void) -> i32;
        fn CloseHandle(h: Handle) -> i32;
    }

    pub struct MahmReader {
        handle: Handle,
        view: *const u8,
    }

    // The mapping is read-only and only unmapped on drop.
    unsafe impl Send for MahmReader {}

    impl MahmReader {
        pub fn open() -> Option<MahmReader> {
            let name: Vec<u16> = "MAHMSharedMemory"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            unsafe {
                let handle = OpenFileMappingW(FILE_MAP_READ, 0, name.as_ptr());
                if handle.is_null() {
                    return None;
                }
                let view = MapViewOfFile(handle, FILE_MAP_READ, 0, 0, 0);
                if view.is_null() {
                    CloseHandle(handle);
                    return None;
                }
                Some(MahmReader {
                    handle,
                    view: view as *const u8,
                })
            }
        }

        /// Read the current CPU / GPU temperatures. Returns `None` when the
        /// shared memory is stale or malformed (e.g. Afterburner shutting down).
        pub fn read_temps(&self) -> Option<Temps> {
            self.read_all().map(|r| Temps { cpu: r.cpu_temp, gpu: r.gpu_temp })
        }

        /// Read every supported metric in one pass over the shared memory.
        pub fn read_all(&self) -> Option<super::Readings> {
            unsafe {
                let base = self.view;
                let signature = read_u32(base, 0);
                if signature == SIG_DEAD {
                    return None;
                }
                if signature != SIG_MULTICHAR && signature != SIG_BYTES_LE {
                    return None;
                }
                let header_size = read_u32(base, 8) as usize;
                let num_entries = read_u32(base, 12) as usize;
                let entry_size = read_u32(base, 16) as usize;
                if header_size < 20 || entry_size < ENTRY_DATA_OFFSET + 4 || num_entries == 0 || num_entries > 4096 {
                    return None;
                }

                let mut r = super::Readings::default();
                for i in 0..num_entries {
                    let entry = base.add(header_size + i * entry_size);
                    let name = read_cstr(entry, ENTRY_NAME_LEN);
                    let value = f32::from_le_bytes([
                        *entry.add(ENTRY_DATA_OFFSET),
                        *entry.add(ENTRY_DATA_OFFSET + 1),
                        *entry.add(ENTRY_DATA_OFFSET + 2),
                        *entry.add(ENTRY_DATA_OFFSET + 3),
                    ]);
                    if !value.is_finite() || !(-1000.0..=1_000_000.0).contains(&value) {
                        continue;
                    }
                    let max_limit = f32::from_le_bytes([
                        *entry.add(ENTRY_DATA_OFFSET + 8),
                        *entry.add(ENTRY_DATA_OFFSET + 9),
                        *entry.add(ENTRY_DATA_OFFSET + 10),
                        *entry.add(ENTRY_DATA_OFFSET + 11),
                    ]);

                    // Aggregate sources win; per-core/per-gpu entries
                    // ("CPU1 temperature", "GPU1 usage") fill gaps.
                    let temp_ok = (-100.0..200.0).contains(&value);
                    if name.eq_ignore_ascii_case("CPU temperature") && temp_ok {
                        r.cpu_temp = Some(value);
                    } else if name.eq_ignore_ascii_case("GPU temperature") && temp_ok {
                        r.gpu_temp = Some(value);
                    } else if r.cpu_temp.is_none() && temp_ok && name.starts_with("CPU") && name.ends_with("temperature") {
                        r.cpu_temp = Some(value);
                    } else if r.gpu_temp.is_none() && temp_ok && name.starts_with("GPU") && name.ends_with("temperature") {
                        r.gpu_temp = Some(value);
                    } else if name.eq_ignore_ascii_case("CPU usage") {
                        r.cpu_load = Some(value.clamp(0.0, 100.0));
                    } else if name.eq_ignore_ascii_case("GPU usage") {
                        r.gpu_load = Some(value.clamp(0.0, 100.0));
                    } else if r.cpu_load.is_none() && name.starts_with("CPU") && name.ends_with("usage") {
                        r.cpu_load = Some(value.clamp(0.0, 100.0));
                    } else if r.gpu_load.is_none() && name.starts_with("GPU") && name.ends_with("usage") {
                        r.gpu_load = Some(value.clamp(0.0, 100.0));
                    } else if name.eq_ignore_ascii_case("RAM usage") {
                        // Reported in MB; the entry's max limit is installed RAM.
                        if max_limit > 0.0 {
                            r.ram_pct = Some((value / max_limit * 100.0).clamp(0.0, 100.0));
                        }
                    } else if name.eq_ignore_ascii_case("Framerate") {
                        r.fps = Some(value.max(0.0));
                        if max_limit.is_finite() && max_limit > 0.0 {
                            r.fps_max = max_limit;
                        }
                    }
                }
                Some(r)
            }
        }
    }

    impl Drop for MahmReader {
        fn drop(&mut self) {
            unsafe {
                UnmapViewOfFile(self.view as *const c_void);
                CloseHandle(self.handle);
            }
        }
    }

    unsafe fn read_u32(base: *const u8, offset: usize) -> u32 {
        u32::from_le_bytes([
            *base.add(offset),
            *base.add(offset + 1),
            *base.add(offset + 2),
            *base.add(offset + 3),
        ])
    }

    unsafe fn read_cstr(ptr: *const u8, max_len: usize) -> String {
        let slice = std::slice::from_raw_parts(ptr, max_len);
        let end = slice.iter().position(|&b| b == 0).unwrap_or(max_len);
        String::from_utf8_lossy(&slice[..end]).trim().to_string()
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{Readings, Temps};

    pub struct MahmReader;

    impl MahmReader {
        pub fn open() -> Option<MahmReader> {
            None
        }
        pub fn read_temps(&self) -> Option<Temps> {
            None
        }
        pub fn read_all(&self) -> Option<Readings> {
            None
        }
    }
}

pub use imp::MahmReader;
