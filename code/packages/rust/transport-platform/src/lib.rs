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
struct ListenerState {
    socket: TcpListener,
    defaults: ListenerStateDefaults,
    readable_interest: bool,
}

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
    use socket2::{SockRef, TcpKeepalive};
    use std::io::{ErrorKind, Read, Write};
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::thread;
    use windows_sys::Win32::Networking::WinSock::{WSAStartup, WSADATA};

    #[derive(Debug)]
    struct WindowsTimerState {
        deadline: Option<Instant>,
    }

    #[derive(Debug)]
    struct WindowsWakeupState {
        reader: TcpStream,
        writer: TcpStream,
    }

    /// `WindowsTransportPlatform` is the current Windows provider. It uses
    /// nonblocking sockets with readiness probes, a loopback socket pair for
    /// wakeups, and user-space timer bookkeeping. A richer IOCP-backed socket
    /// completion provider can replace this later without changing the public
    /// API.
    pub struct WindowsTransportPlatform {
        next_token: u64,
        listeners: BTreeMap<ListenerId, ListenerState>,
        streams: BTreeMap<StreamId, StreamState>,
        timers: BTreeMap<TimerId, WindowsTimerState>,
        wakeups: BTreeMap<WakeupId, WindowsWakeupState>,
    }

    impl WindowsTransportPlatform {
        pub fn new() -> Result<Self, PlatformError> {
            let mut data = std::mem::MaybeUninit::<WSADATA>::uninit();
            let result = unsafe { WSAStartup(0x0202, data.as_mut_ptr()) };
            if result != 0 {
                return Err(PlatformError::from(io::Error::from_raw_os_error(result)));
            }

            Ok(Self {
                next_token: 1,
                listeners: BTreeMap::new(),
                streams: BTreeMap::new(),
                timers: BTreeMap::new(),
                wakeups: BTreeMap::new(),
            })
        }

        fn alloc_token(&mut self) -> u64 {
            let token = self.next_token;
            self.next_token += 1;
            token
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

        fn create_wakeup_pair() -> Result<(TcpStream, TcpStream), PlatformError> {
            let listener =
                TcpListener::bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)))?;
            let addr = listener.local_addr()?;
            let writer = TcpStream::connect(addr)?;
            let (reader, _) = listener.accept()?;
            reader.set_nonblocking(true)?;
            writer.set_nonblocking(true)?;
            Ok((reader, writer))
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

        fn drain_wakeup(reader: &mut TcpStream) -> Result<(), PlatformError> {
            let mut buffer = [0u8; 128];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => return Ok(()),
                    Ok(_) => continue,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(()),
                    Err(error) => return Err(PlatformError::from(error)),
                }
            }
        }
    }

    impl TransportPlatform for WindowsTransportPlatform {
        fn capabilities(&self) -> PlatformCapabilities {
            PlatformCapabilities {
                supports_half_close: true,
                supports_vectored_write: true,
                supports_zero_copy_send: false,
                supports_native_timers: false,
                supports_native_wakeups: false,
            }
        }

        fn bind_listener(
            &mut self,
            address: BindAddress,
            options: ListenerOptions,
        ) -> Result<ListenerId, PlatformError> {
            let BindAddress::Ip(address) = address;
            let listener = TcpListener::bind(address)
                .map_err(|error| PlatformError::Io(format!("bind listener socket: {error}")))?;
            listener
                .set_nonblocking(true)
                .map_err(|error| PlatformError::Io(format!("set listener nonblocking: {error}")))?;
            let id = ListenerId(self.alloc_token());
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
            self.streams
                .get_mut(&stream)
                .ok_or(PlatformError::InvalidResource)?
                .interest = interest;
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
                Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(ReadOutcome::WouldBlock),
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
                Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(WriteOutcome::WouldBlock),
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
                Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(WriteOutcome::WouldBlock),
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
            self.streams
                .remove(&stream)
                .map(|_| ())
                .ok_or(PlatformError::InvalidResource)
        }

        fn close_listener(&mut self, listener: ListenerId) -> Result<(), PlatformError> {
            self.listeners
                .remove(&listener)
                .map(|_| ())
                .ok_or(PlatformError::InvalidResource)
        }

        fn create_timer(&mut self) -> Result<TimerId, PlatformError> {
            let id = TimerId(self.alloc_token());
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
            let (reader, writer) = Self::create_wakeup_pair()?;
            self.wakeups
                .insert(id, WindowsWakeupState { reader, writer });
            Ok(id)
        }

        fn wake(&mut self, wakeup: WakeupId) -> Result<(), PlatformError> {
            let state = self
                .wakeups
                .get_mut(&wakeup)
                .ok_or(PlatformError::InvalidResource)?;
            match state.writer.write(&[1]) {
                Ok(_) => Ok(()),
                Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(()),
                Err(error) => Err(PlatformError::from(error)),
            }
        }

        fn poll(
            &mut self,
            timeout: Option<Duration>,
            output: &mut Vec<PlatformEvent>,
        ) -> Result<(), PlatformError> {
            output.clear();
            let effective_timeout = self.effective_timeout(timeout);

            // WSAPoll is unreliable across the Windows toolchain mix used by
            // the native-extension builds in this repository: valid Rust TCP
            // sockets can be reported as WSAENOTSOCK. Keep the provider
            // correct by using nonblocking probes and the upper reactor's
            // WouldBlock handling as the readiness filter.
            if let Some(duration) = effective_timeout {
                thread::sleep(duration.min(Duration::from_millis(10)));
            } else {
                thread::sleep(Duration::from_millis(10));
            }
            self.collect_due_timers(output);

            for (&listener, state) in &self.listeners {
                if state.readable_interest {
                    output.push(PlatformEvent::ListenerAcceptReady { listener });
                }
            }

            for (&stream, state) in &self.streams {
                if state.interest.readable {
                    output.push(PlatformEvent::StreamReadable { stream });
                }
                if state.interest.writable {
                    output.push(PlatformEvent::StreamWritable { stream });
                }
            }

            for (&wakeup, state) in &mut self.wakeups {
                let mut byte = [0u8; 1];
                match state.reader.peek(&mut byte) {
                    Ok(0) => {}
                    Ok(_) => {
                        Self::drain_wakeup(&mut state.reader)?;
                        output.push(PlatformEvent::Wakeup { wakeup });
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                    Err(error) => return Err(PlatformError::from(error)),
                }
            }

            Ok(())
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
            platform
                .write(stream_a, b"+PONG\r\n")
                .expect("write stream a"),
            WriteOutcome::Wrote(7)
        );
        assert_eq!(
            platform
                .write(stream_b, b"+OK\r\n")
                .expect("write stream b"),
            WriteOutcome::Wrote(5)
        );

        let mut reply = [0u8; 8];
        client_a.read_exact(&mut reply[..7]).expect("reply a");
        assert_eq!(&reply[..7], b"+PONG\r\n");
        client_b.read_exact(&mut reply[..5]).expect("reply b");
        assert_eq!(&reply[..5], b"+OK\r\n");
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
