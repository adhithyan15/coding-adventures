//! # stream-reactor
//!
//! Generic byte-stream reactor built on top of `transport-platform`.
//!
//! This crate owns listener acceptance, per-stream state, queued writes, and
//! neutral byte-stream handler progression without committing to one protocol.

use std::collections::{BTreeMap, VecDeque};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
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
    pub defer_read: bool,
}

impl StreamHandlerResult {
    pub fn write(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            write: bytes.into(),
            close: false,
            defer_read: false,
        }
    }

    pub const fn close() -> Self {
        Self {
            write: Vec::new(),
            close: true,
            defer_read: false,
        }
    }

    pub const fn defer_read() -> Self {
        Self {
            write: Vec::new(),
            close: false,
            defer_read: true,
        }
    }

    pub fn write_and_close(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            write: bytes.into(),
            close: true,
            defer_read: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StreamReactorOptions {
    pub listener: ListenerOptions,
    pub stream: StreamOptions,
    /// Shared counter used by sharded reactors to allocate unique connection IDs.
    pub connection_id_seed: Option<Arc<AtomicU64>>,
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
            connection_id_seed: None,
            read_buffer_size: DEFAULT_READ_BUFFER_SIZE,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            max_pending_write_bytes: DEFAULT_MAX_PENDING_WRITE_BYTES,
            poll_timeout: Duration::from_millis(DEFAULT_POLL_TIMEOUT_MS),
        }
    }
}

impl PartialEq for StreamReactorOptions {
    fn eq(&self, other: &Self) -> bool {
        self.listener == other.listener
            && self.stream == other.stream
            && self.connection_id_seed.is_some() == other.connection_id_seed.is_some()
            && self.read_buffer_size == other.read_buffer_size
            && self.max_connections == other.max_connections
            && self.max_pending_write_bytes == other.max_pending_write_bytes
            && self.poll_timeout == other.poll_timeout
    }
}

impl Eq for StreamReactorOptions {}

#[derive(Clone)]
pub struct StopHandle(Arc<AtomicBool>);

