//! # epoll
//!
//! Thin Rust wrapper over Linux `epoll`.
//!
//! The crate stays intentionally close to the kernel API. It does not invent a
//! new concurrency model or hide Linux-specific semantics such as edge-triggered
//! and one-shot registration.

use std::io;
use std::ops::{BitOr, BitOrAssign};
use std::time::Duration;

#[cfg(target_os = "linux")]
mod imp {
    use super::{io, BitOr, BitOrAssign, Duration};
    use std::ffi::c_int;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

    const EPOLL_CLOEXEC: c_int = 0x80000;
    const EPOLL_CTL_ADD: c_int = 1;
    const EPOLL_CTL_DEL: c_int = 2;
    const EPOLL_CTL_MOD: c_int = 3;

    const EPOLLIN: u32 = 0x001;
    const EPOLLOUT: u32 = 0x004;
    const EPOLLERR: u32 = 0x008;
    const EPOLLHUP: u32 = 0x010;
    const EPOLLRDHUP: u32 = 0x2000;
    const EPOLLET: u32 = 1u32 << 31;
    const EPOLLONESHOT: u32 = 1u32 << 30;

    // Linux declares `struct epoll_event` as packed, so our raw FFI mirror
    // must match that layout exactly or multi-event waits can decode garbage.
    #[repr(C, packed)]
    #[derive(Clone, Copy, Debug, Default)]
    struct RawEpollEvent {
        events: u32,
        data: u64,
    }

