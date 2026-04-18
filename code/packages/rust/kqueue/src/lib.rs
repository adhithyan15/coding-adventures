//! # kqueue
//!
//! Thin Rust wrapper over BSD/macOS `kqueue` and `kevent`.
//!
//! The wrapper focuses on the TCP-first readiness path: read and write filters,
//! token-carrying registrations, and blocking waits.

use std::io;
use std::ops::{BitOr, BitOrAssign};
use std::time::Duration;

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
mod imp {
    use super::{io, BitOr, BitOrAssign, Duration};
    use std::ffi::{c_int, c_void};
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
    const EVFILT_READ: i16 = -1;
    const EVFILT_WRITE: i16 = -2;

    const EV_ADD: u16 = 0x0001;
    const EV_DELETE: u16 = 0x0002;
    const EV_ENABLE: u16 = 0x0004;
    const EV_DISABLE: u16 = 0x0008;
    const EV_ONESHOT: u16 = 0x0010;
    const EV_CLEAR: u16 = 0x0020;
    const EV_ERROR: u16 = 0x4000;
    const EV_EOF: u16 = 0x8000;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, Default)]
    struct Timespec {
        tv_sec: i64,
        tv_nsec: i64,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug)]
    struct RawKevent {
        ident: usize,
        filter: i16,
        flags: u16,
        fflags: u32,
        data: i64,
        udata: *mut c_void,
    }

    impl Default for RawKevent {
        fn default() -> Self {
            Self {
                ident: 0,
                filter: 0,
                flags: 0,
                fflags: 0,
                data: 0,
                udata: std::ptr::null_mut(),
            }
        }
    }

    unsafe extern "C" {
        fn kqueue() -> c_int;
        fn kevent(
            kq: c_int,
            changelist: *const RawKevent,
            nchanges: c_int,
            eventlist: *mut RawKevent,
            nevents: c_int,
            timeout: *const Timespec,
        ) -> c_int;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Filter {
        Read,
        Write,
    }

    impl Filter {
        const fn raw(self) -> i16 {
            match self {
                Self::Read => EVFILT_READ,
                Self::Write => EVFILT_WRITE,
            }
        }

        fn from_raw(raw: i16) -> io::Result<Self> {
            match raw {
                EVFILT_READ => Ok(Self::Read),
                EVFILT_WRITE => Ok(Self::Write),
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unsupported kqueue filter {}", raw),
                )),
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EventFlags(u16);

    impl EventFlags {
        pub const ADD: Self = Self(EV_ADD);
        pub const DELETE: Self = Self(EV_DELETE);
        pub const ENABLE: Self = Self(EV_ENABLE);
        pub const DISABLE: Self = Self(EV_DISABLE);
        pub const ONE_SHOT: Self = Self(EV_ONESHOT);
        pub const CLEAR: Self = Self(EV_CLEAR);

        pub const fn bits(self) -> u16 {
            self.0
        }

        pub const fn contains(self, other: Self) -> bool {
            (self.0 & other.0) == other.0
        }
    }

    impl BitOr for EventFlags {
        type Output = Self;

        fn bitor(self, rhs: Self) -> Self::Output {
            Self(self.0 | rhs.0)
        }
    }

    impl BitOrAssign for EventFlags {
        fn bitor_assign(&mut self, rhs: Self) {
            self.0 |= rhs.0;
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct KqueueChange {
        ident: usize,
        filter: Filter,
        flags: EventFlags,
        fflags: u32,
        data: i64,
        token: u64,
    }

    impl KqueueChange {
        pub const fn new(ident: usize, filter: Filter, token: u64, flags: EventFlags) -> Self {
            Self {
                ident,
                filter,
                flags,
                fflags: 0,
                data: 0,
                token,
            }
        }

        pub fn readable(fd: RawFd, token: u64) -> Self {
            Self::new(
                fd as usize,
                Filter::Read,
                token,
                EventFlags::ADD | EventFlags::ENABLE,
            )
        }

        pub fn writable(fd: RawFd, token: u64) -> Self {
            Self::new(
                fd as usize,
                Filter::Write,
                token,
                EventFlags::ADD | EventFlags::ENABLE,
            )
        }

        pub fn with_flags(mut self, flags: EventFlags) -> Self {
            self.flags = flags;
            self
        }
    }

    impl From<KqueueChange> for RawKevent {
        fn from(value: KqueueChange) -> Self {
            Self {
                ident: value.ident,
                filter: value.filter.raw(),
                flags: value.flags.bits(),
                fflags: value.fflags,
                data: value.data,
                udata: value.token as usize as *mut c_void,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct KqueueEvent {
        ident: usize,
        filter: Filter,
        flags: u16,
        data: i64,
        token: u64,
    }

    impl KqueueEvent {
        pub const fn ident(self) -> usize {
            self.ident
        }

        pub const fn filter(self) -> Filter {
            self.filter
        }

        pub const fn token(self) -> u64 {
            self.token
        }

        pub const fn flags(self) -> u16 {
            self.flags
        }

        pub const fn data(self) -> i64 {
            self.data
        }

        pub const fn is_readable(self) -> bool {
            matches!(self.filter, Filter::Read)
        }

        pub const fn is_writable(self) -> bool {
            matches!(self.filter, Filter::Write)
        }

        pub const fn is_error(self) -> bool {
            (self.flags & EV_ERROR) != 0
        }

        pub const fn is_eof(self) -> bool {
            (self.flags & EV_EOF) != 0
        }
    }

    impl TryFrom<RawKevent> for KqueueEvent {
        type Error = io::Error;

        fn try_from(value: RawKevent) -> Result<Self, Self::Error> {
            Ok(Self {
                ident: value.ident,
                filter: Filter::from_raw(value.filter)?,
                flags: value.flags,
                data: value.data,
                token: value.udata as usize as u64,
            })
        }
    }

    #[derive(Debug)]
    pub struct Kqueue {
        fd: OwnedFd,
    }

    impl Kqueue {
        pub fn new() -> io::Result<Self> {
            let raw = cvt(unsafe { kqueue() })?;
            let fd = unsafe { OwnedFd::from_raw_fd(raw) };
            Ok(Self { fd })
        }

        pub fn apply(&self, change: KqueueChange) -> io::Result<()> {
            self.apply_all(&[change])
        }

        pub fn apply_all(&self, changes: &[KqueueChange]) -> io::Result<()> {
            let raw_changes: Vec<RawKevent> =
                changes.iter().copied().map(RawKevent::from).collect();
            let changelist = if raw_changes.is_empty() {
                std::ptr::null()
            } else {
                raw_changes.as_ptr()
            };
            cvt(unsafe {
                kevent(
                    self.fd.as_raw_fd(),
                    changelist,
                    raw_changes.len() as c_int,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                )
            })?;
            Ok(())
        }

        pub fn wait(
            &self,
            max_events: usize,
            timeout: Option<Duration>,
        ) -> io::Result<Vec<KqueueEvent>> {
            let max_events = max_events.max(1).min(c_int::MAX as usize);
            let mut events = vec![RawKevent::default(); max_events];
            let timeout = timeout.map(duration_to_timespec);
            let timeout_ptr = timeout
                .as_ref()
                .map(|ts| ts as *const Timespec)
                .unwrap_or(std::ptr::null());
            let ready = cvt(unsafe {
                kevent(
                    self.fd.as_raw_fd(),
                    std::ptr::null(),
                    0,
                    events.as_mut_ptr(),
                    max_events as c_int,
                    timeout_ptr,
                )
            })? as usize;
            events.truncate(ready);
            events.into_iter().map(KqueueEvent::try_from).collect()
        }
    }

    fn cvt(result: c_int) -> io::Result<c_int> {
        if result == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(result)
        }
    }

    fn duration_to_timespec(duration: Duration) -> Timespec {
        Timespec {
            tv_sec: duration.as_secs().min(i64::MAX as u64) as i64,
            tv_nsec: duration.subsec_nanos() as i64,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::io::Write;
        use std::os::unix::net::UnixStream;

        #[test]
        fn apply_and_wait_for_readable_socket() {
            let queue = Kqueue::new().expect("create kqueue");
            let (left, mut right) = UnixStream::pair().expect("socket pair");
            queue
                .apply(
                    KqueueChange::readable(left.as_raw_fd(), 21)
                        .with_flags(EventFlags::ADD | EventFlags::ENABLE | EventFlags::CLEAR),
                )
                .expect("register read filter");

            right.write_all(b"ping").expect("write to socket");
            let events = queue
                .wait(8, Some(Duration::from_millis(100)))
                .expect("wait");

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].token(), 21);
            assert!(events[0].is_readable());
        }

        #[test]
        fn delete_removes_filter() {
            let queue = Kqueue::new().expect("create kqueue");
            let (left, mut right) = UnixStream::pair().expect("socket pair");
            queue
                .apply(
                    KqueueChange::readable(left.as_raw_fd(), 22)
                        .with_flags(EventFlags::ADD | EventFlags::ENABLE | EventFlags::CLEAR),
                )
                .expect("register read filter");
            queue
                .apply(KqueueChange::readable(left.as_raw_fd(), 22).with_flags(EventFlags::DELETE))
                .expect("delete read filter");

            right.write_all(b"ping").expect("write to socket");
            let events = queue
                .wait(4, Some(Duration::from_millis(10)))
                .expect("wait");
            assert!(events.is_empty());
        }
    }
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
)))]
mod imp {
    use super::{io, BitOr, BitOrAssign, Duration};

    fn unsupported() -> io::Error {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "kqueue is only available on BSD-family systems and macOS",
        )
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Filter {
        Read,
        Write,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EventFlags(u16);

    impl EventFlags {
        pub const ADD: Self = Self(1 << 0);
        pub const DELETE: Self = Self(1 << 1);
        pub const ENABLE: Self = Self(1 << 2);
        pub const DISABLE: Self = Self(1 << 3);
        pub const ONE_SHOT: Self = Self(1 << 4);
        pub const CLEAR: Self = Self(1 << 5);

        pub const fn bits(self) -> u16 {
            self.0
        }

        pub const fn contains(self, other: Self) -> bool {
            (self.0 & other.0) == other.0
        }
    }

    impl BitOr for EventFlags {
        type Output = Self;

        fn bitor(self, rhs: Self) -> Self::Output {
            Self(self.0 | rhs.0)
        }
    }

    impl BitOrAssign for EventFlags {
        fn bitor_assign(&mut self, rhs: Self) {
            self.0 |= rhs.0;
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct KqueueChange;

    impl KqueueChange {
        pub const fn new(_ident: usize, _filter: Filter, _token: u64, _flags: EventFlags) -> Self {
            Self
        }

        pub const fn readable(_fd: i32, _token: u64) -> Self {
            Self
        }

        pub const fn writable(_fd: i32, _token: u64) -> Self {
            Self
        }

        pub const fn with_flags(self, _flags: EventFlags) -> Self {
            self
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct KqueueEvent;

    impl KqueueEvent {
        pub const fn ident(self) -> usize {
            0
        }

        pub const fn filter(self) -> Filter {
            Filter::Read
        }

        pub const fn token(self) -> u64 {
            0
        }

        pub const fn flags(self) -> u16 {
            0
        }

        pub const fn data(self) -> i64 {
            0
        }

        pub const fn is_readable(self) -> bool {
            false
        }

        pub const fn is_writable(self) -> bool {
            false
        }

        pub const fn is_error(self) -> bool {
            false
        }

        pub const fn is_eof(self) -> bool {
            false
        }
    }

    #[derive(Debug)]
    pub struct Kqueue;

    impl Kqueue {
        pub fn new() -> io::Result<Self> {
            Err(unsupported())
        }

        pub fn apply(&self, _change: KqueueChange) -> io::Result<()> {
            Err(unsupported())
        }

        pub fn apply_all(&self, _changes: &[KqueueChange]) -> io::Result<()> {
            Err(unsupported())
        }

        pub fn wait(
            &self,
            _max_events: usize,
            _timeout: Option<Duration>,
        ) -> io::Result<Vec<KqueueEvent>> {
            Err(unsupported())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn reports_unsupported_off_bsd() {
            let error = Kqueue::new().expect_err("kqueue should be unsupported");
            assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        }
    }
}

pub use imp::*;
