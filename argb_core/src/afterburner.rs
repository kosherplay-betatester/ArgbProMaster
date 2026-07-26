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

                let mut temps = Temps::default();
                for i in 0..num_entries {
                    let entry = base.add(header_size + i * entry_size);
                    let name = read_cstr(entry, ENTRY_NAME_LEN);
                    let is_cpu = name.eq_ignore_ascii_case("CPU temperature");
                    let is_gpu = name.eq_ignore_ascii_case("GPU temperature");
                    // Fall back to per-core / per-gpu entries ("CPU1 temperature",
                    // "GPU1 temperature") if the aggregate sources are disabled.
                    let is_cpu_like = !is_cpu
                        && temps.cpu.is_none()
                        && name.starts_with("CPU")
                        && name.ends_with("temperature");
                    let is_gpu_like = !is_gpu
                        && temps.gpu.is_none()
                        && name.starts_with("GPU")
                        && name.ends_with("temperature");
                    if is_cpu || is_gpu || is_cpu_like || is_gpu_like {
                        let value = f32::from_le_bytes([
                            *entry.add(ENTRY_DATA_OFFSET),
                            *entry.add(ENTRY_DATA_OFFSET + 1),
                            *entry.add(ENTRY_DATA_OFFSET + 2),
                            *entry.add(ENTRY_DATA_OFFSET + 3),
                        ]);
                        if value.is_finite() && value > -100.0 && value < 200.0 {
                            if is_cpu || (is_cpu_like && temps.cpu.is_none()) {
                                temps.cpu = Some(value);
                            }
                            if is_gpu || (is_gpu_like && temps.gpu.is_none()) {
                                temps.gpu = Some(value);
                            }
                        }
                    }
                }
                Some(temps)
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
    use super::Temps;

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

pub use imp::MahmReader;
