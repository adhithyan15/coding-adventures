//! # stream-reactor
//!
//! Generic byte-stream reactor built on top of `transport-platform`.
//!
//! This crate owns listener acceptance, per-stream state, queued writes, and
//! neutral byte-stream handler progression without committing to one protocol.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use transport_platform::{
    BindAddress, CloseKind, ListenerId, ListenerOptions, PlatformError, PlatformEvent, ReadOutcome,
    StreamId, StreamInterest, StreamOptions, TransportPlatform, WriteOutcome,
};

const DEFAULT_MAX_CONNECTIONS: usize = 1_024;
const DEFAULT_MAX_PENDING_WRITE_BYTES: usize = 64 * 1024;
const DEFAULT_READ_BUFFER_SIZE: usize = 4096;
const DEFAULT_POLL_TIMEOUT_MS: u64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamConnectionInfo {
    pub id: ConnectionId,
    pub peer_addr: SocketAddr,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StreamHandlerResult {
    pub write: Vec<u8>,
    pub close: bool,
}

impl StreamHandlerResult {
    pub fn write(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            write: bytes.into(),
            close: false,
        }
    }

    pub const fn close() -> Self {
        Self {
            write: Vec::new(),
            close: true,
        }
    }

    pub fn write_and_close(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            write: bytes.into(),
            close: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamReactorOptions {
    pub listener: ListenerOptions,
    pub stream: StreamOptions,
    pub read_buffer_size: usize,
    pub max_connections: usize,
    pub max_pending_write_bytes: usize,
    pub poll_timeout: Duration,
}

impl Default for StreamReactorOptions {
    fn default() -> Self {
        Self {
            listener: ListenerOptions::default(),
            stream: StreamOptions::default(),
            read_buffer_size: DEFAULT_READ_BUFFER_SIZE,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            max_pending_write_bytes: DEFAULT_MAX_PENDING_WRITE_BYTES,
            poll_timeout: Duration::from_millis(DEFAULT_POLL_TIMEOUT_MS),
        }
    }
}

#[derive(Clone)]
pub struct StopHandle(Arc<AtomicBool>);

impl StopHandle {
    pub fn stop(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

pub type Handler =
    Arc<dyn Fn(StreamConnectionInfo, &[u8]) -> StreamHandlerResult + Send + Sync + 'static>;

struct ConnectionState {
    info: StreamConnectionInfo,
    pending_write: Vec<u8>,
    peer_closed: bool,
    close_when_flushed: bool,
}

pub struct StreamReactor<P> {
    platform: P,
    listener: ListenerId,
    listener_addr: SocketAddr,
    connections: BTreeMap<StreamId, ConnectionState>,
    next_connection_id: u64,
    stop_flag: Arc<AtomicBool>,
    read_buffer_size: usize,
    max_connections: usize,
    max_pending_write_bytes: usize,
    poll_timeout: Duration,
    stream_options: StreamOptions,
    handler: Handler,
}

impl<P: TransportPlatform> StreamReactor<P> {
    pub fn bind<F>(
        mut platform: P,
        address: BindAddress,
        options: StreamReactorOptions,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        F: Fn(StreamConnectionInfo, &[u8]) -> StreamHandlerResult + Send + Sync + 'static,
    {
        let listener = platform.bind_listener(address, options.listener)?;
        let listener_addr = platform.local_addr(listener)?;
        platform.set_listener_interest(listener, true)?;

        Ok(Self {
            platform,
            listener,
            listener_addr,
            connections: BTreeMap::new(),
            next_connection_id: 1,
            stop_flag: Arc::new(AtomicBool::new(false)),
            read_buffer_size: options.read_buffer_size.max(1),
            max_connections: options.max_connections.max(1),
            max_pending_write_bytes: options.max_pending_write_bytes.max(1),
            poll_timeout: options.poll_timeout,
            stream_options: options.stream,
            handler: Arc::new(handler),
        })
    }

    pub fn set_max_connections(&mut self, max_connections: usize) {
        self.max_connections = max_connections.max(1);
    }

    pub fn set_max_pending_write_bytes(&mut self, max_pending_write_bytes: usize) {
        self.max_pending_write_bytes = max_pending_write_bytes.max(1);
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.listener_addr
    }

    pub fn stop_handle(&self) -> StopHandle {
        StopHandle(Arc::clone(&self.stop_flag))
    }

    pub fn serve(&mut self) -> Result<(), PlatformError> {
        self.stop_flag.store(false, Ordering::SeqCst);
        let mut events = Vec::new();
        while !self.stop_flag.load(Ordering::SeqCst) {
            self.platform.poll(Some(self.poll_timeout), &mut events)?;
            for event in &events {
                match event {
                    PlatformEvent::ListenerAcceptReady { listener }
                        if *listener == self.listener =>
                    {
                        self.accept_ready()?;
                    }
                    PlatformEvent::StreamReadable { stream } => {
                        self.read_ready(*stream)?;
                    }
                    PlatformEvent::StreamWritable { stream } => {
                        self.write_ready(*stream)?;
                    }
                    PlatformEvent::StreamClosed { stream, kind } => {
                        self.stream_closed(*stream, *kind)?;
                    }
                    PlatformEvent::Error { resource, error } => match resource {
                        transport_platform::ResourceId::Listener(listener)
                            if *listener == self.listener =>
                        {
                            return Err(error.clone());
                        }
                        transport_platform::ResourceId::Stream(stream) => {
                            self.close_connection(*stream)?;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
        self.shutdown_all()
    }

    fn accept_ready(&mut self) -> Result<(), PlatformError> {
        loop {
            match self.platform.accept(self.listener)? {
                Some(accepted) => {
                    if self.connections.len() >= self.max_connections {
                        self.close_stream_raw(accepted.stream)?;
                        continue;
                    }

                    self.platform
                        .configure_stream(accepted.stream, self.stream_options)?;
                    self.platform
                        .set_stream_interest(accepted.stream, StreamInterest::readable())?;

                    let connection_id = ConnectionId(self.next_connection_id);
                    self.next_connection_id += 1;
                    self.connections.insert(
                        accepted.stream,
                        ConnectionState {
                            info: StreamConnectionInfo {
                                id: connection_id,
                                peer_addr: accepted.peer_addr,
                            },
                            pending_write: Vec::new(),
                            peer_closed: false,
                            close_when_flushed: false,
                        },
                    );
                }
                None => return Ok(()),
            }
        }
    }

    fn read_ready(&mut self, stream: StreamId) -> Result<(), PlatformError> {
        let Some(mut state) = self.connections.remove(&stream) else {
            return Ok(());
        };

        if state.pending_write.len() >= self.max_pending_write_bytes {
            self.platform
                .set_stream_interest(stream, self.interest_for(&state))?;
            self.connections.insert(stream, state);
            return Ok(());
        }

        let mut close_now = false;
        let mut buffer = vec![0u8; self.read_buffer_size];

        loop {
            match self.platform.read(stream, &mut buffer)? {
                ReadOutcome::Read(n) => {
                    let result = (self.handler)(state.info, &buffer[..n]);
                    if !result.write.is_empty() {
                        if state.pending_write.len().saturating_add(result.write.len())
                            > self.max_pending_write_bytes
                        {
                            close_now = true;
                            break;
                        }
                        state.pending_write.extend_from_slice(&result.write);
                    }
                    if result.close {
                        state.close_when_flushed = true;
                    }
                    if n < buffer.len() {
                        break;
                    }
                }
                ReadOutcome::WouldBlock => break,
                ReadOutcome::Closed => {
                    state.peer_closed = true;
                    break;
                }
            }
        }

        if close_now || self.should_close_after_io(&state) {
            self.close_stream_raw(stream)
        } else {
            self.platform
                .set_stream_interest(stream, self.interest_for(&state))?;
            self.connections.insert(stream, state);
            Ok(())
        }
    }

    fn write_ready(&mut self, stream: StreamId) -> Result<(), PlatformError> {
        let Some(mut state) = self.connections.remove(&stream) else {
            return Ok(());
        };

        let mut close_now = false;
        while !state.pending_write.is_empty() {
            match self.platform.write(stream, &state.pending_write)? {
                WriteOutcome::Wrote(n) => {
                    state.pending_write.drain(..n);
                }
                WriteOutcome::WouldBlock => break,
                WriteOutcome::Closed => {
                    close_now = true;
                    break;
                }
            }
        }

        if close_now || self.should_close_after_io(&state) {
            self.close_stream_raw(stream)
        } else {
            self.platform
                .set_stream_interest(stream, self.interest_for(&state))?;
            self.connections.insert(stream, state);
            Ok(())
        }
    }

    fn stream_closed(&mut self, stream: StreamId, kind: CloseKind) -> Result<(), PlatformError> {
        let Some(mut state) = self.connections.remove(&stream) else {
            return Ok(());
        };

        match kind {
            CloseKind::ReadClosed => {
                state.peer_closed = true;
                if self.should_close_after_io(&state) {
                    self.close_stream_raw(stream)
                } else {
                    self.platform
                        .set_stream_interest(stream, self.interest_for(&state))?;
                    self.connections.insert(stream, state);
                    Ok(())
                }
            }
            CloseKind::WriteClosed | CloseKind::FullyClosed | CloseKind::Reset => {
                self.close_stream_raw(stream)
            }
        }
    }

    fn interest_for(&self, state: &ConnectionState) -> StreamInterest {
        if state.pending_write.is_empty() {
            if state.peer_closed || state.close_when_flushed {
                StreamInterest::none()
            } else {
                StreamInterest::readable()
            }
        } else if state.peer_closed || state.close_when_flushed {
            StreamInterest {
                readable: false,
                writable: true,
            }
        } else {
            StreamInterest::readable_writable()
        }
    }

    fn should_close_after_io(&self, state: &ConnectionState) -> bool {
        state.pending_write.is_empty() && (state.peer_closed || state.close_when_flushed)
    }

    fn close_connection(&mut self, stream: StreamId) -> Result<(), PlatformError> {
        self.connections.remove(&stream);
        self.close_stream_raw(stream)
    }

    fn close_stream_raw(&mut self, stream: StreamId) -> Result<(), PlatformError> {
        match self.platform.close_stream(stream) {
            Ok(()) => Ok(()),
            Err(PlatformError::InvalidResource | PlatformError::ResourceClosed) => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn shutdown_all(&mut self) -> Result<(), PlatformError> {
        let streams = self.connections.keys().copied().collect::<Vec<_>>();
        for stream in streams {
            self.close_connection(stream)?;
        }
        match self.platform.close_listener(self.listener) {
            Ok(()) => Ok(()),
            Err(PlatformError::InvalidResource | PlatformError::ResourceClosed) => Ok(()),
            Err(error) => Err(error),
        }
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
impl StreamReactor<transport_platform::bsd::KqueueTransportPlatform> {
    pub fn bind_kqueue<A, F>(addr: A, handler: F) -> Result<Self, PlatformError>
    where
        A: std::net::ToSocketAddrs,
        F: Fn(StreamConnectionInfo, &[u8]) -> StreamHandlerResult + Send + Sync + 'static,
    {
        let address = addr
            .to_socket_addrs()
            .map_err(PlatformError::from)?
            .next()
            .ok_or_else(|| PlatformError::Io("no socket addresses resolved".into()))?;
        let platform = transport_platform::bsd::KqueueTransportPlatform::new()?;
        Self::bind(
            platform,
            BindAddress::Ip(address),
            StreamReactorOptions::default(),
            handler,
        )
    }
}

#[cfg(all(
    test,
    any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    )
))]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpStream};
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[test]
    fn serves_many_concurrent_echo_clients_without_crashing() {
        let mut reactor = StreamReactor::bind_kqueue(("127.0.0.1", 0), |_, bytes| {
            StreamHandlerResult::write(bytes.to_vec())
        })
        .expect("bind");
        let addr = reactor.local_addr();
        let stop = reactor.stop_handle();

        let server = thread::spawn(move || reactor.serve());

        let client_count = 24usize;
        let barrier = Arc::new(Barrier::new(client_count));
        let mut clients = Vec::new();

        for i in 0..client_count {
            let barrier = Arc::clone(&barrier);
            clients.push(thread::spawn(move || -> Result<Vec<u8>, String> {
                barrier.wait();
                let mut stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
                let payload = format!("client-{i}-payload").into_bytes();
                stream.write_all(&payload).map_err(|e| e.to_string())?;
                stream
                    .shutdown(Shutdown::Write)
                    .map_err(|e| e.to_string())?;

                let mut echoed = Vec::new();
                stream.read_to_end(&mut echoed).map_err(|e| e.to_string())?;
                Ok(echoed)
            }));
        }

        for (i, client) in clients.into_iter().enumerate() {
            let echoed = client.join().expect("client thread").expect("client io");
            assert_eq!(echoed, format!("client-{i}-payload").into_bytes());
        }

        stop.stop();
        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");
    }

    #[test]
    fn rejects_connections_above_the_configured_cap() {
        let mut reactor = StreamReactor::bind_kqueue(("127.0.0.1", 0), |_, bytes| {
            StreamHandlerResult::write(bytes.to_vec())
        })
        .expect("bind");
        reactor.set_max_connections(2);
        let addr = reactor.local_addr();

        let client_a = TcpStream::connect(addr).expect("connect a");
        let client_b = TcpStream::connect(addr).expect("connect b");
        for _ in 0..8 {
            reactor.accept_ready().expect("accept first batch");
            if reactor.connections.len() == 2 {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            reactor.connections.len(),
            2,
            "two clients should be admitted"
        );

        let client_c = TcpStream::connect(addr).expect("connect c");
        reactor.accept_ready().expect("reject over-limit client");
        assert_eq!(
            reactor.connections.len(),
            2,
            "reactor should refuse connections past the configured cap"
        );

        drop(client_a);
        drop(client_b);
        drop(client_c);
    }

    #[test]
    fn closes_connections_that_exceed_the_pending_write_budget() {
        let mut reactor = StreamReactor::bind_kqueue(("127.0.0.1", 0), |_, _| {
            StreamHandlerResult::write(vec![1u8; 64])
        })
        .expect("bind");
        reactor.set_max_pending_write_bytes(32);
        let addr = reactor.local_addr();

        let mut client = TcpStream::connect(addr).expect("connect");
        client.write_all(b"trigger").expect("write request");
        for _ in 0..8 {
            reactor.accept_ready().expect("accept client");
            if reactor.connections.len() == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        let stream = *reactor
            .connections
            .keys()
            .next()
            .expect("accepted connection stream");
        for _ in 0..20 {
            reactor
                .read_ready(stream)
                .expect("overflowing write budget should close cleanly");
            if !reactor.connections.contains_key(&stream) {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            !reactor.connections.contains_key(&stream),
            "reactor should drop streams whose queued output exceeds the limit"
        );
    }

    #[test]
    fn stop_handle_shuts_down_the_server() {
        let mut reactor = StreamReactor::bind_kqueue(("127.0.0.1", 0), |_, bytes| {
            StreamHandlerResult::write(bytes.to_vec())
        })
        .expect("bind");
        let stop = reactor.stop_handle();

        let server = thread::spawn(move || reactor.serve());
        thread::sleep(Duration::from_millis(20));
        stop.stop();

        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");
    }
}
