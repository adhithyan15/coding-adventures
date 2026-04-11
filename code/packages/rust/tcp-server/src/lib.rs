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
                        let response = (self.inner.handler)(&mut state.connection, &buffer[..n]);
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
    use std::io;
    use std::net::TcpStream;
    use std::thread;
    use std::time::Instant;

    fn send_recv(port: u16, data: &[u8], read_timeout: std::time::Duration) -> io::Result<Vec<u8>> {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream.write_all(data).expect("write");
        stream.set_read_timeout(Some(read_timeout)).expect("timeout");
        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf)?;
        buf.truncate(n);
        Ok(buf)
    }

    fn wait_until_ready(port: u16, request: &[u8], expected_response: &[u8]) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(response) = send_recv(port, request, Duration::from_millis(100)) {
                if response == expected_response {
                    return;
                }
            }

            if Instant::now() >= deadline {
                panic!("server never became ready");
            }

            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn echo_server_round_trips() {
        let server = TcpServer::new("127.0.0.1", 0);
        server.start().expect("start");
        let port = server.try_address().expect("address").port();
        let runner = server.clone();
        let handle = thread::spawn(move || runner.serve_forever().unwrap());
        wait_until_ready(port, b"ready", b"ready");

        let response = send_recv(port, b"hello", Duration::from_secs(2)).expect("read");
        assert_eq!(response, b"hello");

        server.stop();
        handle.join().unwrap();
    }

    #[test]
    fn custom_handler_can_transform_bytes() {
        let server = TcpServer::with_handler("127.0.0.1", 0, |_, data| {
            data.iter().map(|byte| byte.to_ascii_uppercase()).collect()
        });
        server.start().expect("start");
        let port = server.try_address().expect("address").port();
        let runner = server.clone();
        let handle = thread::spawn(move || runner.serve_forever().unwrap());
        wait_until_ready(port, b"ready", b"READY");

        let response = send_recv(port, b"hello", Duration::from_secs(2)).expect("read");
        assert_eq!(response, b"HELLO");

        server.stop();
        handle.join().unwrap();
    }

    #[test]
    fn stateful_handler_can_use_connection_buffer() {
        let server = TcpServer::with_handler("127.0.0.1", 0, |conn, data| {
            conn.read_buffer.extend_from_slice(data);
            let response = conn.read_buffer.clone();
            conn.read_buffer.clear();
            response
        });
        server.start().expect("start");
        let port = server.try_address().expect("address").port();
        let runner = server.clone();
        let handle = thread::spawn(move || runner.serve_forever().unwrap());
        wait_until_ready(port, b"ready", b"ready");

        let response = send_recv(port, b"buffer", Duration::from_secs(2)).expect("read");
        assert_eq!(response, b"buffer");

        server.stop();
        handle.join().unwrap();
    }
}
