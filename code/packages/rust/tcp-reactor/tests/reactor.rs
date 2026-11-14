//! Cross-platform behaviour tests for `tcp-reactor`.
//!
//! These use only the public API (`bind_native` and plain `std::net` clients), so
//! the same assertions run against epoll on Linux, kqueue on macOS/BSD and the
//! I/O completion port engine on Windows.

use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::{Arc, Barrier};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tcp_reactor::{bind_native, ConnectionInfo, NativeTcpReactor, StopHandle};

/// A reactor running on its own thread.
struct Running {
    addr: SocketAddr,
    stop: StopHandle,
    thread: Option<JoinHandle<io::Result<()>>>,
}

impl Running {
    fn start<F>(handler: F, configure: impl FnOnce(&mut NativeTcpReactor)) -> Self
    where
        F: Fn(ConnectionInfo, &[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        let mut reactor = bind_native(("127.0.0.1", 0), handler).expect("bind");
        configure(&mut reactor);
        let addr = reactor.local_addr();
        let stop = reactor.stop_handle();
        let thread = thread::spawn(move || reactor.serve());
        Self {
            addr,
            stop,
            thread: Some(thread),
        }
    }

    fn connect(&self) -> TcpStream {
        let stream = TcpStream::connect(self.addr).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("read timeout");
        stream
    }

    /// Stop the reactor and require `serve` to return promptly and cleanly.
    fn finish(mut self) {
        self.stop.stop();
        let started = Instant::now();
        let result = self
            .thread
            .take()
            .expect("thread")
            .join()
            .expect("serve thread");
        assert!(result.is_ok(), "serve should exit cleanly: {result:?}");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "serve took {:?} to stop",
            started.elapsed()
        );
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        // A failed assertion must not leave a reactor thread spinning forever.
        self.stop.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// True when the peer closed (or reset) the connection without sending data.
fn closed_without_data(stream: &mut TcpStream) -> bool {
    let mut buf = [0u8; 16];
    match stream.read(&mut buf) {
        Ok(0) => true,
        Ok(_) => false,
        Err(err) => matches!(
            err.kind(),
            io::ErrorKind::ConnectionReset
                | io::ErrorKind::ConnectionAborted
                | io::ErrorKind::BrokenPipe
        ),
    }
}

#[test]
fn serves_many_concurrent_echo_clients_without_crashing() {
    let server = Running::start(|_, bytes| bytes.to_vec(), |_| {});
    let addr = server.addr;

    let client_count = 24usize;
    let barrier = Arc::new(Barrier::new(client_count));
    let clients: Vec<_> = (0..client_count)
        .map(|i| {
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || -> io::Result<Vec<u8>> {
                barrier.wait();
                let mut stream = TcpStream::connect(addr)?;
                stream.set_read_timeout(Some(Duration::from_secs(10)))?;
                let payload = format!("client-{i}-payload").into_bytes();
                stream.write_all(&payload)?;
                stream.shutdown(Shutdown::Write)?;
                let mut echoed = Vec::new();
                stream.read_to_end(&mut echoed)?;
                Ok(echoed)
            })
        })
        .collect();

    for (i, client) in clients.into_iter().enumerate() {
        let echoed = client.join().expect("client thread").expect("client io");
        assert_eq!(echoed, format!("client-{i}-payload").into_bytes());
    }
    server.finish();
}

#[test]
fn reports_peer_and_local_addresses_to_the_handler() {
    let (tx, rx) = std::sync::mpsc::channel();
    let tx = std::sync::Mutex::new(tx);
    let server = Running::start(
        move |info, bytes| {
            let _ = tx.lock().unwrap().send(info);
            bytes.to_vec()
        },
        |_| {},
    );

    let mut client = server.connect();
    client.write_all(b"hi").expect("write");
    let info = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("handler call");
    assert_eq!(info.peer_addr, client.local_addr().expect("client addr"));
    assert_eq!(info.local_addr, server.addr);
    drop(client);
    server.finish();
}

#[test]
fn rejects_connections_above_the_configured_cap() {
    let server = Running::start(
        |_, bytes| bytes.to_vec(),
        |reactor| reactor.set_max_connections(2),
    );

    // Fill the cap and prove both are live.
    let mut a = server.connect();
    let mut b = server.connect();
    for client in [&mut a, &mut b] {
        client.write_all(b"ping").expect("write");
        let mut echoed = [0u8; 4];
        client.read_exact(&mut echoed).expect("echo");
        assert_eq!(&echoed, b"ping");
    }

    // A third connection is closed without being served.
    let mut c = server.connect();
    let _ = c.write_all(b"ping");
    assert!(
        closed_without_data(&mut c),
        "a client over the cap must be closed, not served"
    );

    // The admitted clients are unaffected.
    a.write_all(b"again").expect("write");
    let mut echoed = [0u8; 5];
    a.read_exact(&mut echoed).expect("echo");
    assert_eq!(&echoed, b"again");

    // Freeing a slot admits a new client.
    drop(b);
    let mut admitted = false;
    for _ in 0..50 {
        let mut d = server.connect();
        if d.write_all(b"ok").is_ok() {
            let mut echoed = [0u8; 2];
            if d.read_exact(&mut echoed).is_ok() && &echoed == b"ok" {
                admitted = true;
                break;
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(admitted, "a freed slot should admit a new client");
    server.finish();
}

#[test]
fn closes_connections_that_exceed_the_pending_write_budget() {
    let server = Running::start(
        |_, _| vec![1u8; 64],
        |reactor| reactor.set_max_pending_write_bytes(32),
    );

    let mut client = server.connect();
    client.write_all(b"trigger").expect("write request");
    assert!(
        closed_without_data(&mut client),
        "an over-budget response must close the connection and never be sent"
    );
    server.finish();
}

#[test]
fn delivers_a_large_response_to_a_slow_reader_in_order() {
    // 8 MiB is far larger than any socket buffer, so the reactor must queue,
    // write partially, wait, and resume, without reordering or losing bytes.
    const TOTAL: usize = 8 * 1024 * 1024;
    let server = Running::start(
        |_, _| (0..TOTAL).map(|i| (i % 251) as u8).collect(),
        |reactor| reactor.set_max_pending_write_bytes(16 * 1024 * 1024),
    );

    let mut client = server.connect();
    client.write_all(b"go").expect("write");
    // Let the server hit a full socket buffer before reading anything.
    thread::sleep(Duration::from_millis(300));

    let mut received = Vec::with_capacity(TOTAL);
    let mut chunk = vec![0u8; 64 * 1024];
    while received.len() < TOTAL {
        let n = client.read(&mut chunk).expect("read");
        assert!(
            n > 0,
            "server closed after {} of {TOTAL} bytes",
            received.len()
        );
        received.extend_from_slice(&chunk[..n]);
        // Read slowly enough that the server keeps hitting a full send buffer.
        if received.len() % (1024 * 1024) < chunk.len() {
            thread::sleep(Duration::from_millis(5));
        }
    }
    assert_eq!(received.len(), TOTAL);
    assert!(
        received
            .iter()
            .enumerate()
            .all(|(i, &b)| b == (i % 251) as u8),
        "response bytes were reordered or corrupted"
    );
    drop(client);
    server.finish();
}

#[test]
fn handles_many_small_requests_on_one_connection() {
    let server = Running::start(|_, bytes| bytes.to_vec(), |_| {});
    let mut client = server.connect();
    for i in 0..200u32 {
        let message = format!("message-{i}");
        client.write_all(message.as_bytes()).expect("write");
        let mut echoed = vec![0u8; message.len()];
        client.read_exact(&mut echoed).expect("echo");
        assert_eq!(echoed, message.as_bytes());
    }
    drop(client);
    server.finish();
}

#[test]
fn stop_returns_promptly_and_serve_can_run_again() {
    let mut reactor = bind_native(("127.0.0.1", 0), |_, bytes| bytes.to_vec()).expect("bind");
    let addr = reactor.local_addr();
    let stop = reactor.stop_handle();

    // First run, with a live connection that stop must close.
    let (reactor, mut first_client) = {
        let handle = thread::spawn(move || {
            let result = reactor.serve();
            (reactor, result)
        });
        let mut client = TcpStream::connect(addr).expect("connect");
        client
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("timeout");
        client.write_all(b"one").expect("write");
        let mut echoed = [0u8; 3];
        client.read_exact(&mut echoed).expect("echo");
        assert_eq!(&echoed, b"one");

        stop.stop();
        let started = Instant::now();
        let (reactor, result) = handle.join().expect("serve thread");
        assert!(result.is_ok(), "first serve: {result:?}");
        assert!(started.elapsed() < Duration::from_secs(5), "stop was slow");
        (reactor, client)
    };
    assert!(
        closed_without_data(&mut first_client),
        "stopping must close live connections"
    );

    // Second run on the same reactor.
    let mut reactor = reactor;
    let stop = reactor.stop_handle();
    let handle = thread::spawn(move || reactor.serve());
    let mut client = TcpStream::connect(addr).expect("reconnect");
    client
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("timeout");
    client.write_all(b"two").expect("write");
    let mut echoed = [0u8; 3];
    client.read_exact(&mut echoed).expect("echo");
    assert_eq!(&echoed, b"two");
    drop(client);
    stop.stop();
    assert!(handle.join().expect("serve thread").is_ok());
}

#[test]
fn dropping_reactors_releases_everything() {
    // Idle reactor that never served.
    drop(bind_native(("127.0.0.1", 0), |_, bytes| bytes.to_vec()).expect("bind"));

    // Reactor dropped right after serving with connections still open.
    let mut reactor = bind_native(("127.0.0.1", 0), |_, bytes| bytes.to_vec()).expect("bind");
    let addr = reactor.local_addr();
    let stop = reactor.stop_handle();
    let handle = thread::spawn(move || {
        let _ = reactor.serve();
        // `reactor` is dropped here with the engine's state intact.
    });
    let mut clients: Vec<TcpStream> = (0..8)
        .map(|_| {
            let mut c = TcpStream::connect(addr).expect("connect");
            c.set_read_timeout(Some(Duration::from_secs(10)))
                .expect("timeout");
            c.write_all(b"x").expect("write");
            let mut echoed = [0u8; 1];
            c.read_exact(&mut echoed).expect("echo");
            c
        })
        .collect();
    stop.stop();
    handle.join().expect("serve thread");
    for client in &mut clients {
        assert!(closed_without_data(client), "connection should be closed");
    }
}
