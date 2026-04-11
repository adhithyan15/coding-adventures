//! DT24 TCP server with a pluggable connection-aware handler.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Connection {
    pub id: usize,
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
    pub read_buffer: Vec<u8>,
    pub selected_db: usize,
}

impl Connection {
    fn new(id: usize, peer_addr: SocketAddr, local_addr: SocketAddr) -> Self {
        Self {
            id,
            peer_addr,
            local_addr,
            read_buffer: Vec::new(),
            selected_db: 0,
        }
    }
}

pub type Handler = Arc<dyn Fn(&mut Connection, &[u8]) -> Vec<u8> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct TcpServer {
    inner: Arc<ServerInner>,
}

struct ServerInner {
    host: String,
    port: u16,
    backlog: usize,
    buffer_size: usize,
    handler: Handler,
    listener: Mutex<Option<TcpListener>>,
    connections: Mutex<BTreeMap<usize, ConnectionState>>,
    next_connection_id: AtomicUsize,
    running: AtomicBool,
    stop_flag: AtomicBool,
}

struct ConnectionState {
    stream: TcpStream,
    connection: Connection,
}

impl TcpServer {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self::with_handler(host, port, |_, data| data.to_vec())
    }

    pub fn with_handler<F>(host: impl Into<String>, port: u16, handler: F) -> Self
    where
        F: Fn(&mut Connection, &[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        Self::with_options(host, port, 128, 4096, handler)
    }

    pub fn with_options<F>(
        host: impl Into<String>,
        port: u16,
        backlog: usize,
        buffer_size: usize,
        handler: F,
    ) -> Self
    where
        F: Fn(&mut Connection, &[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(ServerInner {
                host: host.into(),
                port,
                backlog: backlog.max(1),
                buffer_size: buffer_size.max(1),
                handler: Arc::new(handler),
                listener: Mutex::new(None),
                connections: Mutex::new(BTreeMap::new()),
                next_connection_id: AtomicUsize::new(1),
                running: AtomicBool::new(false),
                stop_flag: AtomicBool::new(false),
            }),
        }
    }

    pub fn start(&self) -> io::Result<()> {
        if self.inner.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        let mut guard = self.inner.listener.lock().expect("listener mutex poisoned");
        if guard.is_some() {
            self.inner.running.store(true, Ordering::SeqCst);
            return Ok(());
        }

        let listener = TcpListener::bind((self.inner.host.as_str(), self.inner.port))?;
        listener.set_nonblocking(true)?;
        let _ = self.inner.backlog;
        *guard = Some(listener);
        self.inner.running.store(true, Ordering::SeqCst);
        self.inner.stop_flag.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub fn serve(&self) -> io::Result<()> {
        self.start()?;
        while !self.inner.stop_flag.load(Ordering::SeqCst) {
            let mut did_work = false;
            did_work |= self.accept_pending()?;
            did_work |= self.process_connections()?;
            if !did_work {
                thread::sleep(Duration::from_millis(10));
            }
        }
        self.cleanup();
        Ok(())
    }

    pub fn serve_forever(&self) -> io::Result<()> {
        self.start()?;
        self.serve()
    }

    /// Run the server's handler against a connection without going through TCP.
    ///
    /// This is the transport-agnostic seam used by higher-level crates and
    /// by tests that want to exercise the actual server logic without
    /// depending on sockets or scheduling.
    pub fn handle(&self, connection: &mut Connection, data: &[u8]) -> Vec<u8> {
        (self.inner.handler)(connection, data)
    }

    pub fn stop(&self) {
        self.inner.stop_flag.store(true, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst)
    }

    pub fn address(&self) -> Option<SocketAddr> {
        self.inner
            .listener
            .lock()
            .expect("listener mutex poisoned")
            .as_ref()
            .and_then(|listener| listener.local_addr().ok())
    }

    pub fn try_address(&self) -> io::Result<SocketAddr> {
        self.address()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "server has not been started"))
    }

    fn accept_pending(&self) -> io::Result<bool> {
        let listener_guard = self.inner.listener.lock().expect("listener mutex poisoned");
        let Some(listener) = listener_guard.as_ref() else {
            return Ok(false);
        };
        let listener = listener.try_clone()?;
        drop(listener_guard);

        let mut did_work = false;
        loop {
            match listener.accept() {
                Ok((stream, peer_addr)) => {
                    did_work = true;
                    stream.set_nonblocking(true)?;
                    let local_addr = stream.local_addr().unwrap_or_else(|_| {
                        self.address().unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 0)))
                    });
                    let id = self.inner.next_connection_id.fetch_add(1, Ordering::SeqCst);
                    let connection = Connection::new(id, peer_addr, local_addr);
                    self.inner
                        .connections
                        .lock()
                        .expect("connections mutex poisoned")
                        .insert(id, ConnectionState { stream, connection });
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(err) => return Err(err),
            }
        }
        Ok(did_work)
    }

    fn process_connections(&self) -> io::Result<bool> {
        let ids = {
            let guard = self.inner.connections.lock().expect("connections mutex poisoned");
            guard.keys().copied().collect::<Vec<_>>()
        };

        let mut did_work = false;
        let mut to_reinsert = Vec::new();

        for id in ids {
            let mut state = {
                let mut guard = self.inner.connections.lock().expect("connections mutex poisoned");
                guard.remove(&id)
            };

            let Some(mut state) = state.take() else {
                continue;
            };

            let mut remove_connection = false;
            let mut buffer = vec![0u8; self.inner.buffer_size];

            loop {
                match state.stream.read(&mut buffer) {
                    Ok(0) => {
                        remove_connection = true;
                        did_work = true;
                        break;
                    }
                    Ok(n) => {
                    did_work = true;
                        let response = self.handle(&mut state.connection, &buffer[..n]);
                        if !response.is_empty() {
                            if let Err(err) = state.stream.write_all(&response) {
                                match err.kind() {
                                    io::ErrorKind::BrokenPipe
                                    | io::ErrorKind::ConnectionReset
                                    | io::ErrorKind::WouldBlock => {
                                        remove_connection = true;
                                    }
                                    _ => return Err(err),
                                }
                                break;
                            }
                        }
                        if n < buffer.len() {
                            break;
                        }
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                    Err(err)
                        if matches!(
                            err.kind(),
                            io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset
                        ) =>
                    {
                        remove_connection = true;
                        break;
                    }
                    Err(err) => return Err(err),
                }
            }

            if !remove_connection {
                to_reinsert.push((id, state));
            }
        }

        if !to_reinsert.is_empty() {
            let mut guard = self.inner.connections.lock().expect("connections mutex poisoned");
            for (id, state) in to_reinsert {
                guard.insert(id, state);
            }
        }

        Ok(did_work)
    }

    fn cleanup(&self) {
        let mut connections = self.inner.connections.lock().expect("connections mutex poisoned");
        connections.clear();
        let mut listener = self.inner.listener.lock().expect("listener mutex poisoned");
        *listener = None;
        self.inner.running.store(false, Ordering::SeqCst);
    }
}

