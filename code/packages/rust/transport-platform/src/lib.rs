//! # transport-platform
//!
//! Runtime-facing seam between higher transport layers and whatever provider
//! offers sockets, event delivery, timers, and wakeups underneath.
//!
//! The point of this crate is not to erase platform differences completely. The
//! point is to give upper layers one repository-owned contract so they do not
//! talk directly to `epoll_event`, `kevent`, `OVERLAPPED`, or raw fd values.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, IoSlice};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ListenerId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimerId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WakeupId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeEventProvider {
    Kqueue,
    Epoll,
    Iocp,
    ReadinessProbe,
}

impl NativeEventProvider {
    pub const fn is_completion_based(self) -> bool {
        matches!(self, Self::Iocp)
    }

    pub const fn is_readiness_based(self) -> bool {
        matches!(self, Self::Kqueue | Self::Epoll | Self::ReadinessProbe)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformCapabilities {
    pub supports_half_close: bool,
    pub supports_vectored_write: bool,
    pub supports_zero_copy_send: bool,
    pub supports_native_timers: bool,
    pub supports_native_wakeups: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StreamInterest {
    pub readable: bool,
    pub writable: bool,
}

impl StreamInterest {
    pub const fn none() -> Self {
        Self {
            readable: false,
            writable: false,
        }
    }

    pub const fn readable() -> Self {
        Self {
            readable: true,
            writable: false,
        }
    }

    pub const fn readable_writable() -> Self {
        Self {
            readable: true,
            writable: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindAddress {
    Ip(SocketAddr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListenerOptions {
    pub backlog: u32,
    pub reuse_address: bool,
    pub reuse_port: bool,
    pub nodelay_default: bool,
    pub keepalive_default: Option<Duration>,
}

impl Default for ListenerOptions {
    fn default() -> Self {
        Self {
            backlog: 128,
            reuse_address: true,
            reuse_port: false,
            nodelay_default: true,
            keepalive_default: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StreamOptions {
    pub nodelay: Option<bool>,
    pub keepalive: Option<Duration>,
    pub recv_buffer_size: Option<usize>,
    pub send_buffer_size: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseKind {
    ReadClosed,
    WriteClosed,
    FullyClosed,
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceId {
    Listener(ListenerId),
    Stream(StreamId),
    Timer(TimerId),
    Wakeup(WakeupId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformEvent {
    ListenerAcceptReady {
        listener: ListenerId,
    },
    StreamReadable {
        stream: StreamId,
    },
    StreamWritable {
        stream: StreamId,
    },
    StreamClosed {
        stream: StreamId,
        kind: CloseKind,
    },
    TimerExpired {
        timer: TimerId,
    },
    Wakeup {
        wakeup: WakeupId,
    },
    Error {
        resource: ResourceId,
        error: PlatformError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformError {
    Unsupported(&'static str),
    InvalidResource,
    ResourceClosed,
    AddressInUse,
    AddressNotAvailable,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    BrokenPipe,
    TimedOut,
    Interrupted,
    Io(String),
    ProviderFault(String),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(message) => write!(f, "unsupported: {message}"),
            Self::InvalidResource => write!(f, "invalid resource"),
            Self::ResourceClosed => write!(f, "resource closed"),
            Self::AddressInUse => write!(f, "address already in use"),
            Self::AddressNotAvailable => write!(f, "address not available"),
            Self::PermissionDenied => write!(f, "permission denied"),
            Self::ConnectionRefused => write!(f, "connection refused"),
            Self::ConnectionReset => write!(f, "connection reset"),
            Self::BrokenPipe => write!(f, "broken pipe"),
            Self::TimedOut => write!(f, "timed out"),
            Self::Interrupted => write!(f, "interrupted"),
            Self::Io(message) => write!(f, "{message}"),
            Self::ProviderFault(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for PlatformError {}

impl From<io::Error> for PlatformError {
    fn from(value: io::Error) -> Self {
        use io::ErrorKind;
        match value.kind() {
            ErrorKind::AddrInUse => Self::AddressInUse,
            ErrorKind::AddrNotAvailable => Self::AddressNotAvailable,
            ErrorKind::PermissionDenied => Self::PermissionDenied,
            ErrorKind::ConnectionRefused => Self::ConnectionRefused,
            ErrorKind::ConnectionReset => Self::ConnectionReset,
            ErrorKind::BrokenPipe => Self::BrokenPipe,
            ErrorKind::TimedOut => Self::TimedOut,
            ErrorKind::Interrupted => Self::Interrupted,
            ErrorKind::NotFound | ErrorKind::InvalidInput | ErrorKind::InvalidData => {
                Self::InvalidResource
            }
            ErrorKind::Unsupported => {
                Self::Unsupported("operation is unsupported on this platform")
            }
            _ => Self::Io(value.to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptedStream {
    pub stream: StreamId,
    pub peer_addr: SocketAddr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOutcome {
    Read(usize),
    WouldBlock,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    Wrote(usize),
    WouldBlock,
    Closed,
}

pub trait TransportPlatform {
    fn native_event_provider(&self) -> NativeEventProvider {
        NativeEventProvider::ReadinessProbe
    }

    fn capabilities(&self) -> PlatformCapabilities;

    fn bind_listener(
        &mut self,
        address: BindAddress,
        options: ListenerOptions,
    ) -> Result<ListenerId, PlatformError>;

    fn local_addr(&self, listener: ListenerId) -> Result<SocketAddr, PlatformError>;

    fn set_listener_interest(
        &mut self,
        listener: ListenerId,
        readable: bool,
    ) -> Result<(), PlatformError>;

    fn accept(&mut self, listener: ListenerId) -> Result<Option<AcceptedStream>, PlatformError>;

    fn configure_stream(
        &mut self,
        stream: StreamId,
        options: StreamOptions,
    ) -> Result<(), PlatformError>;

    fn set_stream_interest(
        &mut self,
        stream: StreamId,
        interest: StreamInterest,
    ) -> Result<(), PlatformError>;

    fn read(&mut self, stream: StreamId, buffer: &mut [u8]) -> Result<ReadOutcome, PlatformError>;

    fn write(&mut self, stream: StreamId, buffer: &[u8]) -> Result<WriteOutcome, PlatformError>;

    fn write_vectored(
        &mut self,
        stream: StreamId,
        buffers: &[IoSlice<'_>],
    ) -> Result<WriteOutcome, PlatformError>;

    fn shutdown_read(&mut self, stream: StreamId) -> Result<(), PlatformError>;

    fn shutdown_write(&mut self, stream: StreamId) -> Result<(), PlatformError>;

    fn close_stream(&mut self, stream: StreamId) -> Result<(), PlatformError>;

    fn close_listener(&mut self, listener: ListenerId) -> Result<(), PlatformError>;

    fn create_timer(&mut self) -> Result<TimerId, PlatformError>;

    fn arm_timer(&mut self, timer: TimerId, deadline: Instant) -> Result<(), PlatformError>;

    fn disarm_timer(&mut self, timer: TimerId) -> Result<(), PlatformError>;

    fn create_wakeup(&mut self) -> Result<WakeupId, PlatformError>;

    fn wake(&mut self, wakeup: WakeupId) -> Result<(), PlatformError>;

    fn poll(
        &mut self,
        timeout: Option<Duration>,
        output: &mut Vec<PlatformEvent>,
    ) -> Result<(), PlatformError>;
}

#[derive(Debug, Clone, Copy)]
struct ListenerStateDefaults {
    nodelay: bool,
    keepalive: Option<Duration>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct ListenerState {
    socket: TcpListener,
    defaults: ListenerStateDefaults,
    readable_interest: bool,
}

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
#[derive(Debug)]
struct StreamState {
    socket: TcpStream,
    interest: StreamInterest,
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
    use socket2::{Domain, Protocol, SockRef, Socket, TcpKeepalive, Type};
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;

    #[derive(Debug, Clone, Copy)]
    struct TimerState {
        ident: usize,
        armed: bool,
    }

    #[derive(Debug, Clone, Copy)]
    struct WakeupState {
        ident: usize,
    }

    fn io_flags() -> kqueue::EventFlags {
        kqueue::EventFlags::ADD | kqueue::EventFlags::ENABLE | kqueue::EventFlags::CLEAR
    }

    /// `KqueueTransportPlatform` is the first real provider implementation for
    /// the transport seam. It keeps OS-specific event mechanics inside this
    /// module while exposing normalized listener, stream, timer, and wakeup
    /// operations above.
    pub struct KqueueTransportPlatform {
        queue: kqueue::Kqueue,
        next_token: u64,
        listeners: BTreeMap<ListenerId, ListenerState>,
        streams: BTreeMap<StreamId, StreamState>,
        timers: BTreeMap<TimerId, TimerState>,
        wakeups: BTreeMap<WakeupId, WakeupState>,
        resources: BTreeMap<u64, ResourceId>,
    }

    impl KqueueTransportPlatform {
        pub fn new() -> Result<Self, PlatformError> {
            Ok(Self {
                queue: kqueue::Kqueue::new()?,
                next_token: 1,
                listeners: BTreeMap::new(),
                streams: BTreeMap::new(),
                timers: BTreeMap::new(),
                wakeups: BTreeMap::new(),
                resources: BTreeMap::new(),
            })
        }

        fn alloc_token(&mut self) -> u64 {
            let token = self.next_token;
            self.next_token += 1;
            token
        }

        fn register_resource(&mut self, token: u64, resource: ResourceId) {
            self.resources.insert(token, resource);
        }

        fn unregister_resource(&mut self, token: u64) {
            self.resources.remove(&token);
        }

        fn resource_for_token(&self, token: u64) -> Option<ResourceId> {
            self.resources.get(&token).copied()
        }

        fn set_socket_reuse_port(socket: &Socket, reuse_port: bool) -> Result<(), PlatformError> {
            if !reuse_port {
                return Ok(());
            }
            let value: libc::c_int = 1;
            let result = unsafe {
                libc::setsockopt(
                    socket.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_REUSEPORT,
                    (&value as *const libc::c_int).cast(),
                    std::mem::size_of_val(&value) as libc::socklen_t,
                )
            };
            if result == 0 {
                Ok(())
            } else {
                Err(PlatformError::from(io::Error::last_os_error()))
            }
        }

        fn socket_domain(address: SocketAddr) -> Domain {
            if address.is_ipv4() {
                Domain::IPV4
            } else {
                Domain::IPV6
            }
        }

        fn apply_listener_interest(
            queue: &kqueue::Kqueue,
            listener: &TcpListener,
            token: ListenerId,
            readable: bool,
        ) -> Result<(), PlatformError> {
            let change = kqueue::KqueueChange::readable(listener.as_raw_fd(), token.0);
            if readable {
                queue.apply(change.with_flags(io_flags()))?;
            } else {
                queue.apply(change.with_flags(kqueue::EventFlags::DELETE))?;
            }
            Ok(())
        }

        fn apply_stream_interest(
            queue: &kqueue::Kqueue,
            stream: &TcpStream,
            token: StreamId,
            previous: StreamInterest,
            interest: StreamInterest,
        ) -> Result<(), PlatformError> {
            if previous.readable && !interest.readable {
                queue.apply(
                    kqueue::KqueueChange::readable(stream.as_raw_fd(), token.0)
                        .with_flags(kqueue::EventFlags::DELETE),
                )?;
            }
            if previous.writable && !interest.writable {
                queue.apply(
                    kqueue::KqueueChange::writable(stream.as_raw_fd(), token.0)
                        .with_flags(kqueue::EventFlags::DELETE),
                )?;
            }
            if interest.readable {
                queue.apply(
                    kqueue::KqueueChange::readable(stream.as_raw_fd(), token.0)
                        .with_flags(io_flags()),
                )?;
            }
            if interest.writable {
                queue.apply(
                    kqueue::KqueueChange::writable(stream.as_raw_fd(), token.0)
                        .with_flags(io_flags()),
                )?;
            }
            Ok(())
        }

        fn configure_stream_defaults(
            stream: &TcpStream,
            defaults: ListenerStateDefaults,
        ) -> Result<(), PlatformError> {
            stream.set_nodelay(defaults.nodelay)?;
            if let Some(duration) = defaults.keepalive {
                let keepalive = TcpKeepalive::new().with_time(duration);
                SockRef::from(stream).set_tcp_keepalive(&keepalive)?;
            }
            Ok(())
        }

        fn map_kqueue_error(
            &self,
            event: kqueue::KqueueEvent,
            resource: ResourceId,
        ) -> PlatformEvent {
            let error = if event.data() > 0 {
                PlatformError::from(io::Error::from_raw_os_error(event.data() as i32))
            } else {
                PlatformError::ProviderFault(format!("kqueue reported EV_ERROR for {:?}", resource))
            };
            PlatformEvent::Error { resource, error }
        }
    }

    impl TransportPlatform for KqueueTransportPlatform {
        fn native_event_provider(&self) -> NativeEventProvider {
            NativeEventProvider::Kqueue
        }

        fn capabilities(&self) -> PlatformCapabilities {
            PlatformCapabilities {
                supports_half_close: true,
                supports_vectored_write: true,
                supports_zero_copy_send: false,
                supports_native_timers: true,
                supports_native_wakeups: true,
            }
        }

        fn bind_listener(
            &mut self,
            address: BindAddress,
            options: ListenerOptions,
        ) -> Result<ListenerId, PlatformError> {
            let BindAddress::Ip(address) = address;
            let socket = Socket::new(
                Self::socket_domain(address),
                Type::STREAM,
                Some(Protocol::TCP),
            )?;
            socket.set_reuse_address(options.reuse_address)?;
            Self::set_socket_reuse_port(&socket, options.reuse_port)?;
            socket.bind(&address.into())?;
            socket.listen(options.backlog.min(i32::MAX as u32) as i32)?;
            socket.set_nonblocking(true)?;

            let listener: TcpListener = socket.into();
            let id = ListenerId(self.alloc_token());
            Self::apply_listener_interest(&self.queue, &listener, id, true)?;
            self.register_resource(id.0, ResourceId::Listener(id));
            self.listeners.insert(
                id,
                ListenerState {
                    socket: listener,
                    defaults: ListenerStateDefaults {
                        nodelay: options.nodelay_default,
                        keepalive: options.keepalive_default,
                    },
                    readable_interest: true,
                },
            );
            Ok(id)
        }

        fn local_addr(&self, listener: ListenerId) -> Result<SocketAddr, PlatformError> {
            self.listeners
                .get(&listener)
                .ok_or(PlatformError::InvalidResource)?
                .socket
                .local_addr()
                .map_err(PlatformError::from)
        }

        fn set_listener_interest(
            &mut self,
            listener: ListenerId,
            readable: bool,
        ) -> Result<(), PlatformError> {
            let state = self
                .listeners
                .get_mut(&listener)
                .ok_or(PlatformError::InvalidResource)?;
            if state.readable_interest == readable {
                return Ok(());
            }
            Self::apply_listener_interest(&self.queue, &state.socket, listener, readable)?;
            state.readable_interest = readable;
            Ok(())
        }

        fn accept(
            &mut self,
            listener: ListenerId,
        ) -> Result<Option<AcceptedStream>, PlatformError> {
            let state = self
                .listeners
                .get(&listener)
                .ok_or(PlatformError::InvalidResource)?;
            match state.socket.accept() {
                Ok((stream, peer_addr)) => {
                    stream.set_nonblocking(true)?;
                    Self::configure_stream_defaults(&stream, state.defaults)?;
                    let id = StreamId(self.alloc_token());
                    self.register_resource(id.0, ResourceId::Stream(id));
                    self.streams.insert(
                        id,
                        StreamState {
                            socket: stream,
                            interest: StreamInterest::none(),
                        },
                    );
                    Ok(Some(AcceptedStream {
                        stream: id,
                        peer_addr,
                    }))
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
                Err(error) => Err(PlatformError::from(error)),
            }
        }

        fn configure_stream(
            &mut self,
            stream: StreamId,
            options: StreamOptions,
        ) -> Result<(), PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;
            if let Some(nodelay) = options.nodelay {
                state.socket.set_nodelay(nodelay)?;
            }
            let socket = SockRef::from(&state.socket);
            if let Some(keepalive) = options.keepalive {
                socket.set_tcp_keepalive(&TcpKeepalive::new().with_time(keepalive))?;
            }
            if let Some(size) = options.recv_buffer_size {
                socket.set_recv_buffer_size(size)?;
            }
            if let Some(size) = options.send_buffer_size {
                socket.set_send_buffer_size(size)?;
            }
            Ok(())
        }

        fn set_stream_interest(
            &mut self,
            stream: StreamId,
            interest: StreamInterest,
        ) -> Result<(), PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;
            let previous = state.interest;
            Self::apply_stream_interest(&self.queue, &state.socket, stream, previous, interest)?;
            state.interest = interest;
            Ok(())
        }

        fn read(
            &mut self,
            stream: StreamId,
            buffer: &mut [u8],
        ) -> Result<ReadOutcome, PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;
            match state.socket.read(buffer) {
                Ok(0) => Ok(ReadOutcome::Closed),
                Ok(n) => Ok(ReadOutcome::Read(n)),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    Ok(ReadOutcome::WouldBlock)
                }
                Err(error) => Err(PlatformError::from(error)),
            }
        }

        fn write(
            &mut self,
            stream: StreamId,
            buffer: &[u8],
        ) -> Result<WriteOutcome, PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;
            match state.socket.write(buffer) {
                Ok(0) => Ok(WriteOutcome::Closed),
                Ok(n) => Ok(WriteOutcome::Wrote(n)),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    Ok(WriteOutcome::WouldBlock)
                }
                Err(error) => Err(PlatformError::from(error)),
            }
        }

        fn write_vectored(
            &mut self,
            stream: StreamId,
            buffers: &[IoSlice<'_>],
        ) -> Result<WriteOutcome, PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;
            match state.socket.write_vectored(buffers) {
                Ok(0) => Ok(WriteOutcome::Closed),
                Ok(n) => Ok(WriteOutcome::Wrote(n)),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    Ok(WriteOutcome::WouldBlock)
                }
                Err(error) => Err(PlatformError::from(error)),
            }
        }

        fn shutdown_read(&mut self, stream: StreamId) -> Result<(), PlatformError> {
            self.streams
                .get(&stream)
                .ok_or(PlatformError::InvalidResource)?
                .socket
                .shutdown(Shutdown::Read)
                .map_err(PlatformError::from)
        }

        fn shutdown_write(&mut self, stream: StreamId) -> Result<(), PlatformError> {
            self.streams
                .get(&stream)
                .ok_or(PlatformError::InvalidResource)?
                .socket
                .shutdown(Shutdown::Write)
                .map_err(PlatformError::from)
        }

        fn close_stream(&mut self, stream: StreamId) -> Result<(), PlatformError> {
            if let Some(state) = self.streams.remove(&stream) {
                Self::apply_stream_interest(
                    &self.queue,
                    &state.socket,
                    stream,
                    state.interest,
                    StreamInterest::none(),
                )?;
                self.unregister_resource(stream.0);
                Ok(())
            } else {
                Err(PlatformError::InvalidResource)
            }
        }

        fn close_listener(&mut self, listener: ListenerId) -> Result<(), PlatformError> {
            if let Some(state) = self.listeners.remove(&listener) {
                if state.readable_interest {
                    Self::apply_listener_interest(&self.queue, &state.socket, listener, false)?;
                }
                self.unregister_resource(listener.0);
                Ok(())
            } else {
                Err(PlatformError::InvalidResource)
            }
        }

        fn create_timer(&mut self) -> Result<TimerId, PlatformError> {
            let id = TimerId(self.alloc_token());
            self.register_resource(id.0, ResourceId::Timer(id));
            self.timers.insert(
                id,
                TimerState {
                    ident: id.0 as usize,
                    armed: false,
                },
            );
            Ok(id)
        }

        fn arm_timer(&mut self, timer: TimerId, deadline: Instant) -> Result<(), PlatformError> {
            let state = self
                .timers
                .get_mut(&timer)
                .ok_or(PlatformError::InvalidResource)?;
            if state.armed {
                self.queue.apply(
                    kqueue::KqueueChange::timer(state.ident, timer.0, 1)
                        .with_flags(kqueue::EventFlags::DELETE),
                )?;
            }
            let delay = deadline.saturating_duration_since(Instant::now());
            let timeout_ms = delay.as_millis().max(1).min(i64::MAX as u128) as u64;
            self.queue.apply(kqueue::KqueueChange::timer(
                state.ident,
                timer.0,
                timeout_ms,
            ))?;
            state.armed = true;
            Ok(())
        }

        fn disarm_timer(&mut self, timer: TimerId) -> Result<(), PlatformError> {
            let state = self
                .timers
                .get_mut(&timer)
                .ok_or(PlatformError::InvalidResource)?;
            if state.armed {
                self.queue.apply(
                    kqueue::KqueueChange::timer(state.ident, timer.0, 1)
                        .with_flags(kqueue::EventFlags::DELETE),
                )?;
                state.armed = false;
            }
            Ok(())
        }

        fn create_wakeup(&mut self) -> Result<WakeupId, PlatformError> {
            let id = WakeupId(self.alloc_token());
            let state = WakeupState {
                ident: id.0 as usize,
            };
            self.queue
                .apply(kqueue::KqueueChange::user(state.ident, id.0))?;
            self.register_resource(id.0, ResourceId::Wakeup(id));
            self.wakeups.insert(id, state);
            Ok(id)
        }

        fn wake(&mut self, wakeup: WakeupId) -> Result<(), PlatformError> {
            let state = self
                .wakeups
                .get(&wakeup)
                .ok_or(PlatformError::InvalidResource)?;
            self.queue.apply(
                kqueue::KqueueChange::user(state.ident, wakeup.0)
                    .with_flags(kqueue::EventFlags::ENABLE | kqueue::EventFlags::CLEAR)
                    .with_fflags(kqueue::note_trigger()),
            )?;
            Ok(())
        }

        fn poll(
            &mut self,
            timeout: Option<Duration>,
            output: &mut Vec<PlatformEvent>,
        ) -> Result<(), PlatformError> {
            output.clear();
            let max_events = (self.listeners.len()
                + self.streams.len() * 2
                + self.timers.len()
                + self.wakeups.len())
            .max(1)
            .min(i32::MAX as usize);
            let events = self.queue.wait(max_events, timeout)?;
            for event in events {
                let Some(resource) = self.resource_for_token(event.token()) else {
                    continue;
                };
                if event.is_error() {
                    output.push(self.map_kqueue_error(event, resource));
                    continue;
                }
                match resource {
                    ResourceId::Listener(listener) => {
                        if event.is_readable() {
                            output.push(PlatformEvent::ListenerAcceptReady { listener });
                        }
                    }
                    ResourceId::Stream(stream) => {
                        if event.is_readable() {
                            output.push(PlatformEvent::StreamReadable { stream });
                        }
                        if event.is_writable() {
                            output.push(PlatformEvent::StreamWritable { stream });
                        }
                        if event.is_eof() {
                            let kind = if event.is_readable() {
                                CloseKind::ReadClosed
                            } else if event.is_writable() {
                                CloseKind::WriteClosed
                            } else {
                                CloseKind::FullyClosed
                            };
                            output.push(PlatformEvent::StreamClosed { stream, kind });
                        }
                    }
                    ResourceId::Timer(timer) => {
                        if event.is_timer() {
                            if let Some(state) = self.timers.get_mut(&timer) {
                                state.armed = false;
                            }
                            output.push(PlatformEvent::TimerExpired { timer });
                        }
                    }
                    ResourceId::Wakeup(wakeup) => {
                        if event.is_user() {
                            output.push(PlatformEvent::Wakeup { wakeup });
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;
    use socket2::{Domain, Protocol, SockRef, Socket, TcpKeepalive, Type};
    use std::io::{Read, Write};
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    #[derive(Debug)]
    struct LinuxTimerState {
        fd: OwnedFd,
        armed: bool,
    }

    #[derive(Debug)]
    struct LinuxWakeupState {
        fd: OwnedFd,
    }

    /// `EpollTransportPlatform` is the Linux transport-provider implementation.
    /// It uses `epoll` for socket readiness, `timerfd` for deadline events, and
    /// `eventfd` for explicit wakeups.
    pub struct EpollTransportPlatform {
        poller: epoll::Epoll,
        next_token: u64,
        listeners: BTreeMap<ListenerId, ListenerState>,
        streams: BTreeMap<StreamId, StreamState>,
        timers: BTreeMap<TimerId, LinuxTimerState>,
        wakeups: BTreeMap<WakeupId, LinuxWakeupState>,
        resources: BTreeMap<u64, ResourceId>,
    }

    impl EpollTransportPlatform {
        pub fn new() -> Result<Self, PlatformError> {
            Ok(Self {
                poller: epoll::Epoll::new(true)?,
                next_token: 1,
                listeners: BTreeMap::new(),
                streams: BTreeMap::new(),
                timers: BTreeMap::new(),
                wakeups: BTreeMap::new(),
                resources: BTreeMap::new(),
            })
        }

        fn alloc_token(&mut self) -> u64 {
            let token = self.next_token;
            self.next_token += 1;
            token
        }

        fn register_resource(&mut self, token: u64, resource: ResourceId) {
            self.resources.insert(token, resource);
        }

        fn unregister_resource(&mut self, token: u64) {
            self.resources.remove(&token);
        }

        fn resource_for_token(&self, token: u64) -> Option<ResourceId> {
            self.resources.get(&token).copied()
        }

        fn socket_domain(address: SocketAddr) -> Domain {
            if address.is_ipv4() {
                Domain::IPV4
            } else {
                Domain::IPV6
            }
        }

        fn configure_listener_socket(
            socket: &Socket,
            address: SocketAddr,
            options: ListenerOptions,
        ) -> Result<(), PlatformError> {
            socket.set_reuse_address(options.reuse_address)?;
            if address.is_ipv6() {
                socket.set_only_v6(true)?;
            }
            Self::set_socket_reuse_port(socket, options.reuse_port)?;
            Ok(())
        }

        fn set_socket_reuse_port(socket: &Socket, reuse_port: bool) -> Result<(), PlatformError> {
            if !reuse_port {
                return Ok(());
            }
            let value: libc::c_int = 1;
            let result = unsafe {
                libc::setsockopt(
                    socket.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_REUSEPORT,
                    (&value as *const libc::c_int).cast(),
                    std::mem::size_of_val(&value) as libc::socklen_t,
                )
            };
            if result == 0 {
                Ok(())
            } else {
                Err(PlatformError::from(io::Error::last_os_error()))
            }
        }

        fn configure_stream_defaults(
            stream: &TcpStream,
            defaults: ListenerStateDefaults,
        ) -> Result<(), PlatformError> {
            stream.set_nodelay(defaults.nodelay)?;
            if let Some(duration) = defaults.keepalive {
                let keepalive = TcpKeepalive::new().with_time(duration);
                SockRef::from(stream).set_tcp_keepalive(&keepalive)?;
            }
            Ok(())
        }

        fn create_eventfd() -> Result<OwnedFd, PlatformError> {
            let raw = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
            if raw == -1 {
                Err(PlatformError::from(io::Error::last_os_error()))
            } else {
                Ok(unsafe { OwnedFd::from_raw_fd(raw) })
            }
        }

        fn create_timerfd() -> Result<OwnedFd, PlatformError> {
            let raw = unsafe {
                libc::timerfd_create(
                    libc::CLOCK_MONOTONIC,
                    libc::TFD_CLOEXEC | libc::TFD_NONBLOCK,
                )
            };
            if raw == -1 {
                Err(PlatformError::from(io::Error::last_os_error()))
            } else {
                Ok(unsafe { OwnedFd::from_raw_fd(raw) })
            }
        }

        fn drain_counter_fd(fd: &OwnedFd) -> Result<(), PlatformError> {
            let mut buffer = 0u64;
            let read = unsafe {
                libc::read(
                    fd.as_raw_fd(),
                    (&mut buffer as *mut u64).cast(),
                    std::mem::size_of::<u64>(),
                )
            };
            if read == -1 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::WouldBlock {
                    Ok(())
                } else {
                    Err(PlatformError::from(error))
                }
            } else {
                Ok(())
            }
        }

        fn write_counter_fd(fd: &OwnedFd, value: u64) -> Result<(), PlatformError> {
            let written = unsafe {
                libc::write(
                    fd.as_raw_fd(),
                    (&value as *const u64).cast(),
                    std::mem::size_of::<u64>(),
                )
            };
            if written == -1 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::WouldBlock {
                    Ok(())
                } else {
                    Err(PlatformError::from(error))
                }
            } else {
                Ok(())
            }
        }

        fn apply_listener_interest(
            poller: &epoll::Epoll,
            listener: &TcpListener,
            token: ListenerId,
            readable: bool,
        ) -> Result<(), PlatformError> {
            if readable {
                poller.add(
                    listener.as_raw_fd(),
                    epoll::EpollEvent::new(token.0, epoll::Interest::READABLE),
                )?;
            } else {
                poller.delete(listener.as_raw_fd())?;
            }
            Ok(())
        }

        fn apply_stream_interest(
            poller: &epoll::Epoll,
            stream: &TcpStream,
            token: StreamId,
            previous: StreamInterest,
            interest: StreamInterest,
        ) -> Result<(), PlatformError> {
            match (
                previous.readable || previous.writable,
                interest.readable || interest.writable,
            ) {
                (false, false) => Ok(()),
                (false, true) => {
                    poller.add(
                        stream.as_raw_fd(),
                        epoll::EpollEvent::new(token.0, to_epoll_interest(interest)),
                    )?;
                    Ok(())
                }
                (true, false) => {
                    poller.delete(stream.as_raw_fd())?;
                    Ok(())
                }
                (true, true) => {
                    poller.modify(
                        stream.as_raw_fd(),
                        epoll::EpollEvent::new(token.0, to_epoll_interest(interest)),
                    )?;
                    Ok(())
                }
            }
        }

        fn arm_timer_fd(fd: &OwnedFd, deadline: Instant) -> Result<(), PlatformError> {
            let delay = deadline.saturating_duration_since(Instant::now());
            let spec = libc::itimerspec {
                it_interval: libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                it_value: duration_to_timespec(delay),
            };
            let result =
                unsafe { libc::timerfd_settime(fd.as_raw_fd(), 0, &spec, std::ptr::null_mut()) };
            if result == -1 {
                Err(PlatformError::from(io::Error::last_os_error()))
            } else {
                Ok(())
            }
        }

        fn disarm_timer_fd(fd: &OwnedFd) -> Result<(), PlatformError> {
            let spec = libc::itimerspec {
                it_interval: libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
                it_value: libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                },
            };
            let result =
                unsafe { libc::timerfd_settime(fd.as_raw_fd(), 0, &spec, std::ptr::null_mut()) };
            if result == -1 {
                Err(PlatformError::from(io::Error::last_os_error()))
            } else {
                Ok(())
            }
        }
    }

    impl TransportPlatform for EpollTransportPlatform {
        fn native_event_provider(&self) -> NativeEventProvider {
            NativeEventProvider::Epoll
        }

        fn capabilities(&self) -> PlatformCapabilities {
            PlatformCapabilities {
                supports_half_close: true,
                supports_vectored_write: true,
                supports_zero_copy_send: false,
                supports_native_timers: true,
                supports_native_wakeups: true,
            }
        }

        fn bind_listener(
            &mut self,
            address: BindAddress,
            options: ListenerOptions,
        ) -> Result<ListenerId, PlatformError> {
            let BindAddress::Ip(address) = address;
            let socket = Socket::new(
                Self::socket_domain(address),
                Type::STREAM,
                Some(Protocol::TCP),
            )?;
            Self::configure_listener_socket(&socket, address, options)?;
            socket.bind(&address.into())?;
            socket.listen(options.backlog.min(i32::MAX as u32) as i32)?;
            socket.set_nonblocking(true)?;

            let listener: TcpListener = socket.into();
            let id = ListenerId(self.alloc_token());
            Self::apply_listener_interest(&self.poller, &listener, id, true)?;
            self.register_resource(id.0, ResourceId::Listener(id));
            self.listeners.insert(
                id,
                ListenerState {
                    socket: listener,
                    defaults: ListenerStateDefaults {
                        nodelay: options.nodelay_default,
                        keepalive: options.keepalive_default,
                    },
                    readable_interest: true,
                },
            );
            Ok(id)
        }

        fn local_addr(&self, listener: ListenerId) -> Result<SocketAddr, PlatformError> {
            self.listeners
                .get(&listener)
                .ok_or(PlatformError::InvalidResource)?
                .socket
                .local_addr()
                .map_err(PlatformError::from)
        }

        fn set_listener_interest(
            &mut self,
            listener: ListenerId,
            readable: bool,
        ) -> Result<(), PlatformError> {
            let state = self
                .listeners
                .get_mut(&listener)
                .ok_or(PlatformError::InvalidResource)?;
            if state.readable_interest == readable {
                return Ok(());
            }
            if readable {
                self.poller.add(
                    state.socket.as_raw_fd(),
                    epoll::EpollEvent::new(listener.0, epoll::Interest::READABLE),
                )?;
            } else {
                self.poller.delete(state.socket.as_raw_fd())?;
            }
            state.readable_interest = readable;
            Ok(())
        }

        fn accept(
            &mut self,
            listener: ListenerId,
        ) -> Result<Option<AcceptedStream>, PlatformError> {
            let state = self
                .listeners
                .get(&listener)
                .ok_or(PlatformError::InvalidResource)?;
            match state.socket.accept() {
                Ok((stream, peer_addr)) => {
                    stream.set_nonblocking(true)?;
                    Self::configure_stream_defaults(&stream, state.defaults)?;
                    let id = StreamId(self.alloc_token());
                    self.register_resource(id.0, ResourceId::Stream(id));
                    self.streams.insert(
                        id,
                        StreamState {
                            socket: stream,
                            interest: StreamInterest::none(),
                        },
                    );
                    Ok(Some(AcceptedStream {
                        stream: id,
                        peer_addr,
                    }))
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
                Err(error) => Err(PlatformError::from(error)),
            }
        }

        fn configure_stream(
            &mut self,
            stream: StreamId,
            options: StreamOptions,
        ) -> Result<(), PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;
            if let Some(nodelay) = options.nodelay {
                state.socket.set_nodelay(nodelay)?;
            }
            let socket = SockRef::from(&state.socket);
            if let Some(keepalive) = options.keepalive {
                socket.set_tcp_keepalive(&TcpKeepalive::new().with_time(keepalive))?;
            }
            if let Some(size) = options.recv_buffer_size {
                socket.set_recv_buffer_size(size)?;
            }
            if let Some(size) = options.send_buffer_size {
                socket.set_send_buffer_size(size)?;
            }
            Ok(())
        }

        fn set_stream_interest(
            &mut self,
            stream: StreamId,
            interest: StreamInterest,
        ) -> Result<(), PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;
            let previous = state.interest;
            Self::apply_stream_interest(&self.poller, &state.socket, stream, previous, interest)?;
            state.interest = interest;
            Ok(())
        }

        fn read(
            &mut self,
            stream: StreamId,
            buffer: &mut [u8],
        ) -> Result<ReadOutcome, PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;
            match state.socket.read(buffer) {
                Ok(0) => Ok(ReadOutcome::Closed),
                Ok(n) => Ok(ReadOutcome::Read(n)),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    Ok(ReadOutcome::WouldBlock)
                }
                Err(error) => Err(PlatformError::from(error)),
            }
        }

        fn write(
            &mut self,
            stream: StreamId,
            buffer: &[u8],
        ) -> Result<WriteOutcome, PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;
            match state.socket.write(buffer) {
                Ok(0) => Ok(WriteOutcome::Closed),
                Ok(n) => Ok(WriteOutcome::Wrote(n)),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    Ok(WriteOutcome::WouldBlock)
                }
                Err(error) => Err(PlatformError::from(error)),
            }
        }

        fn write_vectored(
            &mut self,
            stream: StreamId,
            buffers: &[IoSlice<'_>],
        ) -> Result<WriteOutcome, PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;
            match state.socket.write_vectored(buffers) {
                Ok(0) => Ok(WriteOutcome::Closed),
                Ok(n) => Ok(WriteOutcome::Wrote(n)),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    Ok(WriteOutcome::WouldBlock)
                }
                Err(error) => Err(PlatformError::from(error)),
            }
        }

        fn shutdown_read(&mut self, stream: StreamId) -> Result<(), PlatformError> {
            self.streams
                .get(&stream)
                .ok_or(PlatformError::InvalidResource)?
                .socket
                .shutdown(Shutdown::Read)
                .map_err(PlatformError::from)
        }

        fn shutdown_write(&mut self, stream: StreamId) -> Result<(), PlatformError> {
            self.streams
                .get(&stream)
                .ok_or(PlatformError::InvalidResource)?
                .socket
                .shutdown(Shutdown::Write)
                .map_err(PlatformError::from)
        }

        fn close_stream(&mut self, stream: StreamId) -> Result<(), PlatformError> {
            if let Some(state) = self.streams.remove(&stream) {
                Self::apply_stream_interest(
                    &self.poller,
                    &state.socket,
                    stream,
                    state.interest,
                    StreamInterest::none(),
                )?;
                self.unregister_resource(stream.0);
                Ok(())
            } else {
                Err(PlatformError::InvalidResource)
            }
        }

        fn close_listener(&mut self, listener: ListenerId) -> Result<(), PlatformError> {
            if let Some(state) = self.listeners.remove(&listener) {
                if state.readable_interest {
                    self.poller.delete(state.socket.as_raw_fd())?;
                }
                self.unregister_resource(listener.0);
                Ok(())
            } else {
                Err(PlatformError::InvalidResource)
            }
        }

        fn create_timer(&mut self) -> Result<TimerId, PlatformError> {
            let id = TimerId(self.alloc_token());
            let fd = Self::create_timerfd()?;
            self.poller.add(
                fd.as_raw_fd(),
                epoll::EpollEvent::new(id.0, epoll::Interest::READABLE),
            )?;
            self.register_resource(id.0, ResourceId::Timer(id));
            self.timers.insert(id, LinuxTimerState { fd, armed: false });
            Ok(id)
        }

        fn arm_timer(&mut self, timer: TimerId, deadline: Instant) -> Result<(), PlatformError> {
            let state = self
                .timers
                .get_mut(&timer)
                .ok_or(PlatformError::InvalidResource)?;
            Self::arm_timer_fd(&state.fd, deadline)?;
            state.armed = true;
            Ok(())
        }

        fn disarm_timer(&mut self, timer: TimerId) -> Result<(), PlatformError> {
            let state = self
                .timers
                .get_mut(&timer)
                .ok_or(PlatformError::InvalidResource)?;
            Self::disarm_timer_fd(&state.fd)?;
            state.armed = false;
            Ok(())
        }

        fn create_wakeup(&mut self) -> Result<WakeupId, PlatformError> {
            let id = WakeupId(self.alloc_token());
            let fd = Self::create_eventfd()?;
            self.poller.add(
                fd.as_raw_fd(),
                epoll::EpollEvent::new(id.0, epoll::Interest::READABLE),
            )?;
            self.register_resource(id.0, ResourceId::Wakeup(id));
            self.wakeups.insert(id, LinuxWakeupState { fd });
            Ok(id)
        }

        fn wake(&mut self, wakeup: WakeupId) -> Result<(), PlatformError> {
            let state = self
                .wakeups
                .get(&wakeup)
                .ok_or(PlatformError::InvalidResource)?;
            Self::write_counter_fd(&state.fd, 1)
        }

        fn poll(
            &mut self,
            timeout: Option<Duration>,
            output: &mut Vec<PlatformEvent>,
        ) -> Result<(), PlatformError> {
            output.clear();
            let max_events = (self.listeners.len()
                + self.streams.len()
                + self.timers.len()
                + self.wakeups.len())
            .max(1);
            let events = self.poller.wait(max_events, timeout)?;
            for event in events {
                let Some(resource) = self.resource_for_token(event.token()) else {
                    continue;
                };
                match resource {
                    ResourceId::Listener(listener) => {
                        if event.is_error() {
                            output.push(PlatformEvent::Error {
                                resource,
                                error: PlatformError::ProviderFault(format!(
                                    "epoll listener error bits=0x{:x}",
                                    event.bits()
                                )),
                            });
                        } else if event.is_readable() {
                            output.push(PlatformEvent::ListenerAcceptReady { listener });
                        }
                    }
                    ResourceId::Stream(stream) => {
                        if event.is_error() {
                            output.push(PlatformEvent::Error {
                                resource,
                                error: PlatformError::ProviderFault(format!(
                                    "epoll stream error bits=0x{:x}",
                                    event.bits()
                                )),
                            });
                        }
                        if event.is_readable() {
                            output.push(PlatformEvent::StreamReadable { stream });
                        }
                        if event.is_writable() {
                            output.push(PlatformEvent::StreamWritable { stream });
                        }
                        if event.is_hangup() {
                            output.push(PlatformEvent::StreamClosed {
                                stream,
                                kind: CloseKind::FullyClosed,
                            });
                        }
                    }
                    ResourceId::Timer(timer) => {
                        if event.is_readable() {
                            if let Some(state) = self.timers.get_mut(&timer) {
                                Self::drain_counter_fd(&state.fd)?;
                                state.armed = false;
                            }
                            output.push(PlatformEvent::TimerExpired { timer });
                        }
                    }
                    ResourceId::Wakeup(wakeup) => {
                        if event.is_readable() {
                            if let Some(state) = self.wakeups.get(&wakeup) {
                                Self::drain_counter_fd(&state.fd)?;
                            }
                            output.push(PlatformEvent::Wakeup { wakeup });
                        }
                    }
                }
            }
            Ok(())
        }
    }

    fn to_epoll_interest(interest: StreamInterest) -> epoll::Interest {
        match (interest.readable, interest.writable) {
            (true, true) => epoll::Interest::READABLE | epoll::Interest::WRITABLE,
            (true, false) => epoll::Interest::READABLE,
            (false, true) => epoll::Interest::WRITABLE,
            (false, false) => epoll::Interest::READABLE,
        }
    }

    fn duration_to_timespec(duration: Duration) -> libc::timespec {
        libc::timespec {
            tv_sec: duration.as_secs().min(i64::MAX as u64) as libc::time_t,
            tv_nsec: duration.subsec_nanos() as libc::c_long,
        }
    }
}

#[cfg(not(target_os = "linux"))]
pub mod linux {
    use super::*;

    pub struct EpollTransportPlatform;

    impl EpollTransportPlatform {
        pub fn new() -> Result<Self, PlatformError> {
            Err(PlatformError::Unsupported(
                "epoll transport platform is only available on Linux",
            ))
        }
    }
}

#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;
    use socket2::{Domain, Protocol, SockRef, Socket, TcpKeepalive, Type};
    use std::collections::VecDeque;
    use std::ffi::c_void;
    use std::io::ErrorKind;
    use std::os::windows::io::AsRawSocket;
    use std::ptr;
    use windows_sys::Win32::Networking::WinSock::{WSAStartup, WSADATA};

    const WINDOWS_IO_BUFFER_SIZE: usize = 64 * 1024;
    const WINDOWS_ACCEPTEX_ADDRESS_SIZE: usize = 128 + 16;
    const WINDOWS_MAX_PENDING_ACCEPTS: usize = 1024;
    const SOCKET_ERROR: i32 = -1;
    const WSA_IO_PENDING: i32 = 997;
    const SIO_GET_EXTENSION_FUNCTION_POINTER: Dword = 0xC800_0006;
    const SOL_SOCKET: i32 = 0xffff;
    const SO_UPDATE_ACCEPT_CONTEXT: i32 = 0x700b;

    type RawSocketValue = usize;
    type Dword = u32;

    type AcceptExFn = unsafe extern "system" fn(
        listen_socket: RawSocketValue,
        accept_socket: RawSocketValue,
        output_buffer: *mut c_void,
        receive_data_length: Dword,
        local_address_length: Dword,
        remote_address_length: Dword,
        bytes_received: *mut Dword,
        overlapped: *mut iocp::Overlapped,
    ) -> i32;

    #[repr(C)]
    struct WsaBuf {
        len: u32,
        buf: *mut i8,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    const WSAID_ACCEPTEX: Guid = Guid {
        data1: 0xb5367df1,
        data2: 0xcbac,
        data3: 0x11cf,
        data4: [0x95, 0xca, 0x00, 0x80, 0x5f, 0x48, 0xa1, 0x92],
    };

    #[link(name = "Ws2_32")]
    unsafe extern "system" {
        fn WSAIoctl(
            socket: RawSocketValue,
            io_control_code: Dword,
            in_buffer: *mut c_void,
            in_buffer_size: Dword,
            out_buffer: *mut c_void,
            out_buffer_size: Dword,
            bytes_returned: *mut Dword,
            overlapped: *mut iocp::Overlapped,
            completion_routine: *mut c_void,
        ) -> i32;
        fn WSARecv(
            socket: RawSocketValue,
            buffers: *mut WsaBuf,
            buffer_count: Dword,
            bytes_received: *mut Dword,
            flags: *mut Dword,
            overlapped: *mut iocp::Overlapped,
            completion_routine: *mut c_void,
        ) -> i32;
        fn WSASend(
            socket: RawSocketValue,
            buffers: *mut WsaBuf,
            buffer_count: Dword,
            bytes_sent: *mut Dword,
            flags: Dword,
            overlapped: *mut iocp::Overlapped,
            completion_routine: *mut c_void,
        ) -> i32;
        fn setsockopt(
            socket: RawSocketValue,
            level: i32,
            optname: i32,
            optval: *const i8,
            optlen: i32,
        ) -> i32;
        fn WSAGetLastError() -> i32;
    }

    #[derive(Debug)]
    struct WindowsCompletedAccept {
        stream: TcpStream,
        peer_addr: SocketAddr,
    }

    #[derive(Debug)]
    struct WindowsListenerState {
        socket: TcpListener,
        defaults: ListenerStateDefaults,
        readable_interest: bool,
        domain: Domain,
        accept_ex: AcceptExFn,
        accept_queue_depth: usize,
        pending_accepts: usize,
        completed_accepts: VecDeque<WindowsCompletedAccept>,
    }

    #[derive(Debug)]
    struct WindowsTimerState {
        deadline: Option<Instant>,
    }

    #[derive(Debug)]
    struct WindowsWakeupState;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum WindowsOperationKind {
        Read,
        Write,
    }

    #[repr(C)]
    #[derive(Debug)]
    struct WindowsOverlappedOperation {
        overlapped: iocp::Overlapped,
        stream: StreamId,
        kind: WindowsOperationKind,
        buffer: Vec<u8>,
    }

    unsafe impl Send for WindowsOverlappedOperation {}

    impl WindowsOverlappedOperation {
        fn read(stream: StreamId, buffer_size: usize) -> Self {
            Self {
                overlapped: zeroed_overlapped(),
                stream,
                kind: WindowsOperationKind::Read,
                buffer: vec![0; buffer_size.max(1)],
            }
        }

        fn write(stream: StreamId, buffer: Vec<u8>) -> Self {
            Self {
                overlapped: zeroed_overlapped(),
                stream,
                kind: WindowsOperationKind::Write,
                buffer,
            }
        }

        fn key(&mut self) -> usize {
            (&mut self.overlapped as *mut iocp::Overlapped) as usize
        }
    }

    #[repr(C)]
    #[derive(Debug)]
    struct WindowsAcceptOperation {
        overlapped: iocp::Overlapped,
        listener: ListenerId,
        socket: Option<Socket>,
        buffer: Vec<u8>,
    }

    unsafe impl Send for WindowsAcceptOperation {}

    impl WindowsAcceptOperation {
        fn new(listener: ListenerId, socket: Socket) -> Self {
            Self {
                overlapped: zeroed_overlapped(),
                listener,
                socket: Some(socket),
                buffer: vec![0; WINDOWS_ACCEPTEX_ADDRESS_SIZE * 2],
            }
        }

        fn key(&mut self) -> usize {
            (&mut self.overlapped as *mut iocp::Overlapped) as usize
        }
    }

    #[derive(Debug)]
    struct WindowsStreamState {
        socket: TcpStream,
        interest: StreamInterest,
        read_buffer_size: usize,
        pending_read: Option<usize>,
        completed_reads: VecDeque<Vec<u8>>,
        pending_write: Option<usize>,
        completed_writes: VecDeque<usize>,
        read_closed: bool,
    }

    /// `WindowsTransportPlatform` owns the Windows completion-port provider.
    /// Sockets are associated with IOCP as they enter the runtime; wakeups use
    /// posted completion packets, and stream reads/writes keep the same
    /// nonblocking contract that `stream-reactor` already consumes.
    pub struct WindowsTransportPlatform {
        completion_port: iocp::CompletionPort,
        next_token: u64,
        listeners: BTreeMap<ListenerId, WindowsListenerState>,
        streams: BTreeMap<StreamId, WindowsStreamState>,
        timers: BTreeMap<TimerId, WindowsTimerState>,
        wakeups: BTreeMap<WakeupId, WindowsWakeupState>,
        resources: BTreeMap<u64, ResourceId>,
        accept_operations: BTreeMap<usize, Box<WindowsAcceptOperation>>,
        operations: BTreeMap<usize, Box<WindowsOverlappedOperation>>,
    }

    pub type IocpTransportPlatform = WindowsTransportPlatform;

    impl WindowsTransportPlatform {
        pub fn new() -> Result<Self, PlatformError> {
            let mut data = std::mem::MaybeUninit::<WSADATA>::uninit();
            let result = unsafe { WSAStartup(0x0202, data.as_mut_ptr()) };
            if result != 0 {
                return Err(PlatformError::from(io::Error::from_raw_os_error(result)));
            }

            Ok(Self {
                completion_port: iocp::CompletionPort::new(0)?,
                next_token: 1,
                listeners: BTreeMap::new(),
                streams: BTreeMap::new(),
                timers: BTreeMap::new(),
                wakeups: BTreeMap::new(),
                resources: BTreeMap::new(),
                accept_operations: BTreeMap::new(),
                operations: BTreeMap::new(),
            })
        }

        fn alloc_token(&mut self) -> u64 {
            let token = self.next_token;
            self.next_token += 1;
            token
        }

        fn register_resource(&mut self, token: u64, resource: ResourceId) {
            self.resources.insert(token, resource);
        }

        fn unregister_resource(&mut self, token: u64) {
            self.resources.remove(&token);
        }

        fn resource_for_token(&self, token: u64) -> Option<ResourceId> {
            self.resources.get(&token).copied()
        }

        fn socket_domain(address: SocketAddr) -> Domain {
            if address.is_ipv4() {
                Domain::IPV4
            } else {
                Domain::IPV6
            }
        }

        fn configure_listener_socket(
            socket: &Socket,
            address: SocketAddr,
            options: ListenerOptions,
        ) -> Result<(), PlatformError> {
            socket.set_reuse_address(options.reuse_address)?;
            if address.is_ipv6() {
                socket.set_only_v6(true)?;
            }
            if options.reuse_port {
                return Err(PlatformError::Unsupported(
                    "SO_REUSEPORT is not supported by the Windows TCP provider",
                ));
            }
            Ok(())
        }

        fn load_accept_ex(socket: RawSocketValue) -> Result<AcceptExFn, PlatformError> {
            let mut guid = WSAID_ACCEPTEX;
            let mut accept_ex = std::mem::MaybeUninit::<AcceptExFn>::uninit();
            let mut bytes = 0;
            let result = unsafe {
                WSAIoctl(
                    socket,
                    SIO_GET_EXTENSION_FUNCTION_POINTER,
                    (&mut guid as *mut Guid).cast(),
                    std::mem::size_of::<Guid>() as Dword,
                    accept_ex.as_mut_ptr().cast(),
                    std::mem::size_of::<AcceptExFn>() as Dword,
                    &mut bytes,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            };
            if result == SOCKET_ERROR {
                return Err(PlatformError::from(io::Error::from_raw_os_error(unsafe {
                    WSAGetLastError()
                })));
            }
            Ok(unsafe { accept_ex.assume_init() })
        }

        fn associate_listener(
            &self,
            listener: &TcpListener,
            listener_id: ListenerId,
        ) -> Result<(), PlatformError> {
            self.completion_port
                .associate_handle(
                    raw_socket_handle(listener.as_raw_socket()),
                    listener_id.0 as usize,
                )
                .map_err(PlatformError::from)
        }

        fn associate_stream(
            &self,
            stream: &TcpStream,
            stream_id: StreamId,
        ) -> Result<(), PlatformError> {
            self.completion_port
                .associate_handle(
                    raw_socket_handle(stream.as_raw_socket()),
                    stream_id.0 as usize,
                )
                .map_err(PlatformError::from)
        }

        fn queue_accept(&mut self, listener: ListenerId) -> Result<(), PlatformError> {
            let Some(state) = self.listeners.get(&listener) else {
                return Err(PlatformError::InvalidResource);
            };
            if !state.readable_interest {
                return Ok(());
            }

            let accept_socket = Socket::new(state.domain, Type::STREAM, Some(Protocol::TCP))?;
            accept_socket.set_nonblocking(true)?;
            let listen_socket = raw_socket_value(state.socket.as_raw_socket());
            let accept_socket_raw = raw_socket_value(accept_socket.as_raw_socket());
            let accept_ex = state.accept_ex;

            let mut operation = Box::new(WindowsAcceptOperation::new(listener, accept_socket));
            let mut bytes = 0;
            let result = unsafe {
                accept_ex(
                    listen_socket,
                    accept_socket_raw,
                    operation.buffer.as_mut_ptr().cast(),
                    0,
                    WINDOWS_ACCEPTEX_ADDRESS_SIZE as Dword,
                    WINDOWS_ACCEPTEX_ADDRESS_SIZE as Dword,
                    &mut bytes,
                    &mut operation.overlapped,
                )
            };
            if result == SOCKET_ERROR {
                let error = unsafe { WSAGetLastError() };
                if error != WSA_IO_PENDING {
                    return Err(PlatformError::from(io::Error::from_raw_os_error(error)));
                }
            }

            let key = operation.key();
            self.accept_operations.insert(key, operation);
            if let Some(state) = self.listeners.get_mut(&listener) {
                state.pending_accepts += 1;
            }
            Ok(())
        }

        fn replenish_accepts(&mut self, listener: ListenerId) -> Result<(), PlatformError> {
            loop {
                let Some(state) = self.listeners.get(&listener) else {
                    return Ok(());
                };
                if !state.readable_interest || state.pending_accepts >= state.accept_queue_depth {
                    return Ok(());
                }
                self.queue_accept(listener)?;
            }
        }

        fn replenish_all_accepts(&mut self) -> Result<(), PlatformError> {
            let listeners: Vec<_> = self.listeners.keys().copied().collect();
            for listener in listeners {
                self.replenish_accepts(listener)?;
            }
            Ok(())
        }

        fn update_accept_context(
            accept_socket: RawSocketValue,
            listen_socket: RawSocketValue,
        ) -> Result<(), PlatformError> {
            let listen_socket_bytes = listen_socket.to_ne_bytes();
            let result = unsafe {
                setsockopt(
                    accept_socket,
                    SOL_SOCKET,
                    SO_UPDATE_ACCEPT_CONTEXT,
                    listen_socket_bytes.as_ptr().cast(),
                    listen_socket_bytes.len() as i32,
                )
            };
            if result == SOCKET_ERROR {
                Err(PlatformError::from(io::Error::from_raw_os_error(unsafe {
                    WSAGetLastError()
                })))
            } else {
                Ok(())
            }
        }

        fn queue_read_if_needed(&mut self, stream: StreamId) -> Result<(), PlatformError> {
            let Some(state) = self.streams.get(&stream) else {
                return Err(PlatformError::InvalidResource);
            };
            if !state.interest.readable
                || state.read_closed
                || state.pending_read.is_some()
                || !state.completed_reads.is_empty()
            {
                return Ok(());
            }

            let socket = raw_socket_value(state.socket.as_raw_socket());
            let mut operation = Box::new(WindowsOverlappedOperation::read(
                stream,
                state.read_buffer_size,
            ));
            let mut wsabuf = WsaBuf {
                len: operation.buffer.len().min(u32::MAX as usize) as u32,
                buf: operation.buffer.as_mut_ptr().cast(),
            };
            let mut bytes = 0;
            let mut flags = 0;
            let result = unsafe {
                WSARecv(
                    socket,
                    &mut wsabuf,
                    1,
                    &mut bytes,
                    &mut flags,
                    &mut operation.overlapped,
                    ptr::null_mut(),
                )
            };
            if result == SOCKET_ERROR {
                let error = unsafe { WSAGetLastError() };
                if error != WSA_IO_PENDING {
                    return Err(PlatformError::from(io::Error::from_raw_os_error(error)));
                }
            }

            let key = operation.key();
            self.operations.insert(key, operation);
            if let Some(state) = self.streams.get_mut(&stream) {
                state.pending_read = Some(key);
            }
            Ok(())
        }

        fn queue_send(&mut self, stream: StreamId, buffer: Vec<u8>) -> Result<(), PlatformError> {
            let Some(state) = self.streams.get(&stream) else {
                return Err(PlatformError::InvalidResource);
            };
            if state.pending_write.is_some() {
                return Ok(());
            }

            let socket = raw_socket_value(state.socket.as_raw_socket());
            let mut operation = Box::new(WindowsOverlappedOperation::write(stream, buffer));
            let mut wsabuf = WsaBuf {
                len: operation.buffer.len().min(u32::MAX as usize) as u32,
                buf: operation.buffer.as_mut_ptr().cast(),
            };
            let mut bytes = 0;
            let result = unsafe {
                WSASend(
                    socket,
                    &mut wsabuf,
                    1,
                    &mut bytes,
                    0,
                    &mut operation.overlapped,
                    ptr::null_mut(),
                )
            };
            if result == SOCKET_ERROR {
                let error = unsafe { WSAGetLastError() };
                if error != WSA_IO_PENDING {
                    return Err(PlatformError::from(io::Error::from_raw_os_error(error)));
                }
            }

            let key = operation.key();
            self.operations.insert(key, operation);
            if let Some(state) = self.streams.get_mut(&stream) {
                state.pending_write = Some(key);
            }
            Ok(())
        }

        fn configure_stream_defaults(
            stream: &TcpStream,
            defaults: ListenerStateDefaults,
        ) -> Result<(), PlatformError> {
            stream.set_nodelay(defaults.nodelay)?;
            if let Some(duration) = defaults.keepalive {
                let keepalive = TcpKeepalive::new().with_time(duration);
                SockRef::from(stream).set_tcp_keepalive(&keepalive)?;
            }
            Ok(())
        }

        fn next_timer_deadline(&self) -> Option<Instant> {
            self.timers
                .values()
                .filter_map(|state| state.deadline)
                .min()
        }

        fn effective_timeout(&self, timeout: Option<Duration>) -> Option<Duration> {
            let timer_timeout = self
                .next_timer_deadline()
                .map(|deadline| deadline.saturating_duration_since(Instant::now()));
            match (timeout, timer_timeout) {
                (Some(left), Some(right)) => Some(left.min(right)),
                (Some(left), None) => Some(left),
                (None, Some(right)) => Some(right),
                (None, None) => None,
            }
        }

        fn collect_due_timers(&mut self, output: &mut Vec<PlatformEvent>) {
            let now = Instant::now();
            for (&timer, state) in &mut self.timers {
                if let Some(deadline) = state.deadline {
                    if deadline <= now {
                        state.deadline = None;
                        output.push(PlatformEvent::TimerExpired { timer });
                    }
                }
            }
        }

        fn wait_for_iocp_packet(
            &mut self,
            timeout: Option<Duration>,
            output: &mut Vec<PlatformEvent>,
        ) -> Result<bool, PlatformError> {
            match self.completion_port.get(timeout) {
                Ok(packet) => {
                    self.push_completion_packet(packet, output);
                    Ok(true)
                }
                Err(error) if error.kind() == ErrorKind::TimedOut => Ok(false),
                Err(error) => Err(PlatformError::from(error)),
            }
        }

        fn drain_iocp_now(&mut self, output: &mut Vec<PlatformEvent>) -> Result<(), PlatformError> {
            while self.wait_for_iocp_packet(Some(Duration::ZERO), output)? {}
            Ok(())
        }

        fn push_completion_packet(
            &mut self,
            packet: iocp::CompletionPacket,
            output: &mut Vec<PlatformEvent>,
        ) {
            if !packet.overlapped.is_null() {
                self.push_overlapped_completion(packet, output);
                return;
            }

            let Some(resource) = self.resource_for_token(packet.completion_key as u64) else {
                return;
            };

            match resource {
                ResourceId::Listener(listener) => {
                    output.push(PlatformEvent::ListenerAcceptReady { listener });
                }
                ResourceId::Stream(stream) => {
                    if let Some(state) = self.streams.get(&stream) {
                        if state.interest.readable {
                            output.push(PlatformEvent::StreamReadable { stream });
                        }
                        if state.interest.writable {
                            output.push(PlatformEvent::StreamWritable { stream });
                        }
                    }
                }
                ResourceId::Timer(timer) => {
                    output.push(PlatformEvent::TimerExpired { timer });
                }
                ResourceId::Wakeup(wakeup) => {
                    output.push(PlatformEvent::Wakeup { wakeup });
                }
            }
        }

        fn push_overlapped_completion(
            &mut self,
            packet: iocp::CompletionPacket,
            output: &mut Vec<PlatformEvent>,
        ) {
            let key = packet.overlapped as usize;
            if self.accept_operations.contains_key(&key) {
                self.push_accept_completion(packet, output);
                return;
            }

            let Some(mut operation) = self.operations.remove(&key) else {
                return;
            };
            let stream = operation.stream;
            let kind = operation.kind;

            let Some(state) = self.streams.get_mut(&stream) else {
                return;
            };

            match kind {
                WindowsOperationKind::Read => {
                    if state.pending_read == Some(key) {
                        state.pending_read = None;
                    }
                    if packet.error.is_some() {
                        state.read_closed = true;
                        output.push(PlatformEvent::StreamClosed {
                            stream,
                            kind: CloseKind::Reset,
                        });
                        return;
                    }
                    if packet.bytes_transferred == 0 {
                        state.read_closed = true;
                        output.push(PlatformEvent::StreamClosed {
                            stream,
                            kind: CloseKind::ReadClosed,
                        });
                        return;
                    }

                    let mut buffer = std::mem::take(&mut operation.buffer);
                    buffer.truncate(packet.bytes_transferred as usize);
                    state.completed_reads.push_back(buffer);
                    output.push(PlatformEvent::StreamReadable { stream });
                }
                WindowsOperationKind::Write => {
                    if state.pending_write == Some(key) {
                        state.pending_write = None;
                    }
                    if packet.error.is_some() {
                        output.push(PlatformEvent::StreamClosed {
                            stream,
                            kind: CloseKind::Reset,
                        });
                        return;
                    }
                    state
                        .completed_writes
                        .push_back(packet.bytes_transferred as usize);
                    output.push(PlatformEvent::StreamWritable { stream });
                }
            }
        }

        fn push_accept_completion(
            &mut self,
            packet: iocp::CompletionPacket,
            output: &mut Vec<PlatformEvent>,
        ) {
            let key = packet.overlapped as usize;
            let Some(mut operation) = self.accept_operations.remove(&key) else {
                return;
            };
            let listener = operation.listener;

            let Some(listener_state) = self.listeners.get_mut(&listener) else {
                return;
            };
            listener_state.pending_accepts = listener_state.pending_accepts.saturating_sub(1);

            if packet.error.is_some() {
                return;
            }

            let Some(socket) = operation.socket.take() else {
                return;
            };
            let listen_socket = raw_socket_value(listener_state.socket.as_raw_socket());
            let accept_socket = raw_socket_value(socket.as_raw_socket());

            if let Err(error) = Self::update_accept_context(accept_socket, listen_socket) {
                output.push(PlatformEvent::Error {
                    resource: ResourceId::Listener(listener),
                    error,
                });
                return;
            }

            let stream: TcpStream = socket.into();
            if let Err(error) = stream.set_nonblocking(true) {
                output.push(PlatformEvent::Error {
                    resource: ResourceId::Listener(listener),
                    error: PlatformError::from(error),
                });
                return;
            }

            let peer_addr = match stream.peer_addr() {
                Ok(peer_addr) => peer_addr,
                Err(error) => {
                    output.push(PlatformEvent::Error {
                        resource: ResourceId::Listener(listener),
                        error: PlatformError::from(error),
                    });
                    return;
                }
            };

            listener_state
                .completed_accepts
                .push_back(WindowsCompletedAccept { stream, peer_addr });
            if listener_state.readable_interest {
                output.push(PlatformEvent::ListenerAcceptReady { listener });
            }
        }
    }

    impl TransportPlatform for WindowsTransportPlatform {
        fn native_event_provider(&self) -> NativeEventProvider {
            NativeEventProvider::Iocp
        }

        fn capabilities(&self) -> PlatformCapabilities {
            PlatformCapabilities {
                supports_half_close: true,
                supports_vectored_write: true,
                supports_zero_copy_send: false,
                supports_native_timers: false,
                supports_native_wakeups: true,
            }
        }

        fn bind_listener(
            &mut self,
            address: BindAddress,
            options: ListenerOptions,
        ) -> Result<ListenerId, PlatformError> {
            let BindAddress::Ip(address) = address;
            let domain = Self::socket_domain(address);
            let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
            Self::configure_listener_socket(&socket, address, options)?;
            socket.bind(&address.into())?;
            socket.listen(options.backlog.min(i32::MAX as u32) as i32)?;
            socket.set_nonblocking(true)?;
            let accept_ex = Self::load_accept_ex(raw_socket_value(socket.as_raw_socket()))?;

            let listener: TcpListener = socket.into();
            let id = ListenerId(self.alloc_token());
            self.associate_listener(&listener, id)?;
            self.register_resource(id.0, ResourceId::Listener(id));
            self.listeners.insert(
                id,
                WindowsListenerState {
                    socket: listener,
                    defaults: ListenerStateDefaults {
                        nodelay: options.nodelay_default,
                        keepalive: options.keepalive_default,
                    },
                    readable_interest: true,
                    domain,
                    accept_ex,
                    accept_queue_depth: (options.backlog as usize)
                        .max(1)
                        .min(WINDOWS_MAX_PENDING_ACCEPTS),
                    pending_accepts: 0,
                    completed_accepts: VecDeque::new(),
                },
            );
            self.replenish_accepts(id)?;
            Ok(id)
        }

        fn local_addr(&self, listener: ListenerId) -> Result<SocketAddr, PlatformError> {
            self.listeners
                .get(&listener)
                .ok_or(PlatformError::InvalidResource)?
                .socket
                .local_addr()
                .map_err(|error| PlatformError::Io(format!("read listener local_addr: {error}")))
        }

        fn set_listener_interest(
            &mut self,
            listener: ListenerId,
            readable: bool,
        ) -> Result<(), PlatformError> {
            self.listeners
                .get_mut(&listener)
                .ok_or(PlatformError::InvalidResource)?
                .readable_interest = readable;
            if readable {
                self.replenish_accepts(listener)?;
            }
            Ok(())
        }

        fn accept(
            &mut self,
            listener: ListenerId,
        ) -> Result<Option<AcceptedStream>, PlatformError> {
            let state = self
                .listeners
                .get_mut(&listener)
                .ok_or(PlatformError::InvalidResource)?;
            let Some(accepted) = state.completed_accepts.pop_front() else {
                return Ok(None);
            };
            let defaults = state.defaults;
            let WindowsCompletedAccept { stream, peer_addr } = accepted;

            Self::configure_stream_defaults(&stream, defaults)?;
            let id = StreamId(self.alloc_token());
            self.associate_stream(&stream, id)?;
            self.register_resource(id.0, ResourceId::Stream(id));
            self.streams.insert(
                id,
                WindowsStreamState {
                    socket: stream,
                    interest: StreamInterest::none(),
                    read_buffer_size: WINDOWS_IO_BUFFER_SIZE,
                    pending_read: None,
                    completed_reads: VecDeque::new(),
                    pending_write: None,
                    completed_writes: VecDeque::new(),
                    read_closed: false,
                },
            );
            Ok(Some(AcceptedStream {
                stream: id,
                peer_addr,
            }))
        }

        fn configure_stream(
            &mut self,
            stream: StreamId,
            options: StreamOptions,
        ) -> Result<(), PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;
            if let Some(nodelay) = options.nodelay {
                state.socket.set_nodelay(nodelay)?;
            }
            let socket = SockRef::from(&state.socket);
            if let Some(keepalive) = options.keepalive {
                socket.set_tcp_keepalive(&TcpKeepalive::new().with_time(keepalive))?;
            }
            if let Some(size) = options.recv_buffer_size {
                socket.set_recv_buffer_size(size)?;
                state.read_buffer_size = size.max(1);
            }
            if let Some(size) = options.send_buffer_size {
                socket.set_send_buffer_size(size)?;
            }
            Ok(())
        }

        fn set_stream_interest(
            &mut self,
            stream: StreamId,
            interest: StreamInterest,
        ) -> Result<(), PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;
            let previous = state.interest;
            state.interest = interest;
            let should_nudge_writable = interest.writable && !previous.writable;
            self.queue_read_if_needed(stream)?;
            if should_nudge_writable {
                self.completion_port
                    .post(0, stream.0 as usize, ptr::null_mut())
                    .map_err(PlatformError::from)?;
            }
            Ok(())
        }

        fn read(
            &mut self,
            stream: StreamId,
            buffer: &mut [u8],
        ) -> Result<ReadOutcome, PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;

            if let Some(mut completed) = state.completed_reads.pop_front() {
                let n = completed.len().min(buffer.len());
                buffer[..n].copy_from_slice(&completed[..n]);
                if n < completed.len() {
                    let remainder = completed.split_off(n);
                    state.completed_reads.push_front(remainder);
                }
                let _ = state;
                self.queue_read_if_needed(stream)?;
                return Ok(ReadOutcome::Read(n));
            }

            if state.read_closed {
                return Ok(ReadOutcome::Closed);
            }

            let _ = state;
            self.queue_read_if_needed(stream)?;
            Ok(ReadOutcome::WouldBlock)
        }

        fn write(
            &mut self,
            stream: StreamId,
            buffer: &[u8],
        ) -> Result<WriteOutcome, PlatformError> {
            let state = self
                .streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?;

            if let Some(n) = state.completed_writes.pop_front() {
                return if n == 0 {
                    Ok(WriteOutcome::Closed)
                } else {
                    Ok(WriteOutcome::Wrote(n.min(buffer.len())))
                };
            }

            if state.pending_write.is_some() {
                return Ok(WriteOutcome::WouldBlock);
            }

            let _ = state;
            self.queue_send(stream, buffer.to_vec())?;
            Ok(WriteOutcome::WouldBlock)
        }

        fn write_vectored(
            &mut self,
            stream: StreamId,
            buffers: &[IoSlice<'_>],
        ) -> Result<WriteOutcome, PlatformError> {
            if buffers.is_empty() {
                return Ok(WriteOutcome::Wrote(0));
            }
            let total = buffers.iter().map(|buffer| buffer.len()).sum();
            let mut flattened = Vec::with_capacity(total);
            for buffer in buffers {
                flattened.extend_from_slice(buffer);
            }
            self.write(stream, &flattened)
        }

        fn shutdown_read(&mut self, stream: StreamId) -> Result<(), PlatformError> {
            self.streams
                .get(&stream)
                .ok_or(PlatformError::InvalidResource)?
                .socket
                .shutdown(Shutdown::Read)
                .map_err(PlatformError::from)
        }

        fn shutdown_write(&mut self, stream: StreamId) -> Result<(), PlatformError> {
            self.streams
                .get(&stream)
                .ok_or(PlatformError::InvalidResource)?
                .socket
                .shutdown(Shutdown::Write)
                .map_err(PlatformError::from)
        }

        fn close_stream(&mut self, stream: StreamId) -> Result<(), PlatformError> {
            if self.streams.remove(&stream).is_some() {
                self.unregister_resource(stream.0);
                Ok(())
            } else {
                Err(PlatformError::InvalidResource)
            }
        }

        fn close_listener(&mut self, listener: ListenerId) -> Result<(), PlatformError> {
            if self.listeners.remove(&listener).is_some() {
                self.unregister_resource(listener.0);
                Ok(())
            } else {
                Err(PlatformError::InvalidResource)
            }
        }

        fn create_timer(&mut self) -> Result<TimerId, PlatformError> {
            let id = TimerId(self.alloc_token());
            self.register_resource(id.0, ResourceId::Timer(id));
            self.timers.insert(id, WindowsTimerState { deadline: None });
            Ok(id)
        }

        fn arm_timer(&mut self, timer: TimerId, deadline: Instant) -> Result<(), PlatformError> {
            self.timers
                .get_mut(&timer)
                .ok_or(PlatformError::InvalidResource)?
                .deadline = Some(deadline);
            Ok(())
        }

        fn disarm_timer(&mut self, timer: TimerId) -> Result<(), PlatformError> {
            self.timers
                .get_mut(&timer)
                .ok_or(PlatformError::InvalidResource)?
                .deadline = None;
            Ok(())
        }

        fn create_wakeup(&mut self) -> Result<WakeupId, PlatformError> {
            let id = WakeupId(self.alloc_token());
            self.register_resource(id.0, ResourceId::Wakeup(id));
            self.wakeups.insert(id, WindowsWakeupState);
            Ok(id)
        }

        fn wake(&mut self, wakeup: WakeupId) -> Result<(), PlatformError> {
            self.wakeups
                .get(&wakeup)
                .ok_or(PlatformError::InvalidResource)?;
            self.completion_port
                .post(0, wakeup.0 as usize, ptr::null_mut())
                .map_err(PlatformError::from)
        }

        fn poll(
            &mut self,
            timeout: Option<Duration>,
            output: &mut Vec<PlatformEvent>,
        ) -> Result<(), PlatformError> {
            output.clear();
            let effective_timeout = self.effective_timeout(timeout);

            self.replenish_all_accepts()?;
            let _ = self.wait_for_iocp_packet(effective_timeout, output)?;
            self.collect_due_timers(output);
            self.drain_iocp_now(output)?;

            for (&listener, state) in &self.listeners {
                if state.readable_interest && !state.completed_accepts.is_empty() {
                    output.push(PlatformEvent::ListenerAcceptReady { listener });
                }
            }

            for (&stream, state) in &self.streams {
                if !state.completed_reads.is_empty() {
                    output.push(PlatformEvent::StreamReadable { stream });
                }
                if !state.completed_writes.is_empty() {
                    output.push(PlatformEvent::StreamWritable { stream });
                }
            }

            self.replenish_all_accepts()?;
            Ok(())
        }
    }

    fn raw_socket_handle(raw: std::os::windows::io::RawSocket) -> *mut c_void {
        raw as usize as *mut c_void
    }

    fn raw_socket_value(raw: std::os::windows::io::RawSocket) -> RawSocketValue {
        raw as usize
    }

    fn zeroed_overlapped() -> iocp::Overlapped {
        iocp::Overlapped {
            internal: 0,
            internal_high: 0,
            offset: 0,
            offset_high: 0,
            h_event: ptr::null_mut(),
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub mod windows {
    use super::*;

    pub struct WindowsTransportPlatform;

    impl WindowsTransportPlatform {
        pub fn new() -> Result<Self, PlatformError> {
            Err(PlatformError::Unsupported(
                "windows transport platform is only available on Windows",
            ))
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
pub mod bsd {
    use super::*;

    pub struct KqueueTransportPlatform;

    impl KqueueTransportPlatform {
        pub fn new() -> Result<Self, PlatformError> {
            Err(PlatformError::Unsupported(
                "kqueue transport platform is only available on BSD/macOS",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::thread;

    fn poll_until<P, F>(platform: &mut P, timeout: Duration, mut predicate: F) -> Vec<PlatformEvent>
    where
        P: TransportPlatform,
        F: FnMut(&[PlatformEvent]) -> bool,
    {
        let deadline = Instant::now() + timeout;
        let mut events = Vec::new();
        while Instant::now() < deadline {
            platform
                .poll(Some(Duration::from_millis(10)), &mut events)
                .expect("poll transport platform");
            if predicate(&events) {
                return events;
            }
        }
        events
    }

    fn accept_loop<P: TransportPlatform>(platform: &mut P, listener: ListenerId) -> AcceptedStream {
        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            if let Some(accepted) = platform.accept(listener).expect("accept stream") {
                return accepted;
            }
            let events = poll_until(platform, Duration::from_millis(50), |events| {
                events
                    .iter()
                    .any(|event| matches!(event, PlatformEvent::ListenerAcceptReady { listener: id } if *id == listener))
            });
            if !events.is_empty() {
                if let Some(accepted) = platform
                    .accept(listener)
                    .expect("accept stream after readiness")
                {
                    return accepted;
                }
            }
        }
        panic!("listener never accepted a client");
    }

    fn assert_accepts_reads_and_writes_multiple_streams<P: TransportPlatform>(platform: &mut P) {
        let listener = platform
            .bind_listener(
                BindAddress::Ip("127.0.0.1:0".parse().expect("loopback addr")),
                ListenerOptions::default(),
            )
            .expect("bind listener");
        let addr = platform.local_addr(listener).expect("listener addr");

        let mut client_a = TcpStream::connect(addr).expect("connect client a");
        let mut client_b = TcpStream::connect(addr).expect("connect client b");
        client_a
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("read timeout a");
        client_b
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("read timeout b");

        let accept_ready = poll_until(platform, Duration::from_secs(1), |events| {
            events.iter().any(
                |event| matches!(event, PlatformEvent::ListenerAcceptReady { listener: id } if *id == listener),
            )
        });
        assert!(
            accept_ready.iter().any(
                |event| matches!(event, PlatformEvent::ListenerAcceptReady { listener: id } if *id == listener),
            ),
            "listener never reported accept readiness",
        );

        let stream_a = accept_loop(platform, listener).stream;
        let stream_b = accept_loop(platform, listener).stream;

        platform
            .set_stream_interest(stream_a, StreamInterest::readable())
            .expect("interest a");
        platform
            .set_stream_interest(stream_b, StreamInterest::readable())
            .expect("interest b");

        client_a.write_all(b"PING").expect("write a");
        client_b.write_all(b"PONG").expect("write b");

        let deadline = Instant::now() + Duration::from_secs(1);
        let mut events = Vec::new();
        let mut saw_a = false;
        let mut saw_b = false;
        while Instant::now() < deadline && !(saw_a && saw_b) {
            platform
                .poll(Some(Duration::from_millis(10)), &mut events)
                .expect("poll readable streams");
            saw_a |= events
                .iter()
                .any(|event| matches!(event, PlatformEvent::StreamReadable { stream } if *stream == stream_a));
            saw_b |= events
                .iter()
                .any(|event| matches!(event, PlatformEvent::StreamReadable { stream } if *stream == stream_b));
        }

        assert!(saw_a, "stream A never became readable");
        assert!(saw_b, "stream B never became readable");

        let mut buffer = [0u8; 16];
        assert_eq!(
            platform.read(stream_a, &mut buffer).expect("read stream a"),
            ReadOutcome::Read(4)
        );
        assert_eq!(&buffer[..4], b"PING");
        assert_eq!(
            platform.read(stream_b, &mut buffer).expect("read stream b"),
            ReadOutcome::Read(4)
        );
        assert_eq!(&buffer[..4], b"PONG");

        assert_eq!(
            write_until_progress(platform, stream_a, b"+PONG\r\n"),
            WriteOutcome::Wrote(7)
        );
        assert_eq!(
            write_until_progress(platform, stream_b, b"+OK\r\n"),
            WriteOutcome::Wrote(5)
        );

        let mut reply = [0u8; 8];
        client_a.read_exact(&mut reply[..7]).expect("reply a");
        assert_eq!(&reply[..7], b"+PONG\r\n");
        client_b.read_exact(&mut reply[..5]).expect("reply b");
        assert_eq!(&reply[..5], b"+OK\r\n");
    }

    fn write_until_progress<P: TransportPlatform>(
        platform: &mut P,
        stream: StreamId,
        bytes: &[u8],
    ) -> WriteOutcome {
        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline {
            match platform.write(stream, bytes).expect("write stream") {
                WriteOutcome::WouldBlock => {
                    let _ = poll_until(platform, Duration::from_millis(50), |events| {
                        events.iter().any(
                            |event| matches!(event, PlatformEvent::StreamWritable { stream: id } if *id == stream),
                        )
                    });
                }
                outcome => return outcome,
            }
        }
        panic!("stream write did not complete before the deadline");
    }

    fn assert_timer_and_wakeup_generate_events<P: TransportPlatform>(platform: &mut P) {
        let timer = platform.create_timer().expect("timer");
        let wakeup = platform.create_wakeup().expect("wakeup");

        platform
            .arm_timer(timer, Instant::now() + Duration::from_millis(10))
            .expect("arm timer");
        platform.wake(wakeup).expect("trigger wakeup");

        let deadline = Instant::now() + Duration::from_secs(1);
        let mut events = Vec::new();
        let mut saw_timer = false;
        let mut saw_wakeup = false;
        while Instant::now() < deadline && !(saw_timer && saw_wakeup) {
            platform
                .poll(Some(Duration::from_millis(10)), &mut events)
                .expect("poll timer and wakeup");
            saw_timer |= events.iter().any(
                |event| matches!(event, PlatformEvent::TimerExpired { timer: id } if *id == timer),
            );
            saw_wakeup |= events.iter().any(
                |event| matches!(event, PlatformEvent::Wakeup { wakeup: id } if *id == wakeup),
            );
        }

        assert!(saw_timer, "timer never expired");
        assert!(saw_wakeup, "wakeup was never observed");
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn assert_ipv6_listener_stays_ipv6_only<P: TransportPlatform>(platform: &mut P) {
        let listener = platform
            .bind_listener(
                BindAddress::Ip("[::]:0".parse().expect("ipv6 wildcard addr")),
                ListenerOptions::default(),
            )
            .expect("bind ipv6 listener");
        let addr = platform.local_addr(listener).expect("ipv6 listener addr");

        let result = TcpStream::connect_timeout(
            &SocketAddr::from(([127, 0, 0, 1], addr.port())),
            Duration::from_millis(200),
        );
        assert!(
            result.is_err(),
            "ipv6 wildcard listener unexpectedly accepted an ipv4 connection",
        );
    }

    #[cfg(any(target_os = "linux", target_os = "windows"))]
    fn assert_peer_shutdown_is_observable<P: TransportPlatform>(platform: &mut P) {
        let listener = platform
            .bind_listener(
                BindAddress::Ip("127.0.0.1:0".parse().expect("loopback addr")),
                ListenerOptions::default(),
            )
            .expect("bind listener");
        let addr = platform.local_addr(listener).expect("listener addr");

        let client = TcpStream::connect(addr).expect("connect client");
        let stream = accept_loop(platform, listener).stream;

        platform
            .set_stream_interest(stream, StreamInterest::readable())
            .expect("read interest");

        thread::spawn(move || {
            let _ = client.shutdown(Shutdown::Write);
        })
        .join()
        .expect("shutdown thread");

        let events = poll_until(platform, Duration::from_secs(1), |events| {
            events.iter().any(|event| {
                matches!(
                    event,
                    PlatformEvent::StreamClosed { stream: id, .. }
                        | PlatformEvent::StreamReadable { stream: id }
                        if *id == stream
                )
            })
        });

        assert!(
            events.iter().any(|event| {
                matches!(
                    event,
                    PlatformEvent::StreamClosed { stream: id, .. }
                        | PlatformEvent::StreamReadable { stream: id }
                        if *id == stream
                )
            }),
            "peer shutdown did not surface as readability or closure",
        );

        let mut buffer = [0u8; 1];
        assert_eq!(
            platform
                .read(stream, &mut buffer)
                .expect("read closed stream"),
            ReadOutcome::Closed
        );
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    mod bsd_tests {
        use super::*;
        use crate::bsd::KqueueTransportPlatform;

        #[test]
        fn accepts_reads_and_writes_multiple_streams() {
            let mut platform = KqueueTransportPlatform::new().expect("create platform");
            assert_eq!(
                platform.native_event_provider(),
                NativeEventProvider::Kqueue
            );
            assert_accepts_reads_and_writes_multiple_streams(&mut platform);
        }

        #[test]
        fn timer_and_wakeup_generate_events() {
            let mut platform = KqueueTransportPlatform::new().expect("create platform");
            assert_timer_and_wakeup_generate_events(&mut platform);
        }

        #[test]
        fn close_event_is_reported_after_peer_shutdown() {
            let mut platform = KqueueTransportPlatform::new().expect("create platform");
            let listener = platform
                .bind_listener(
                    BindAddress::Ip("127.0.0.1:0".parse().expect("loopback addr")),
                    ListenerOptions::default(),
                )
                .expect("bind listener");
            let addr = platform.local_addr(listener).expect("listener addr");

            let client = TcpStream::connect(addr).expect("connect client");
            let stream = accept_loop(&mut platform, listener).stream;

            platform
                .set_stream_interest(stream, StreamInterest::readable())
                .expect("read interest");

            thread::spawn(move || {
                let _ = client.shutdown(Shutdown::Write);
            })
            .join()
            .expect("shutdown thread");

            let events = poll_until(&mut platform, Duration::from_secs(1), |events| {
                events.iter().any(
                    |event| matches!(event, PlatformEvent::StreamClosed { stream: id, .. } if *id == stream),
                )
            });

            assert!(events.iter().any(|event| matches!(
                event,
                PlatformEvent::StreamClosed {
                    stream: id,
                    kind: CloseKind::ReadClosed
                } if *id == stream
            )));
        }
    }

    #[cfg(target_os = "linux")]
    mod linux_tests {
        use super::*;
        use crate::linux::EpollTransportPlatform;

        #[test]
        fn accepts_reads_and_writes_multiple_streams() {
            let mut platform = EpollTransportPlatform::new().expect("create platform");
            assert_eq!(platform.native_event_provider(), NativeEventProvider::Epoll);
            assert_accepts_reads_and_writes_multiple_streams(&mut platform);
        }

        #[test]
        fn timer_and_wakeup_generate_events() {
            let mut platform = EpollTransportPlatform::new().expect("create platform");
            assert_timer_and_wakeup_generate_events(&mut platform);
        }

        #[test]
        fn peer_shutdown_is_observable() {
            let mut platform = EpollTransportPlatform::new().expect("create platform");
            assert_peer_shutdown_is_observable(&mut platform);
        }

        #[test]
        fn ipv6_listener_stays_ipv6_only() {
            let mut platform = EpollTransportPlatform::new().expect("create platform");
            assert_ipv6_listener_stays_ipv6_only(&mut platform);
        }
    }

    #[cfg(target_os = "windows")]
    mod windows_tests {
        use super::*;
        use crate::windows::WindowsTransportPlatform;

        #[test]
        fn accepts_reads_and_writes_multiple_streams() {
            let mut platform = WindowsTransportPlatform::new().expect("create platform");
            assert_eq!(platform.native_event_provider(), NativeEventProvider::Iocp);
            assert!(platform.capabilities().supports_native_wakeups);
            assert_accepts_reads_and_writes_multiple_streams(&mut platform);
        }

        #[test]
        fn timer_and_wakeup_generate_events() {
            let mut platform = WindowsTransportPlatform::new().expect("create platform");
            assert_timer_and_wakeup_generate_events(&mut platform);
        }

        #[test]
        fn peer_shutdown_is_observable() {
            let mut platform = WindowsTransportPlatform::new().expect("create platform");
            assert_peer_shutdown_is_observable(&mut platform);
        }

        #[test]
        fn ipv6_listener_stays_ipv6_only() {
            let mut platform = WindowsTransportPlatform::new().expect("create platform");
            assert_ipv6_listener_stays_ipv6_only(&mut platform);
        }
    }
}
