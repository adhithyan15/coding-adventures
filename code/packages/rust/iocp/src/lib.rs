//! # iocp
//!
//! Thin Rust wrapper over Windows I/O Completion Ports.
//!
//! This crate exposes the completion-port primitives directly instead of
//! pretending that Windows is a readiness-based polling platform.

use std::io;
use std::time::Duration;

#[cfg(target_os = "windows")]
mod imp {
    use super::{io, Duration};
    use std::ffi::c_void;
    use std::ptr;

    type Handle = *mut c_void;
    type Bool = i32;
    type Dword = u32;
    type UlongPtr = usize;

    const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
    const WAIT_TIMEOUT: i32 = 258;
    const INFINITE: Dword = 0xFFFF_FFFF;

    #[repr(C)]
    #[derive(Debug)]
    pub struct Overlapped {
        pub internal: usize,
        pub internal_high: usize,
        pub offset: u32,
        pub offset_high: u32,
        pub h_event: Handle,
    }

    unsafe extern "system" {
        fn CreateIoCompletionPort(
            file_handle: Handle,
            existing_completion_port: Handle,
            completion_key: UlongPtr,
            number_of_concurrent_threads: Dword,
        ) -> Handle;
        fn GetQueuedCompletionStatus(
            completion_port: Handle,
            lp_number_of_bytes_transferred: *mut Dword,
            lp_completion_key: *mut UlongPtr,
            lp_overlapped: *mut *mut Overlapped,
            dw_milliseconds: Dword,
        ) -> Bool;
        fn PostQueuedCompletionStatus(
            completion_port: Handle,
            dw_number_of_bytes_transferred: Dword,
            dw_completion_key: UlongPtr,
            lp_overlapped: *mut Overlapped,
        ) -> Bool;
        fn CloseHandle(handle: Handle) -> Bool;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CompletionPacket {
        pub bytes_transferred: u32,
        pub completion_key: usize,
        pub overlapped: *mut Overlapped,
        pub error: Option<i32>,
    }

    #[derive(Debug)]
    pub struct CompletionPort {
        handle: Handle,
    }

    unsafe impl Send for CompletionPort {}
    unsafe impl Sync for CompletionPort {}

    impl CompletionPort {
        pub fn new(concurrency: u32) -> io::Result<Self> {
            let handle = unsafe {
                CreateIoCompletionPort(
                    INVALID_HANDLE_VALUE,
                    ptr::null_mut(),
                    0,
                    concurrency as Dword,
                )
            };
            if handle.is_null() {
                Err(io::Error::last_os_error())
            } else {
                Ok(Self { handle })
            }
        }

        pub fn associate_handle(&self, handle: Handle, key: usize) -> io::Result<()> {
            let result = unsafe { CreateIoCompletionPort(handle, self.handle, key, 0) };
            if result.is_null() {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }

        pub fn post(
            &self,
            bytes_transferred: u32,
            completion_key: usize,
            overlapped: *mut Overlapped,
        ) -> io::Result<()> {
            let ok = unsafe {
                PostQueuedCompletionStatus(
                    self.handle,
                    bytes_transferred as Dword,
                    completion_key,
                    overlapped,
                )
            };
            if ok == 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }

        pub fn get(&self, timeout: Option<Duration>) -> io::Result<CompletionPacket> {
            let timeout = timeout_to_millis(timeout);
            let mut bytes = 0u32;
            let mut key = 0usize;
            let mut overlapped: *mut Overlapped = ptr::null_mut();
            let ok = unsafe {
                GetQueuedCompletionStatus(
                    self.handle,
                    &mut bytes,
                    &mut key,
                    &mut overlapped,
                    timeout,
                )
            };
            if ok == 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() == Some(WAIT_TIMEOUT) {
                    Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "IOCP wait timed out",
                    ))
                } else if !overlapped.is_null() {
                    Ok(CompletionPacket {
                        bytes_transferred: bytes,
                        completion_key: key,
                        overlapped,
                        error: error.raw_os_error(),
                    })
                } else {
                    Err(error)
                }
            } else {
                Ok(CompletionPacket {
                    bytes_transferred: bytes,
                    completion_key: key,
                    overlapped,
                    error: None,
                })
            }
        }
    }

    impl Drop for CompletionPort {
        fn drop(&mut self) {
            let _ = unsafe { CloseHandle(self.handle) };
        }
    }

    fn timeout_to_millis(timeout: Option<Duration>) -> Dword {
        match timeout {
            Some(duration) => duration.as_millis().min(Dword::MAX as u128) as Dword,
            None => INFINITE,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::ptr;

        #[test]
        fn posted_packet_round_trips() {
            let port = CompletionPort::new(1).expect("create completion port");
            port.post(5, 9, ptr::null_mut()).expect("post packet");
            let packet = port.get(Some(Duration::from_secs(1))).expect("get packet");
            assert_eq!(packet.bytes_transferred, 5);
            assert_eq!(packet.completion_key, 9);
            assert!(packet.overlapped.is_null());
            assert_eq!(packet.error, None);
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::{io, Duration};

    fn unsupported() -> io::Error {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "IOCP is only available on Windows",
        )
    }

    #[repr(C)]
    #[derive(Debug)]
    pub struct Overlapped {
        _private: (),
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CompletionPacket {
        pub bytes_transferred: u32,
        pub completion_key: usize,
        pub overlapped: *mut Overlapped,
        pub error: Option<i32>,
    }

    #[derive(Debug)]
    pub struct CompletionPort;

    impl CompletionPort {
        pub fn new(_concurrency: u32) -> io::Result<Self> {
            Err(unsupported())
        }

        pub fn associate_handle(
            &self,
            _handle: *mut std::ffi::c_void,
            _key: usize,
        ) -> io::Result<()> {
            Err(unsupported())
        }

        pub fn post(
            &self,
            _bytes_transferred: u32,
            _completion_key: usize,
            _overlapped: *mut Overlapped,
        ) -> io::Result<()> {
            Err(unsupported())
        }

        pub fn get(&self, _timeout: Option<Duration>) -> io::Result<CompletionPacket> {
            Err(unsupported())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn reports_unsupported_off_windows() {
            let error = CompletionPort::new(1).expect_err("iocp should be unsupported");
            assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        }
    }
}

pub use imp::*;