impl fmt::Debug for TcpServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.is_running() {
            "running"
        } else {
            "stopped"
        };
        f.debug_struct("TcpServer")
            .field("host", &self.inner.host)
            .field("port", &self.inner.port)
            .field("status", &status)
            .finish()
    }
}

impl fmt::Display for TcpServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl Drop for TcpServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_connection() -> Connection {
        Connection {
            id: 1,
            peer_addr: SocketAddr::from(([127, 0, 0, 1], 45_001)),
            local_addr: SocketAddr::from(([127, 0, 0, 1], 63_079)),
            read_buffer: Vec::new(),
            selected_db: 0,
        }
    }

    fn invoke(server: &TcpServer, connection: &mut Connection, data: &[u8]) -> Vec<u8> {
        server.handle(connection, data)
    }

    #[test]
    fn handler_round_trips_bytes_without_tcp() {
        let server = TcpServer::new("127.0.0.1", 0);
        let mut connection = make_connection();
        let response = invoke(&server, &mut connection, b"hello");
        assert_eq!(response, b"hello");
        assert_eq!(connection.selected_db, 0);
    }

    #[test]
    fn custom_handler_can_transform_bytes() {
        let server = TcpServer::with_handler("127.0.0.1", 0, |_, data| {
            data.iter().map(|byte| byte.to_ascii_uppercase()).collect()
        });
        let mut connection = make_connection();
        let response = invoke(&server, &mut connection, b"hello");
        assert_eq!(response, b"HELLO");
    }

    #[test]
    fn stateful_handler_can_use_connection_buffer() {
        let server = TcpServer::with_handler("127.0.0.1", 0, |conn, data| {
            conn.read_buffer.extend_from_slice(data);
            if conn.read_buffer.len() < 6 {
                Vec::new()
            } else {
                let response = conn.read_buffer.clone();
                conn.read_buffer.clear();
                response
            }
        });
        let mut connection = make_connection();
        assert!(invoke(&server, &mut connection, b"buf").is_empty());
        let response = invoke(&server, &mut connection, b"fer");
        assert_eq!(response, b"buffer");
        assert!(connection.read_buffer.is_empty());
    }

    #[test]
    fn configuration_helpers_cover_introspection_and_debug_output() {
        let server = TcpServer::with_options("127.0.0.1", 0, 0, 0, |_, data| data.to_vec());

        assert!(!server.is_running());
        assert!(server.address().is_none());
        assert!(server.try_address().is_err());

        let debug = format!("{server:?}");
        assert_eq!(debug, format!("{server}"));
        assert!(debug.contains("TcpServer"));
        assert!(debug.contains("stopped"));

        server.start().expect("start");
        server.start().expect("second start should be a no-op");
        assert!(server.is_running());
        assert!(server.address().is_some());
        assert!(server.try_address().is_ok());
        server.stop();
    }
}
