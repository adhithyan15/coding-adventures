//! # tcp-reactor
//!
//! Small nonblocking TCP reactor built on top of `native-event-core`.
//!
//! The reactor is intentionally transport-focused. It accepts connections,
//! forwards readable bytes to a handler, buffers replies, and flushes queued
//! writes when the backend reports writable readiness.

use native_event_core::{
    EventBackend, Interest, NativeEventLoop, PollTimeout, SourceKind, SourceRef, Token,
};
use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub type ConnectionId = u64;
pub type Handler = Arc<dyn Fn(ConnectionInfo, &[u8]) -> Vec<u8> + Send + Sync + 'static>;

const LISTENER_TOKEN: Token = Token(1);
const DEFAULT_MAX_CONNECTIONS: usize = 1_024;
const DEFAULT_MAX_PENDING_WRITE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub id: ConnectionId,
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
}

#[derive(Clone)]
pub struct StopHandle(Arc<AtomicBool>);

impl StopHandle {
    pub fn stop(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

struct ConnectionState {
    info: ConnectionInfo,
    stream: TcpStream,
    pending_write: Vec<u8>,
    peer_closed: bool,
}

pub struct TcpReactor<B> {
    loop_: NativeEventLoop<B>,
    listener: TcpListener,
    listener_addr: SocketAddr,
    connections: BTreeMap<Token, ConnectionState>,
    next_token: AtomicU64,
    stop_flag: Arc<AtomicBool>,
    read_buffer_size: usize,
    max_connections: usize,
    max_pending_write_bytes: usize,
    handler: Handler,
}

impl<B: EventBackend> TcpReactor<B> {
    pub fn with_backend<F>(listener: TcpListener, backend: B, handler: F) -> io::Result<Self>
    where
        F: Fn(ConnectionInfo, &[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        listener.set_nonblocking(true)?;
        let listener_addr = listener.local_addr()?;
        let mut loop_ = NativeEventLoop::new(backend);
        loop_.register(
            SourceRef::from_fd(listener.as_raw_fd()),
            LISTENER_TOKEN,
            SourceKind::Io,
            Interest::READABLE,
        )?;

        Ok(Self {
            loop_,
            listener,
            listener_addr,
            connections: BTreeMap::new(),
            next_token: AtomicU64::new(2),
            stop_flag: Arc::new(AtomicBool::new(false)),
            read_buffer_size: 4096,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            max_pending_write_bytes: DEFAULT_MAX_PENDING_WRITE_BYTES,
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

    pub fn serve(&mut self) -> io::Result<()> {
        self.stop_flag.store(false, Ordering::SeqCst);
        while !self.stop_flag.load(Ordering::SeqCst) {
            let events = self
                .loop_
                .poll_once(PollTimeout::After(Duration::from_millis(10)))?;
            for event in events {
                if event.token == LISTENER_TOKEN {
                    self.accept_ready()?;
                    continue;
                }

                if event.error {
                    self.close_connection(event.token)?;
                    continue;
                }
                if event.readable {
                    self.read_ready(event.token)?;
                }
                if event.writable {
                    self.write_ready(event.token)?;
                }
                if event.hangup && !event.readable && !event.writable {
                    self.close_connection(event.token)?;
                }
            }
        }
        self.shutdown_all()
    }

    fn accept_ready(&mut self) -> io::Result<()> {
        loop {
            match self.listener.accept() {
                Ok((stream, peer_addr)) => {
                    stream.set_nonblocking(true)?;
                    if self.connections.len() >= self.max_connections {
                        let _ = stream.shutdown(Shutdown::Both);
                        continue;
                    }
                    let local_addr = stream.local_addr()?;
                    let token = Token(self.next_token.fetch_add(1, Ordering::SeqCst));
                    let info = ConnectionInfo {
                        id: token.0,
                        peer_addr,
                        local_addr,
                    };
                    self.loop_.register(
                        SourceRef::from_fd(stream.as_raw_fd()),
                        token,
                        SourceKind::Io,
                        Interest::READABLE,
                    )?;
                    self.connections.insert(
                        token,
                        ConnectionState {
                            info,
                            stream,
                            pending_write: Vec::new(),
                            peer_closed: false,
                        },
                    );
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => return Ok(()),
                Err(err) => return Err(err),
            }
        }
    }

    fn read_ready(&mut self, token: Token) -> io::Result<()> {
        let mut close_after_read = false;
        let mut interest = Interest::READABLE;
        let mut buffer = vec![0u8; self.read_buffer_size];

        if let Some(state) = self.connections.get_mut(&token) {
            if state.pending_write.len() >= self.max_pending_write_bytes {
                return self.reregister(token, Interest::WRITABLE);
            }

            loop {
                match state.stream.read(&mut buffer) {
                    Ok(0) => {
                        state.peer_closed = true;
                        break;
                    }
                    Ok(n) => {
                        let response = (self.handler)(state.info, &buffer[..n]);
                        if !response.is_empty() {
                            if state
                                .pending_write
                                .len()
                                .saturating_add(response.len())
                                > self.max_pending_write_bytes
                            {
                                close_after_read = true;
                                break;
                            }
                            state.pending_write.extend_from_slice(&response);
                            interest |= Interest::WRITABLE;
                        }
                        if n < buffer.len() {
                            break;
                        }
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                    Err(err)
                        if matches!(
                            err.kind(),
                            io::ErrorKind::BrokenPipe
                                | io::ErrorKind::ConnectionReset
                                | io::ErrorKind::ConnectionAborted
                        ) =>
                    {
                        close_after_read = true;
                        break;
                    }
                    Err(err) => return Err(err),
                }
            }

            if state.peer_closed {
                if state.pending_write.is_empty() {
                    close_after_read = true;
                } else {
                    interest = Interest::WRITABLE;
                }
            } else if !state.pending_write.is_empty() {
                interest |= Interest::WRITABLE;
            }
        } else {
            return Ok(());
        }

        if close_after_read {
            self.close_connection(token)
        } else {
            self.reregister(token, interest)
        }
    }

    fn write_ready(&mut self, token: Token) -> io::Result<()> {
        let mut close_after_write = false;
        let mut interest = Interest::READABLE;

        if let Some(state) = self.connections.get_mut(&token) {
            while !state.pending_write.is_empty() {
                match state.stream.write(&state.pending_write) {
                    Ok(0) => {
                        close_after_write = true;
                        break;
                    }
                    Ok(n) => {
                        state.pending_write.drain(..n);
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                    Err(err)
                        if matches!(
                            err.kind(),
                            io::ErrorKind::BrokenPipe
                                | io::ErrorKind::ConnectionReset
                                | io::ErrorKind::ConnectionAborted
                        ) =>
                    {
                        close_after_write = true;
                        break;
                    }
                    Err(err) => return Err(err),
                }
            }

            if !state.pending_write.is_empty() {
                interest |= Interest::WRITABLE;
            } else if state.peer_closed {
                close_after_write = true;
            }
        } else {
            return Ok(());
        }

        if close_after_write {
            self.close_connection(token)
        } else {
            self.reregister(token, interest)
        }
    }

    fn reregister(&mut self, token: Token, interest: Interest) -> io::Result<()> {
        if let Some(state) = self.connections.get(&token) {
            self.loop_.reregister(
                SourceRef::from_fd(state.stream.as_raw_fd()),
                token,
                SourceKind::Io,
                interest,
            )
        } else {
            Ok(())
        }
    }

    fn close_connection(&mut self, token: Token) -> io::Result<()> {
        if let Some(state) = self.connections.remove(&token) {
            let source = SourceRef::from_fd(state.stream.as_raw_fd());
            self.loop_.deregister(source, token)?;
        }
        Ok(())
    }

    fn shutdown_all(&mut self) -> io::Result<()> {
        let tokens = self.connections.keys().copied().collect::<Vec<_>>();
        for token in tokens {
            self.close_connection(token)?;
        }
        Ok(())
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
impl TcpReactor<native_event_core::bsd::KqueueBackend> {
    pub fn bind_kqueue<A, F>(addr: A, handler: F) -> io::Result<Self>
    where
        A: std::net::ToSocketAddrs,
        F: Fn(ConnectionInfo, &[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        let listener = TcpListener::bind(addr)?;
        let backend = native_event_core::bsd::KqueueBackend::new()?;
        Self::with_backend(listener, backend, handler)
    }
}

#[cfg(not(unix))]
compile_error!("tcp-reactor currently requires Unix-style file descriptors");

#[cfg(unix)]
use std::os::fd::AsRawFd;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    #[test]
    fn serves_many_concurrent_echo_clients_without_crashing() {
        let mut reactor =
            TcpReactor::bind_kqueue(("127.0.0.1", 0), |_, bytes| bytes.to_vec()).expect("bind");
        let addr = reactor.local_addr();
        let stop = reactor.stop_handle();

        let server = thread::spawn(move || reactor.serve());

        let client_count = 24usize;
        let barrier = Arc::new(Barrier::new(client_count));
        let mut clients = Vec::new();

        for i in 0..client_count {
            let barrier = Arc::clone(&barrier);
            clients.push(thread::spawn(move || -> io::Result<Vec<u8>> {
                barrier.wait();
                let mut stream = TcpStream::connect(addr)?;
                let payload = format!("client-{i}-payload").into_bytes();
                stream.write_all(&payload)?;
                stream.shutdown(Shutdown::Write)?;

                let mut echoed = Vec::new();
                stream.read_to_end(&mut echoed)?;
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

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    #[test]
    fn rejects_connections_above_the_configured_cap() {
        let mut reactor =
            TcpReactor::bind_kqueue(("127.0.0.1", 0), |_, bytes| bytes.to_vec()).expect("bind");
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
        assert_eq!(reactor.connections.len(), 2, "two clients should be admitted");

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

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    #[test]
    fn closes_connections_that_exceed_the_pending_write_budget() {
        let mut reactor = TcpReactor::bind_kqueue(("127.0.0.1", 0), |_, _| vec![1u8; 64])
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

        let token = *reactor
            .connections
            .keys()
            .next()
            .expect("accepted connection token");
        for _ in 0..20 {
            reactor
                .read_ready(token)
                .expect("overflowing write budget should close cleanly");
            if !reactor.connections.contains_key(&token) {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            !reactor.connections.contains_key(&token),
            "reactor should drop connections whose queued output exceeds the limit"
        );
    }
}