impl StopHandle {
    pub fn stop(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

#[derive(Clone, Default)]
pub struct StreamMailbox {
    commands: Arc<Mutex<VecDeque<StreamMailboxCommand>>>,
}

impl StreamMailbox {
    pub fn send(&self, connection_id: ConnectionId, bytes: impl Into<Vec<u8>>) {
        self.push(StreamMailboxCommand::Write {
            connection_id,
            bytes: bytes.into(),
            close: false,
        });
    }

    pub fn send_and_close(&self, connection_id: ConnectionId, bytes: impl Into<Vec<u8>>) {
        self.push(StreamMailboxCommand::Write {
            connection_id,
            bytes: bytes.into(),
            close: true,
        });
    }

    pub fn close(&self, connection_id: ConnectionId) {
        self.push(StreamMailboxCommand::Close { connection_id });
    }

    pub fn pause_reads(&self, connection_id: ConnectionId) {
        self.push(StreamMailboxCommand::PauseReads { connection_id });
    }

    pub fn resume_reads(&self, connection_id: ConnectionId) {
        self.push(StreamMailboxCommand::ResumeReads { connection_id });
    }

    pub fn resume_all_reads(&self) {
        self.push(StreamMailboxCommand::ResumeAllReads);
    }

    fn push(&self, command: StreamMailboxCommand) {
        self.commands
            .lock()
            .expect("stream mailbox mutex poisoned")
            .push_back(command);
    }

    fn drain(&self) -> Vec<StreamMailboxCommand> {
        self.commands
            .lock()
            .expect("stream mailbox mutex poisoned")
            .drain(..)
            .collect()
    }
}

#[derive(Debug)]
enum StreamMailboxCommand {
    Write {
        connection_id: ConnectionId,
        bytes: Vec<u8>,
        close: bool,
    },
    Close {
        connection_id: ConnectionId,
    },
    PauseReads {
        connection_id: ConnectionId,
    },
    ResumeReads {
        connection_id: ConnectionId,
    },
    ResumeAllReads,
}

type StateInit<S> = Arc<dyn Fn(StreamConnectionInfo) -> S + Send + Sync + 'static>;
type StatefulHandler<S> =
    Arc<dyn Fn(StreamConnectionInfo, &mut S, &[u8]) -> StreamHandlerResult + Send + Sync + 'static>;
type CloseHandler<S> = Arc<dyn Fn(StreamConnectionInfo, S) + Send + Sync + 'static>;

struct ConnectionState<S> {
    info: StreamConnectionInfo,
    app_state: S,
    pending_write: Vec<u8>,
    deferred_reads: VecDeque<Vec<u8>>,
    peer_closed: bool,
    read_paused: bool,
    close_when_flushed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadChunkOutcome {
    Applied,
    Deferred,
    CloseNow,
}

pub struct StreamReactor<P, S = ()> {
    platform: P,
    listener: ListenerId,
    listener_addr: SocketAddr,
    connections: BTreeMap<StreamId, ConnectionState<S>>,
    connection_index: BTreeMap<ConnectionId, StreamId>,
    next_connection_id: u64,
    stop_flag: Arc<AtomicBool>,
    mailbox: StreamMailbox,
    read_buffer_size: usize,
    max_connections: usize,
    max_pending_write_bytes: usize,
    poll_timeout: Duration,
    stream_options: StreamOptions,
    connection_id_seed: Option<Arc<AtomicU64>>,
    state_init: StateInit<S>,
    handler: StatefulHandler<S>,
    on_close: CloseHandler<S>,
}

impl<P: TransportPlatform> StreamReactor<P, ()> {
    pub fn bind<F>(
        platform: P,
        address: BindAddress,
        options: StreamReactorOptions,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        F: Fn(StreamConnectionInfo, &[u8]) -> StreamHandlerResult + Send + Sync + 'static,
    {
        StreamReactor::bind_with_state(
            platform,
            address,
            options,
            |_| (),
            move |info, _, bytes| handler(info, bytes),
            |_, _| {},
        )
    }
}

impl<P: TransportPlatform, S: Send + 'static> StreamReactor<P, S> {
    pub fn bind_with_state<I, F, C>(
        mut platform: P,
        address: BindAddress,
        options: StreamReactorOptions,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<Self, PlatformError>
    where
        I: Fn(StreamConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(StreamConnectionInfo, &mut S, &[u8]) -> StreamHandlerResult + Send + Sync + 'static,
        C: Fn(StreamConnectionInfo, S) + Send + Sync + 'static,
    {
        let listener = platform
            .bind_listener(address, options.listener)
            .map_err(|error| {
                PlatformError::ProviderFault(format!("bind listener resource: {error}"))
            })?;
        let listener_addr = platform.local_addr(listener).map_err(|error| {
            PlatformError::ProviderFault(format!("read listener address: {error}"))
        })?;
        platform
            .set_listener_interest(listener, true)
            .map_err(|error| {
                PlatformError::ProviderFault(format!("enable listener interest: {error}"))
            })?;

        Ok(Self {
            platform,
            listener,
            listener_addr,
            connections: BTreeMap::new(),
            connection_index: BTreeMap::new(),
            next_connection_id: 1,
            stop_flag: Arc::new(AtomicBool::new(false)),
            mailbox: StreamMailbox::default(),
            read_buffer_size: options.read_buffer_size.max(1),
            max_connections: options.max_connections.max(1),
            max_pending_write_bytes: options.max_pending_write_bytes.max(1),
            poll_timeout: options.poll_timeout,
            stream_options: options.stream,
            connection_id_seed: options.connection_id_seed,
            state_init: Arc::new(init),
            handler: Arc::new(handler),
            on_close: Arc::new(on_close),
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

    pub fn mailbox(&self) -> StreamMailbox {
        self.mailbox.clone()
    }

    pub fn serve(&mut self) -> Result<(), PlatformError> {
        self.stop_flag.store(false, Ordering::SeqCst);
        let mut events = Vec::new();
        while !self.stop_flag.load(Ordering::SeqCst) {
            self.drain_mailbox()?;
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
            self.drain_mailbox()?;
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

                    let connection_id = if let Some(seed) = &self.connection_id_seed {
                        ConnectionId(seed.fetch_add(1, Ordering::SeqCst))
                    } else {
                        let id = ConnectionId(self.next_connection_id);
                        self.next_connection_id += 1;
                        id
                    };
                    self.connection_index.insert(connection_id, accepted.stream);
                    self.connections.insert(
                        accepted.stream,
                        ConnectionState {
                            info: StreamConnectionInfo {
                                id: connection_id,
                                peer_addr: accepted.peer_addr,
                            },
                            app_state: (self.state_init)(StreamConnectionInfo {
                                id: connection_id,
                                peer_addr: accepted.peer_addr,
                            }),
                            pending_write: Vec::new(),
                            deferred_reads: VecDeque::new(),
                            peer_closed: false,
                            read_paused: false,
                            close_when_flushed: false,
                        },
                    );
                }
                None => return Ok(()),
            }
        }
    }

    fn drain_mailbox(&mut self) -> Result<(), PlatformError> {
        for command in self.mailbox.drain() {
            match command {
                StreamMailboxCommand::ResumeAllReads => self.resume_all_reads()?,
                other => self.apply_mailbox_command(other)?,
            }
        }
        Ok(())
    }

    fn apply_mailbox_command(
        &mut self,
        command: StreamMailboxCommand,
    ) -> Result<(), PlatformError> {
        let connection_id = match &command {
            StreamMailboxCommand::Write { connection_id, .. } => *connection_id,
            StreamMailboxCommand::Close { connection_id } => *connection_id,
            StreamMailboxCommand::PauseReads { connection_id } => *connection_id,
            StreamMailboxCommand::ResumeReads { connection_id } => *connection_id,
            StreamMailboxCommand::ResumeAllReads => return self.resume_all_reads(),
        };

        let Some(stream) = self.connection_index.get(&connection_id).copied() else {
            return Ok(());
        };
        let Some(mut state) = self.connections.remove(&stream) else {
            self.connection_index.remove(&connection_id);
            return Ok(());
        };

        let mut close_now = false;
        match command {
            StreamMailboxCommand::Write { bytes, close, .. } => {
                if !bytes.is_empty() {
                    if state.pending_write.len().saturating_add(bytes.len())
                        > self.max_pending_write_bytes
                    {
                        close_now = true;
                    } else {
                        state.pending_write.extend_from_slice(&bytes);
                    }
                }
                if close {
                    state.close_when_flushed = true;
                }
            }
            StreamMailboxCommand::Close { .. } => {
                state.close_when_flushed = true;
            }
            StreamMailboxCommand::PauseReads { .. } => {
                state.read_paused = true;
            }
            StreamMailboxCommand::ResumeReads { .. } => {
                state.read_paused = false;
                return self.progress_reads_with_state(stream, state);
            }
            StreamMailboxCommand::ResumeAllReads => unreachable!("handled before dispatch"),
        }

        if close_now || self.should_close_after_io(&state) {
            self.close_connection_with_state(stream, state)
        } else {
            self.platform
                .set_stream_interest(stream, self.interest_for(&state))?;
            self.connections.insert(stream, state);
            Ok(())
        }
    }

    fn read_ready(&mut self, stream: StreamId) -> Result<(), PlatformError> {
        let Some(state) = self.connections.remove(&stream) else {
            return Ok(());
        };

        if state.read_paused {
            self.platform
                .set_stream_interest(stream, self.interest_for(&state))?;
            self.connections.insert(stream, state);
            return Ok(());
        }

        self.progress_reads_with_state(stream, state)
    }

    fn progress_reads_with_state(
        &mut self,
        stream: StreamId,
        mut state: ConnectionState<S>,
    ) -> Result<(), PlatformError> {
        if state.pending_write.len() >= self.max_pending_write_bytes {
            self.platform
                .set_stream_interest(stream, self.interest_for(&state))?;
            self.connections.insert(stream, state);
            return Ok(());
        }

        let mut close_now = false;
        let mut buffer = vec![0u8; self.read_buffer_size];

        while let Some(chunk) = state.deferred_reads.pop_front() {
            match self.apply_read_chunk(&mut state, &chunk) {
                ReadChunkOutcome::Applied => {}
                ReadChunkOutcome::Deferred => {
                    state.deferred_reads.push_front(chunk);
                    state.read_paused = true;
                    break;
                }
                ReadChunkOutcome::CloseNow => {
                    close_now = true;
                    break;
                }
            }

            if self.should_close_after_io(&state) {
                break;
            }
        }

        loop {
            if close_now || state.read_paused || state.close_when_flushed || state.peer_closed {
                break;
            }
            match self.platform.read(stream, &mut buffer)? {
                ReadOutcome::Read(n) => {
                    let chunk = &buffer[..n];
                    match self.apply_read_chunk(&mut state, chunk) {
                        ReadChunkOutcome::Applied => {}
                        ReadChunkOutcome::Deferred => {
                            state.deferred_reads.push_back(chunk.to_vec());
                            state.read_paused = true;
                            break;
                        }
                        ReadChunkOutcome::CloseNow => {
                            close_now = true;
                            break;
                        }
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
            self.close_connection_with_state(stream, state)
        } else {
            self.platform
                .set_stream_interest(stream, self.interest_for(&state))?;
            self.connections.insert(stream, state);
            Ok(())
        }
    }

    fn apply_read_chunk(&self, state: &mut ConnectionState<S>, bytes: &[u8]) -> ReadChunkOutcome {
        let result = (self.handler)(state.info, &mut state.app_state, bytes);
        if result.defer_read {
            return ReadChunkOutcome::Deferred;
        }
        if !result.write.is_empty() {
            if state.pending_write.len().saturating_add(result.write.len())
                > self.max_pending_write_bytes
            {
                return ReadChunkOutcome::CloseNow;
            }
            state.pending_write.extend_from_slice(&result.write);
        }
        if result.close {
            state.close_when_flushed = true;
        }
        ReadChunkOutcome::Applied
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
            self.close_connection_with_state(stream, state)
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
                    self.close_connection_with_state(stream, state)
                } else {
                    self.platform
                        .set_stream_interest(stream, self.interest_for(&state))?;
                    self.connections.insert(stream, state);
                    Ok(())
                }
            }
            CloseKind::WriteClosed | CloseKind::FullyClosed | CloseKind::Reset => {
                self.close_connection_with_state(stream, state)
            }
        }
    }

    fn interest_for(&self, state: &ConnectionState<S>) -> StreamInterest {
        if state.pending_write.is_empty() {
            if state.read_paused || state.peer_closed || state.close_when_flushed {
                StreamInterest::none()
            } else {
                StreamInterest::readable()
            }
        } else if state.read_paused || state.peer_closed || state.close_when_flushed {
            StreamInterest {
                readable: false,
                writable: true,
            }
        } else {
            StreamInterest::readable_writable()
        }
    }

    fn should_close_after_io(&self, state: &ConnectionState<S>) -> bool {
        state.pending_write.is_empty() && (state.peer_closed || state.close_when_flushed)
    }

    fn resume_all_reads(&mut self) -> Result<(), PlatformError> {
        let streams = self
            .connections
            .iter()
            .filter_map(|(stream, state)| state.read_paused.then_some(*stream))
            .collect::<Vec<_>>();
        for stream in streams {
            let Some(mut state) = self.connections.remove(&stream) else {
                continue;
            };
            state.read_paused = false;
            self.progress_reads_with_state(stream, state)?;
        }
        Ok(())
    }

    fn close_connection(&mut self, stream: StreamId) -> Result<(), PlatformError> {
        if let Some(state) = self.connections.remove(&stream) {
            self.close_connection_with_state(stream, state)
        } else {
            self.close_stream_raw(stream)
        }
    }

    fn close_connection_with_state(
        &mut self,
        stream: StreamId,
        state: ConnectionState<S>,
    ) -> Result<(), PlatformError> {
        self.connection_index.remove(&state.info.id);
        (self.on_close)(state.info, state.app_state);
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
        self.connection_index.clear();
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
impl StreamReactor<transport_platform::bsd::KqueueTransportPlatform, ()> {
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

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
impl<S: Send + 'static> StreamReactor<transport_platform::bsd::KqueueTransportPlatform, S> {
    pub fn bind_kqueue_with_state<A, I, F, C>(
        addr: A,
        options: StreamReactorOptions,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<Self, PlatformError>
    where
        A: std::net::ToSocketAddrs,
        I: Fn(StreamConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(StreamConnectionInfo, &mut S, &[u8]) -> StreamHandlerResult + Send + Sync + 'static,
        C: Fn(StreamConnectionInfo, S) + Send + Sync + 'static,
    {
        let address = addr
            .to_socket_addrs()
            .map_err(PlatformError::from)?
            .next()
            .ok_or_else(|| PlatformError::Io("no socket addresses resolved".into()))?;
        let platform = transport_platform::bsd::KqueueTransportPlatform::new()?;
        Self::bind_with_state(
            platform,
            BindAddress::Ip(address),
            options,
            init,
            handler,
            on_close,
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
    use std::sync::{Arc, Barrier, Mutex};
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
    fn preserves_connection_state_across_multiple_reads() {
        let mut reactor = StreamReactor::bind_kqueue_with_state(
            ("127.0.0.1", 0),
            StreamReactorOptions::default(),
            |_| Vec::<u8>::new(),
            |_, state, bytes| {
                state.extend_from_slice(bytes);
                if state.ends_with(b"\n") {
                    StreamHandlerResult::write(state.clone())
                } else {
                    StreamHandlerResult::default()
                }
            },
            |_, _| {},
        )
        .expect("bind");
        let addr = reactor.local_addr();

        let mut client = TcpStream::connect(addr).expect("connect");
        client.write_all(b"hel").expect("write first fragment");
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
        reactor.read_ready(stream).expect("first partial read");

        let mut probe = [0u8; 16];
        client
            .set_read_timeout(Some(Duration::from_millis(50)))
            .expect("read timeout");
        let err = client.read(&mut probe).expect_err("no response yet");
        assert!(
            matches!(
                err.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ),
            "expected no response before newline, got {err}"
        );

        client.write_all(b"lo\n").expect("write second fragment");
        for _ in 0..8 {
            reactor.read_ready(stream).expect("second read");
            reactor.write_ready(stream).expect("flush response");
            if reactor
                .connections
                .get(&stream)
                .map(|state| state.pending_write.is_empty())
                .unwrap_or(true)
            {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        let mut echoed = [0u8; 6];
        let mut saw_echo = false;
        for _ in 0..8 {
            if reactor.connections.contains_key(&stream) {
                reactor.read_ready(stream).expect("progress readable state");
                if reactor.connections.contains_key(&stream) {
                    reactor
                        .write_ready(stream)
                        .expect("progress writable state");
                }
            }
            match client.read_exact(&mut echoed) {
                Ok(()) => {
                    saw_echo = true;
                    break;
                }
                Err(err)
                    if matches!(
                        err.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(err) => panic!("read echoed frame: {err}"),
            }
        }
        assert!(
            saw_echo,
            "client should eventually observe the echoed frame"
        );
        assert_eq!(&echoed, b"hello\n");
    }

    #[test]
    fn invokes_close_callback_once_with_final_state() {
        let closed = Arc::new(Mutex::new(Vec::<(ConnectionId, Vec<u8>)>::new()));
        let closed_observer = Arc::clone(&closed);
        let mut reactor = StreamReactor::bind_kqueue_with_state(
            ("127.0.0.1", 0),
            StreamReactorOptions::default(),
            |_| Vec::<u8>::new(),
            |_, state, bytes| {
                state.extend_from_slice(bytes);
                StreamHandlerResult::close()
            },
            move |info, state| {
                closed_observer
                    .lock()
                    .expect("close observer mutex poisoned")
                    .push((info.id, state));
            },
        )
        .expect("bind");
        let addr = reactor.local_addr();

        let mut client = TcpStream::connect(addr).expect("connect");
        client.write_all(b"bye").expect("write request");
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
        for _ in 0..8 {
            if !reactor.connections.contains_key(&stream) {
                break;
            }
            reactor.read_ready(stream).expect("read request");
            thread::sleep(Duration::from_millis(5));
        }

        let observed = closed.lock().expect("close observer mutex poisoned");
        assert_eq!(observed.len(), 1, "close callback should run exactly once");
        assert_eq!(observed[0].1, b"bye".to_vec());
    }

    #[test]
    fn mailbox_can_queue_delayed_writes_after_read_handler_returns() {
        let seen = Arc::new(Mutex::new(Vec::<ConnectionId>::new()));
        let seen_in_handler = Arc::clone(&seen);
        let mut reactor = StreamReactor::bind_kqueue_with_state(
            ("127.0.0.1", 0),
            StreamReactorOptions::default(),
            |_| (),
            move |info, _, _| {
                seen_in_handler
                    .lock()
                    .expect("seen mutex poisoned")
                    .push(info.id);
                StreamHandlerResult::default()
            },
            |_, _| {},
        )
        .expect("bind");
        let mailbox = reactor.mailbox();
        let addr = reactor.local_addr();

        let mut client = TcpStream::connect(addr).expect("connect");
        client.write_all(b"request").expect("write request");
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
        let mut connection_id = None;
        for _ in 0..20 {
            reactor.read_ready(stream).expect("read request");
            if let Some(id) = seen.lock().expect("seen mutex poisoned").first().copied() {
                connection_id = Some(id);
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let connection_id = connection_id.expect("handler should observe the request");

        mailbox.send(connection_id, b"delayed-response".to_vec());
        reactor.drain_mailbox().expect("drain mailbox");
        reactor.write_ready(stream).expect("flush delayed response");

        let mut response = [0u8; 16];
        client
            .read_exact(&mut response)
            .expect("read delayed response");
        assert_eq!(&response, b"delayed-response");
    }

    #[test]
    fn mailbox_resume_replays_deferred_reads_without_losing_bytes() {
        let accepting = Arc::new(AtomicBool::new(false));
        let accepting_in_handler = Arc::clone(&accepting);
        let mut reactor = StreamReactor::bind_kqueue(("127.0.0.1", 0), move |_, bytes| {
            if accepting_in_handler.load(Ordering::SeqCst) {
                StreamHandlerResult::write(bytes.to_vec())
            } else {
                StreamHandlerResult::defer_read()
            }
        })
        .expect("bind");
        let mailbox = reactor.mailbox();
        let addr = reactor.local_addr();

        let mut client = TcpStream::connect(addr).expect("connect");
        client
            .set_read_timeout(Some(Duration::from_millis(50)))
            .expect("read timeout");
        client.write_all(b"queued").expect("write request");
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
            reactor.read_ready(stream).expect("defer first read");
            let Some(state) = reactor.connections.get(&stream) else {
                panic!("connection should remain open while read is deferred");
            };
            if state.read_paused {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let state = reactor
            .connections
            .get(&stream)
            .expect("connection should remain open while read is deferred");
        assert!(state.read_paused, "deferred read should pause readability");
        assert_eq!(state.deferred_reads.len(), 1);

        let mut probe = [0u8; 16];
        let err = client
            .read(&mut probe)
            .expect_err("no response while paused");
        assert!(
            matches!(
                err.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ),
            "expected no response while read is paused, got {err}"
        );

        accepting.store(true, Ordering::SeqCst);
        mailbox.resume_all_reads();
        reactor.drain_mailbox().expect("resume deferred reads");
        reactor
            .write_ready(stream)
            .expect("flush replayed response");

        let mut response = [0u8; 6];
        client
            .read_exact(&mut response)
            .expect("read replayed response");
        assert_eq!(&response, b"queued");
    }

    #[test]
    fn mailbox_can_write_and_close_after_flush() {
        let seen = Arc::new(Mutex::new(Vec::<ConnectionId>::new()));
        let seen_in_handler = Arc::clone(&seen);
        let closed = Arc::new(Mutex::new(Vec::<ConnectionId>::new()));
        let closed_observer = Arc::clone(&closed);
        let mut reactor = StreamReactor::bind_kqueue_with_state(
            ("127.0.0.1", 0),
            StreamReactorOptions::default(),
            |_| (),
            move |info, _, _| {
                seen_in_handler
                    .lock()
                    .expect("seen mutex poisoned")
                    .push(info.id);
                StreamHandlerResult::default()
            },
            move |info, _| {
                closed_observer
                    .lock()
                    .expect("closed mutex poisoned")
                    .push(info.id);
            },
        )
        .expect("bind");
        let mailbox = reactor.mailbox();
        let addr = reactor.local_addr();

        let mut client = TcpStream::connect(addr).expect("connect");
        client.write_all(b"request").expect("write request");
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
        let mut connection_id = None;
        for _ in 0..20 {
            reactor.read_ready(stream).expect("read request");
            if let Some(id) = seen.lock().expect("seen mutex poisoned").first().copied() {
                connection_id = Some(id);
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let connection_id = connection_id.expect("handler should observe the request");

        mailbox.send_and_close(connection_id, b"last".to_vec());
        reactor.drain_mailbox().expect("drain mailbox");
        reactor.write_ready(stream).expect("flush and close");

        let mut response = Vec::new();
        client.read_to_end(&mut response).expect("read to eof");
        assert_eq!(response, b"last");
        assert_eq!(
            closed.lock().expect("closed mutex poisoned").as_slice(),
            &[connection_id]
        );
    }

    #[test]
    fn mailbox_ignores_unknown_or_stale_connection_ids() {
        let mut reactor = StreamReactor::bind_kqueue(("127.0.0.1", 0), |_, bytes| {
            StreamHandlerResult::write(bytes.to_vec())
        })
        .expect("bind");
        let mailbox = reactor.mailbox();

        mailbox.send(ConnectionId(999_999), b"nobody".to_vec());
        mailbox.close(ConnectionId(999_999));
        reactor
            .drain_mailbox()
            .expect("stale mailbox commands should be harmless");
        assert!(reactor.connections.is_empty());
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