    unsafe extern "C" {
        fn epoll_create1(flags: c_int) -> c_int;
        fn epoll_ctl(epfd: c_int, op: c_int, fd: c_int, event: *mut RawEpollEvent) -> c_int;
        fn epoll_wait(
            epfd: c_int,
            events: *mut RawEpollEvent,
            maxevents: c_int,
            timeout: c_int,
        ) -> c_int;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Interest(u32);

    impl Interest {
        pub const READABLE: Self = Self(EPOLLIN);
        pub const WRITABLE: Self = Self(EPOLLOUT);
        pub const EDGE_TRIGGERED: Self = Self(EPOLLET);
        pub const ONE_SHOT: Self = Self(EPOLLONESHOT);

        pub const fn bits(self) -> u32 {
            self.0
        }

        pub const fn contains(self, other: Self) -> bool {
            (self.0 & other.0) == other.0
        }
    }

    impl BitOr for Interest {
        type Output = Self;

        fn bitor(self, rhs: Self) -> Self::Output {
            Self(self.0 | rhs.0)
        }
    }

    impl BitOrAssign for Interest {
        fn bitor_assign(&mut self, rhs: Self) {
            self.0 |= rhs.0;
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EpollEvent {
        events: u32,
        token: u64,
    }

    impl EpollEvent {
        pub const fn new(token: u64, interest: Interest) -> Self {
            Self {
                events: interest.bits(),
                token,
            }
        }

        pub const fn token(self) -> u64 {
            self.token
        }

        pub const fn bits(self) -> u32 {
            self.events
        }

        pub const fn is_readable(self) -> bool {
            (self.events & EPOLLIN) != 0
        }

        pub const fn is_writable(self) -> bool {
            (self.events & EPOLLOUT) != 0
        }

        pub const fn is_error(self) -> bool {
            (self.events & EPOLLERR) != 0
        }

        pub const fn is_hangup(self) -> bool {
            (self.events & (EPOLLHUP | EPOLLRDHUP)) != 0
        }
    }

    impl From<RawEpollEvent> for EpollEvent {
        fn from(value: RawEpollEvent) -> Self {
            Self {
                events: value.events,
                token: value.data,
            }
        }
    }

    impl From<EpollEvent> for RawEpollEvent {
        fn from(value: EpollEvent) -> Self {
            Self {
                events: value.events,
                data: value.token,
            }
        }
    }

    #[derive(Debug)]
    pub struct Epoll {
        fd: OwnedFd,
    }

    impl Epoll {
        pub fn new(cloexec: bool) -> io::Result<Self> {
            let flags = if cloexec { EPOLL_CLOEXEC } else { 0 };
            let raw = cvt(unsafe { epoll_create1(flags) })?;
            let fd = unsafe { OwnedFd::from_raw_fd(raw) };
            Ok(Self { fd })
        }

        pub fn add(&self, fd: RawFd, event: EpollEvent) -> io::Result<()> {
            self.ctl(EPOLL_CTL_ADD, fd, Some(event))
        }

        pub fn modify(&self, fd: RawFd, event: EpollEvent) -> io::Result<()> {
            self.ctl(EPOLL_CTL_MOD, fd, Some(event))
        }

        pub fn delete(&self, fd: RawFd) -> io::Result<()> {
            self.ctl(EPOLL_CTL_DEL, fd, None)
        }

        pub fn wait(
            &self,
            max_events: usize,
            timeout: Option<Duration>,
        ) -> io::Result<Vec<EpollEvent>> {
            let max_events = max_events.max(1).min(c_int::MAX as usize);
            let mut events = vec![RawEpollEvent::default(); max_events];
            let timeout_ms = timeout_to_millis(timeout);
            let ready = cvt(unsafe {
                epoll_wait(
                    self.fd.as_raw_fd(),
                    events.as_mut_ptr(),
                    max_events as c_int,
                    timeout_ms,
                )
            })? as usize;
            events.truncate(ready);
            Ok(events.into_iter().map(EpollEvent::from).collect())
        }

        fn ctl(&self, op: c_int, fd: RawFd, event: Option<EpollEvent>) -> io::Result<()> {
            let mut raw = event.map(RawEpollEvent::from);
            let ptr = match raw.as_mut() {
                Some(raw) => raw as *mut RawEpollEvent,
                None => std::ptr::null_mut(),
            };
            cvt(unsafe { epoll_ctl(self.fd.as_raw_fd(), op, fd, ptr) })?;
            Ok(())
        }
    }

    fn cvt(result: c_int) -> io::Result<c_int> {
        if result == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(result)
        }
    }

    fn timeout_to_millis(timeout: Option<Duration>) -> c_int {
        match timeout {
            Some(duration) => duration.as_millis().min(c_int::MAX as u128) as c_int,
            None => -1,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::io::Write;
        use std::os::unix::net::UnixStream;

        #[test]
        fn interest_flags_combine() {
            let interest = Interest::READABLE | Interest::WRITABLE | Interest::EDGE_TRIGGERED;
            assert!(interest.contains(Interest::READABLE));
            assert!(interest.contains(Interest::WRITABLE));
            assert!(interest.contains(Interest::EDGE_TRIGGERED));
            assert!(!interest.contains(Interest::ONE_SHOT));
        }

        #[test]
        fn wait_reports_readable_socket() {
            let epoll = Epoll::new(true).expect("create epoll");
            let (left, mut right) = UnixStream::pair().expect("socket pair");
            epoll
                .add(left.as_raw_fd(), EpollEvent::new(7, Interest::READABLE))
                .expect("register fd");

            right.write_all(b"ping").expect("write to socket");
            let events = epoll
                .wait(8, Some(Duration::from_millis(100)))
                .expect("wait");

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].token(), 7);
            assert!(events[0].is_readable());
        }

        #[test]
        fn delete_stops_future_notifications() {
            let epoll = Epoll::new(true).expect("create epoll");
            let (left, mut right) = UnixStream::pair().expect("socket pair");
            epoll
                .add(left.as_raw_fd(), EpollEvent::new(11, Interest::READABLE))
                .expect("register fd");
            epoll.delete(left.as_raw_fd()).expect("delete fd");

            right.write_all(b"ping").expect("write to socket");
            let events = epoll
                .wait(4, Some(Duration::from_millis(10)))
                .expect("wait");
            assert!(events.is_empty());
        }

        #[test]
        fn wait_reports_multiple_ready_fds() {
            let epoll = Epoll::new(true).expect("create epoll");
            let (left_a, mut right_a) = UnixStream::pair().expect("socket pair a");
            let (left_b, mut right_b) = UnixStream::pair().expect("socket pair b");

            epoll
                .add(left_a.as_raw_fd(), EpollEvent::new(21, Interest::READABLE))
                .expect("register fd a");
            epoll
                .add(left_b.as_raw_fd(), EpollEvent::new(22, Interest::READABLE))
                .expect("register fd b");

            right_a.write_all(b"a").expect("write to socket a");
            right_b.write_all(b"b").expect("write to socket b");

            let events = epoll
                .wait(8, Some(Duration::from_millis(100)))
                .expect("wait");

            assert_eq!(events.len(), 2);
            assert!(events
                .iter()
                .any(|event| event.token() == 21 && event.is_readable()));
            assert!(events
                .iter()
                .any(|event| event.token() == 22 && event.is_readable()));
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use super::{io, BitOr, BitOrAssign, Duration};

    fn unsupported() -> io::Error {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "epoll is only available on Linux",
        )
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Interest(u32);

    impl Interest {
        pub const READABLE: Self = Self(1 << 0);
        pub const WRITABLE: Self = Self(1 << 1);
        pub const EDGE_TRIGGERED: Self = Self(1 << 2);
        pub const ONE_SHOT: Self = Self(1 << 3);

        pub const fn bits(self) -> u32 {
            self.0
        }

        pub const fn contains(self, other: Self) -> bool {
            (self.0 & other.0) == other.0
        }
    }

    impl BitOr for Interest {
        type Output = Self;

        fn bitor(self, rhs: Self) -> Self::Output {
            Self(self.0 | rhs.0)
        }
    }

    impl BitOrAssign for Interest {
        fn bitor_assign(&mut self, rhs: Self) {
            self.0 |= rhs.0;
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EpollEvent {
        events: u32,
        token: u64,
    }

    impl EpollEvent {
        pub const fn new(token: u64, interest: Interest) -> Self {
            Self {
                events: interest.bits(),
                token,
            }
        }

        pub const fn token(self) -> u64 {
            self.token
        }

        pub const fn bits(self) -> u32 {
            self.events
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

        pub const fn is_hangup(self) -> bool {
            false
        }
    }

    #[derive(Debug)]
    pub struct Epoll;

    impl Epoll {
        pub fn new(_cloexec: bool) -> io::Result<Self> {
            Err(unsupported())
        }

        pub fn add(&self, _fd: i32, _event: EpollEvent) -> io::Result<()> {
            Err(unsupported())
        }

        pub fn modify(&self, _fd: i32, _event: EpollEvent) -> io::Result<()> {
            Err(unsupported())
        }

        pub fn delete(&self, _fd: i32) -> io::Result<()> {
            Err(unsupported())
        }

        pub fn wait(
            &self,
            _max_events: usize,
            _timeout: Option<Duration>,
        ) -> io::Result<Vec<EpollEvent>> {
            Err(unsupported())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn reports_unsupported_off_linux() {
            let error = Epoll::new(true).expect_err("epoll should be unsupported");
            assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        }
    }
}

pub use imp::*;
