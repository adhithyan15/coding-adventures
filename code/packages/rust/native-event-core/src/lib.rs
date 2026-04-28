//! # native-event-core
//!
//! Generic native event substrate above raw backends such as `epoll`, `kqueue`,
//! and `iocp`.
//!
//! The crate stops at the event boundary. It knows about tokens, interests,
//! source kinds, normalized events, and backend polling traits. It deliberately
//! does not know about TCP request parsing, WebSocket framing, or widget trees.

use std::collections::BTreeMap;
use std::io;
use std::ops::{BitOr, BitOrAssign};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceRef(pub usize);

impl SourceRef {
    #[cfg(unix)]
    pub const fn from_fd(fd: std::os::fd::RawFd) -> Self {
        Self(fd as usize)
    }

    #[cfg(target_os = "windows")]
    pub fn from_raw_handle(handle: std::os::windows::io::RawHandle) -> Self {
        Self(handle as usize)
    }

    pub const fn raw(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interest(u8);

impl Interest {
    pub const READABLE: Self = Self(1 << 0);
    pub const WRITABLE: Self = Self(1 << 1);

    pub const fn bits(self) -> u8 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn is_readable(self) -> bool {
        self.contains(Self::READABLE)
    }

    pub const fn is_writable(self) -> bool {
        self.contains(Self::WRITABLE)
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
pub enum SourceKind {
    Io,
    Wake,
    Timer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollTimeout {
    Immediate,
    Infinite,
    After(Duration),
}

impl PollTimeout {
    pub const fn into_duration(self) -> Option<Duration> {
        match self {
            Self::Immediate => Some(Duration::from_millis(0)),
            Self::Infinite => None,
            Self::After(duration) => Some(duration),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEvent {
    pub token: Token,
    pub source: SourceKind,
    pub readable: bool,
    pub writable: bool,
    pub error: bool,
    pub hangup: bool,
    pub completion_bytes: Option<u32>,
}

impl NativeEvent {
    pub const fn readiness(
        token: Token,
        source: SourceKind,
        readable: bool,
        writable: bool,
        error: bool,
        hangup: bool,
    ) -> Self {
        Self {
            token,
            source,
            readable,
            writable,
            error,
            hangup,
            completion_bytes: None,
        }
    }

    pub const fn completion(token: Token, source: SourceKind, bytes: u32) -> Self {
        Self {
            token,
            source,
            readable: false,
            writable: false,
            error: false,
            hangup: false,
            completion_bytes: Some(bytes),
        }
    }
}

pub trait EventBackend {
    fn register(
        &mut self,
        source: SourceRef,
        token: Token,
        kind: SourceKind,
        interest: Interest,
    ) -> io::Result<()>;

    fn reregister(
        &mut self,
        source: SourceRef,
        token: Token,
        kind: SourceKind,
        interest: Interest,
    ) -> io::Result<()>;

    fn deregister(&mut self, source: SourceRef, token: Token) -> io::Result<()>;

    fn poll(&mut self, timeout: PollTimeout) -> io::Result<Vec<NativeEvent>>;
}

pub struct NativeEventLoop<B> {
    backend: B,
}

impl<B> NativeEventLoop<B> {
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    pub fn into_backend(self) -> B {
        self.backend
    }
}

impl<B: EventBackend> NativeEventLoop<B> {
    pub fn register(
        &mut self,
        source: SourceRef,
        token: Token,
        kind: SourceKind,
        interest: Interest,
    ) -> io::Result<()> {
        self.backend.register(source, token, kind, interest)
    }

    pub fn reregister(
        &mut self,
        source: SourceRef,
        token: Token,
        kind: SourceKind,
        interest: Interest,
    ) -> io::Result<()> {
        self.backend.reregister(source, token, kind, interest)
    }

    pub fn deregister(&mut self, source: SourceRef, token: Token) -> io::Result<()> {
        self.backend.deregister(source, token)
    }

    pub fn poll_once(&mut self, timeout: PollTimeout) -> io::Result<Vec<NativeEvent>> {
        self.backend.poll(timeout)
    }
}

type SourceState = (Token, SourceKind, Interest);

#[derive(Default)]
struct SourceRegistry {
    by_source: BTreeMap<SourceRef, SourceState>,
    by_token: BTreeMap<Token, SourceKind>,
}

impl SourceRegistry {
    #[allow(dead_code)]
    fn len(&self) -> usize {
        self.by_source.len()
    }

    #[allow(dead_code)]
    fn get(&self, source: &SourceRef) -> Option<&SourceState> {
        self.by_source.get(source)
    }

    fn insert(&mut self, source: SourceRef, token: Token, kind: SourceKind, interest: Interest) {
        if let Some((previous_token, _, _)) = self.by_source.insert(source, (token, kind, interest))
        {
            self.by_token.remove(&previous_token);
        }
        self.by_token.insert(token, kind);
    }

    fn remove(&mut self, source: &SourceRef) -> Option<SourceState> {
        let state = self.by_source.remove(source)?;
        self.by_token.remove(&state.0);
        Some(state)
    }

    fn kind_for_token(&self, token: Token) -> SourceKind {
        self.by_token.get(&token).copied().unwrap_or(SourceKind::Io)
    }
}

#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;

    pub struct LinuxBackend {
        poller: epoll::Epoll,
        sources: SourceRegistry,
    }

    impl LinuxBackend {
        pub fn new() -> io::Result<Self> {
            Ok(Self {
                poller: epoll::Epoll::new(true)?,
                sources: SourceRegistry::default(),
            })
        }
    }

    impl EventBackend for LinuxBackend {
        fn register(
            &mut self,
            source: SourceRef,
            token: Token,
            kind: SourceKind,
            interest: Interest,
        ) -> io::Result<()> {
            let event = epoll::EpollEvent::new(token.0, to_epoll_interest(interest));
            self.poller.add(source.raw() as i32, event)?;
            self.sources.insert(source, token, kind, interest);
            Ok(())
        }

        fn reregister(
            &mut self,
            source: SourceRef,
            token: Token,
            kind: SourceKind,
            interest: Interest,
        ) -> io::Result<()> {
            let event = epoll::EpollEvent::new(token.0, to_epoll_interest(interest));
            self.poller.modify(source.raw() as i32, event)?;
            self.sources.insert(source, token, kind, interest);
            Ok(())
        }

        fn deregister(&mut self, source: SourceRef, _token: Token) -> io::Result<()> {
            self.poller.delete(source.raw() as i32)?;
            self.sources.remove(&source);
            Ok(())
        }

        fn poll(&mut self, timeout: PollTimeout) -> io::Result<Vec<NativeEvent>> {
            let raw = self
                .poller
                .wait(self.sources.len().max(1), timeout.into_duration())?;
            Ok(raw
                .into_iter()
                .map(|event| {
                    let token = Token(event.token());
                    let source = self.sources.kind_for_token(token);
                    NativeEvent::readiness(
                        token,
                        source,
                        event.is_readable(),
                        event.is_writable(),
                        event.is_error(),
                        event.is_hangup(),
                    )
                })
                .collect())
        }
    }

    fn to_epoll_interest(interest: Interest) -> epoll::Interest {
        let mut native = epoll::Interest::READABLE;
        if !interest.is_readable() {
            native = epoll::Interest::WRITABLE;
            if !interest.is_writable() {
                native = epoll::Interest::READABLE;
            }
        }
        if interest.is_readable() && interest.is_writable() {
            native = epoll::Interest::READABLE | epoll::Interest::WRITABLE;
        }
        native
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
pub mod bsd {
    use super::*;

    pub struct KqueueBackend {
        poller: kqueue::Kqueue,
        sources: SourceRegistry,
    }

    impl KqueueBackend {
        pub fn new() -> io::Result<Self> {
            Ok(Self {
                poller: kqueue::Kqueue::new()?,
                sources: SourceRegistry::default(),
            })
        }
    }

    impl EventBackend for KqueueBackend {
        fn register(
            &mut self,
            source: SourceRef,
            token: Token,
            kind: SourceKind,
            interest: Interest,
        ) -> io::Result<()> {
            apply_interest(
                &self.poller,
                source,
                token,
                interest,
                kqueue::EventFlags::ADD | kqueue::EventFlags::ENABLE | kqueue::EventFlags::CLEAR,
            )?;
            self.sources.insert(source, token, kind, interest);
            Ok(())
        }

        fn reregister(
            &mut self,
            source: SourceRef,
            token: Token,
            kind: SourceKind,
            interest: Interest,
        ) -> io::Result<()> {
            if let Some((_, _, previous)) = self.sources.get(&source).copied() {
                delete_interest(&self.poller, source, token, previous)?;
            }
            apply_interest(
                &self.poller,
                source,
                token,
                interest,
                kqueue::EventFlags::ADD | kqueue::EventFlags::ENABLE | kqueue::EventFlags::CLEAR,
            )?;
            self.sources.insert(source, token, kind, interest);
            Ok(())
        }

        fn deregister(&mut self, source: SourceRef, token: Token) -> io::Result<()> {
            if let Some((_, _, interest)) = self.sources.remove(&source) {
                delete_interest(&self.poller, source, token, interest)?;
            }
            Ok(())
        }

        fn poll(&mut self, timeout: PollTimeout) -> io::Result<Vec<NativeEvent>> {
            let raw = self
                .poller
                .wait((self.sources.len().max(1)) * 2, timeout.into_duration())?;
            Ok(raw
                .into_iter()
                .map(|event| {
                    let token = Token(event.token());
                    let source = self.sources.kind_for_token(token);
                    NativeEvent::readiness(
                        token,
                        source,
                        event.is_readable(),
                        event.is_writable(),
                        event.is_error(),
                        event.is_eof(),
                    )
                })
                .collect())
        }
    }

    fn apply_interest(
        poller: &kqueue::Kqueue,
        source: SourceRef,
        token: Token,
        interest: Interest,
        flags: kqueue::EventFlags,
    ) -> io::Result<()> {
        let fd = source.raw() as i32;
        let mut changes = Vec::new();
        if interest.is_readable() {
            changes.push(kqueue::KqueueChange::readable(fd, token.0).with_flags(flags));
        }
        if interest.is_writable() {
            changes.push(kqueue::KqueueChange::writable(fd, token.0).with_flags(flags));
        }
        poller.apply_all(&changes)
    }

    fn delete_interest(
        poller: &kqueue::Kqueue,
        source: SourceRef,
        token: Token,
        interest: Interest,
    ) -> io::Result<()> {
        let fd = source.raw() as i32;
        let mut changes = Vec::new();
        if interest.is_readable() {
            changes.push(
                kqueue::KqueueChange::readable(fd, token.0).with_flags(kqueue::EventFlags::DELETE),
            );
        }
        if interest.is_writable() {
            changes.push(
                kqueue::KqueueChange::writable(fd, token.0).with_flags(kqueue::EventFlags::DELETE),
            );
        }
        poller.apply_all(&changes)
    }
}

#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;

    pub struct IocpBackend {
        poller: iocp::CompletionPort,
        sources: SourceRegistry,
    }

    impl IocpBackend {
        pub fn new(concurrency: u32) -> io::Result<Self> {
            Ok(Self {
                poller: iocp::CompletionPort::new(concurrency)?,
                sources: SourceRegistry::default(),
            })
        }
    }

    impl EventBackend for IocpBackend {
        fn register(
            &mut self,
            source: SourceRef,
            token: Token,
            kind: SourceKind,
            interest: Interest,
        ) -> io::Result<()> {
            self.poller
                .associate_handle(source.raw() as *mut std::ffi::c_void, token.0 as usize)?;
            self.sources.insert(source, token, kind, interest);
            Ok(())
        }

        fn reregister(
            &mut self,
            source: SourceRef,
            token: Token,
            kind: SourceKind,
            interest: Interest,
        ) -> io::Result<()> {
            self.sources.insert(source, token, kind, interest);
            Ok(())
        }

        fn deregister(&mut self, source: SourceRef, _token: Token) -> io::Result<()> {
            self.sources.remove(&source);
            Ok(())
        }

        fn poll(&mut self, timeout: PollTimeout) -> io::Result<Vec<NativeEvent>> {
            let packet = self.poller.get(timeout.into_duration())?;
            let token = Token(packet.completion_key as u64);
            let source = self.sources.kind_for_token(token);
            Ok(vec![NativeEvent::completion(
                token,
                source,
                packet.bytes_transferred,
            )])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeBackend {
        events: Vec<NativeEvent>,
        registrations: Vec<(SourceRef, Token, SourceKind, Interest)>,
    }

    impl EventBackend for FakeBackend {
        fn register(
            &mut self,
            source: SourceRef,
            token: Token,
            kind: SourceKind,
            interest: Interest,
        ) -> io::Result<()> {
            self.registrations.push((source, token, kind, interest));
            Ok(())
        }

        fn reregister(
            &mut self,
            source: SourceRef,
            token: Token,
            kind: SourceKind,
            interest: Interest,
        ) -> io::Result<()> {
            self.registrations.push((source, token, kind, interest));
            Ok(())
        }

        fn deregister(&mut self, _source: SourceRef, _token: Token) -> io::Result<()> {
            Ok(())
        }

        fn poll(&mut self, _timeout: PollTimeout) -> io::Result<Vec<NativeEvent>> {
            Ok(self.events.clone())
        }
    }

    #[test]
    fn interest_combines_readable_and_writable() {
        let interest = Interest::READABLE | Interest::WRITABLE;
        assert!(interest.is_readable());
        assert!(interest.is_writable());
    }

    #[test]
    fn loop_delegates_polling_to_backend() {
        let expected = NativeEvent::readiness(Token(5), SourceKind::Io, true, false, false, false);
        let backend = FakeBackend {
            events: vec![expected],
            registrations: Vec::new(),
        };
        let mut loop_ = NativeEventLoop::new(backend);
        let events = loop_.poll_once(PollTimeout::Immediate).expect("poll");
        assert_eq!(events, vec![expected]);
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
#[cfg(test)]
mod bsd_tests {
    use super::bsd::KqueueBackend;
    use super::*;
    use std::io::Write;
    use std::os::fd::AsRawFd;
    use std::os::unix::net::UnixStream;

    #[test]
    fn kqueue_backend_reports_readable_event_locally() {
        let mut backend = KqueueBackend::new().expect("create backend");
        let (left, mut right) = UnixStream::pair().expect("socket pair");
        backend
            .register(
                SourceRef::from_fd(left.as_raw_fd()),
                Token(44),
                SourceKind::Io,
                Interest::READABLE,
            )
            .expect("register socket");

        right.write_all(b"ping").expect("write");
        let events = backend
            .poll(PollTimeout::After(Duration::from_millis(100)))
            .expect("poll backend");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].token, Token(44));
        assert!(events[0].readable);
        assert_eq!(events[0].source, SourceKind::Io);
    }
}
