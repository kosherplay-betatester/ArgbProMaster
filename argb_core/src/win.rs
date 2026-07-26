//! Minimal hand-rolled Win32 bindings shared by the daemon and the GUI:
//! a named-mutex single-instance guard and a "is the daemon running?" probe.

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    type Handle = *mut c_void;

    const ERROR_ALREADY_EXISTS: u32 = 183;
    const SYNCHRONIZE: u32 = 0x0010_0000;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(attrs: *const c_void, initial_owner: i32, name: *const u16) -> Handle;
        fn OpenMutexW(desired_access: u32, inherit: i32, name: *const u16) -> Handle;
        fn CloseHandle(handle: Handle) -> i32;
        fn GetLastError() -> u32;
    }

    pub fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub const DAEMON_MUTEX_NAME: &str = "Local\\ArgbProMaster_ThermalDaemon";

    /// Holds a named mutex for the lifetime of the process.
    pub struct SingleInstance {
        handle: Handle,
    }

    // The handle is only closed on drop; it is safe to move between threads.
    unsafe impl Send for SingleInstance {}

    impl SingleInstance {
        /// Returns `None` if another process already owns the name.
        pub fn acquire(name: &str) -> Option<SingleInstance> {
            let wname = wide(name);
            unsafe {
                let handle = CreateMutexW(std::ptr::null(), 0, wname.as_ptr());
                if handle.is_null() {
                    return None;
                }
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    CloseHandle(handle);
                    return None;
                }
                Some(SingleInstance { handle })
            }
        }
    }

    impl Drop for SingleInstance {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }

    /// True if a live process currently holds the daemon mutex.
    pub fn daemon_running() -> bool {
        let wname = wide(DAEMON_MUTEX_NAME);
        unsafe {
            let handle = OpenMutexW(SYNCHRONIZE, 0, wname.as_ptr());
            if handle.is_null() {
                false
            } else {
                CloseHandle(handle);
                true
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub const DAEMON_MUTEX_NAME: &str = "ArgbProMaster_ThermalDaemon";

    pub struct SingleInstance;

    impl SingleInstance {
        pub fn acquire(_name: &str) -> Option<SingleInstance> {
            Some(SingleInstance)
        }
    }

    pub fn daemon_running() -> bool {
        false
    }
}

pub use imp::{daemon_running, SingleInstance, DAEMON_MUTEX_NAME};
