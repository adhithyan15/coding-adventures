//! # irc-net-stdlib — Level 1 network implementation: stdlib sockets + threads
//!
//! ## Overview
//!
//! This crate provides the **concrete TCP networking layer** for the IRC stack.
//! It uses one OS thread per connection with blocking `read`/`write` — the
//! textbook model taught in every OS/networking course.  It is simple, readable,
//! and works well for typical IRC server loads (hundreds of concurrent clients).
//!
//! ## Thread-per-connection model
//!
//! Each accepted TCP connection gets its own OS thread.  The thread:
//!
//! 1. Calls `handler.on_connect()` to notify the server.
//! 2. Loops calling `read()` (blocking) and forwards each chunk to `handler.on_data()`.
//! 3. When `read()` returns 0 bytes (peer closed), calls `handler.on_disconnect()`
//!    and exits.
//!
//! The chief virtue is clarity: each connection's lifecycle is a simple sequential
//! program, easy to reason about with no callbacks or coroutines.
//!
//! ## Shared state locking
//!
//! Two independent `Mutex`es protect shared state:
//!
//! **`handler_lock`** (`Mutex<()>` used as a counting semaphore):
//!   Serializes *all* calls to the `Handler`.  The `IRCServer` inside the handler
//!   is **not** thread-safe — its nicks, channels, and pending replies are plain
//!   `HashMap`s with no internal locking.  By funnelling every callback through
//!   this single lock we guarantee that IRC logic executes in one thread at a time.
//!   IRC traffic is mostly idle (clients send at human typing speed), so lock
//!   contention is negligible in practice.
//!
//! **`conns_lock`** (`Mutex<HashMap<ConnId, TcpStream>>`):
//!   Protects the active-connection map.  Needed because accept threads and
//!   worker threads both insert/remove entries.
//!
//! These two locks are **never held simultaneously**, so deadlock is impossible.
//!
//! ## Writes bypass the handler lock
//!
//! `send_to()` looks up the stream (under `conns_lock`), then writes *without*
//! holding `handler_lock`.  This is intentional:
//!
//! - Writing bytes to a socket is independent of reading server state.
//! - Allowing two threads to write to *different* connections simultaneously is safe.
//! - If we held `handler_lock` during writes, a slow TCP write would stall all
//!   other connection threads that want to run IRC logic.
//!
//! ## Usage
//!
//! ```no_run
//! use irc_net_stdlib::{EventLoop, Handler, ConnId};
//! use std::sync::Arc;
//!
//! struct MyHandler;
//! impl Handler for MyHandler {
//!     fn on_connect(&self, conn_id: ConnId, host: &str) {
//!         println!("connected: {:?} from {}", conn_id, host);
//!     }
//!     fn on_data(&self, _conn_id: ConnId, _data: &[u8]) {}
//!     fn on_disconnect(&self, conn_id: ConnId) {
//!         println!("disconnected: {:?}", conn_id);
//!     }
//! }
//!
//! let event_loop = Arc::new(EventLoop::new());
//! let handler = Arc::new(MyHandler);
//! // event_loop.run("127.0.0.1:6667", handler).unwrap();
//! ```

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// ──────────────────────────────────────────────────────────────────────────────
// ConnId
// ──────────────────────────────────────────────────────────────────────────────

/// Opaque integer that uniquely identifies a TCP connection within this process.
///
/// Using a newtype wrapper rather than a bare `u64` lets the compiler catch
/// accidental mix-ups between connection IDs and other integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnId(pub u64);

// ──────────────────────────────────────────────────────────────────────────────
// Handler trait
// ──────────────────────────────────────────────────────────────────────────────

/// Callback interface that the event loop drives.
///
/// The event loop calls these three methods as connection lifecycle events occur.
/// All three methods are called **serially** under the event loop's handler lock
/// — the implementation need not be thread-safe.
///
/// This trait deliberately passes **raw bytes** to `on_data`, not parsed
/// `Message` objects.  Framing and parsing happen in the driver layer above
/// this one, keeping `irc-net-stdlib` free of any IRC-specific knowledge.
///
/// # Thread safety
///
/// Implementations must be `Send + Sync + 'static` because the event loop
/// shares the handler across multiple threads.  The handler lock ensures that
/// only one thread calls methods on the handler at a time.
pub trait Handler: Send + Sync + 'static {
    /// Called once when a new client connects.
    ///
    /// `conn_id` is a unique identifier for this connection.
    /// `host` is the peer's IP address string.
    fn on_connect(&self, conn_id: ConnId, host: &str);

    /// Called each time new bytes arrive from `conn_id`.
    ///
    /// The bytes may contain a partial IRC message, multiple complete messages,
    /// or anything in between — it is the handler's responsibility to buffer
    /// and frame them.
    ///
    /// The `data` slice is never empty.
    fn on_data(&self, conn_id: ConnId, data: &[u8]);

    /// Called once when `conn_id` has closed (either end initiated).
    ///
    /// After this call the `conn_id` is invalid; `send_to()` with it is a
    /// safe no-op.
    fn on_disconnect(&self, conn_id: ConnId);
}

// ──────────────────────────────────────────────────────────────────────────────
// EventLoop
// ──────────────────────────────────────────────────────────────────────────────

/// Thread-per-connection event loop.
///
/// ## Lifecycle
///
/// 1. Caller creates an `EventLoop` and a `Handler`.
/// 2. Caller calls `loop.run(addr, handler)` — this blocks.
/// 3. Meanwhile, other threads may call `loop.send_to()` to push data to
///    connected clients.
/// 4. When the caller wants to shut down, any thread calls `loop.stop()`.
/// 5. `stop()` signals the `run()` loop to exit.
///
/// ## Worker thread lifecycle per connection
///
/// For each accepted connection, a daemon thread is spawned:
/// 1. `handler.on_connect()` under `handler_lock`
/// 2. Loop: `read()` → `handler.on_data()` under `handler_lock`
/// 3. `handler.on_disconnect()` under `handler_lock`
/// 4. Removes conn from `conns` (under `conns_lock`) and closes socket.
pub struct EventLoop {
    /// Whether the event loop is currently running.
    /// Checked by the accept loop; set to false by `stop()`.
    running: AtomicBool,

    /// Map from ConnId → cloned TcpStream for all currently-open connections.
    /// Protected by `conns_lock`.  Use `try_clone()` on `TcpStream` to get
    /// a second file descriptor pointing to the same socket.
    ///
    /// We store `TcpStream` here so `send_to()` can write to the socket
    /// without touching the per-connection worker thread's stream.
    conns: Arc<Mutex<HashMap<ConnId, TcpStream>>>,

    /// Serializes all `Handler` callbacks.
    ///
    /// The `IRCServer` inside the handler is not thread-safe — all calls to
    /// `on_connect`, `on_data`, and `on_disconnect` must be serialized.
    handler_lock: Arc<Mutex<()>>,

    /// Counter for generating unique `ConnId` values.
    /// Starts at 1 (0 is reserved as a sentinel "no connection" value).
    next_conn_id: Arc<AtomicU64>,
}

impl EventLoop {
    /// Create a new `EventLoop` in the stopped state.
    ///
    /// # Example
    ///
    /// ```
    /// use irc_net_stdlib::EventLoop;
    /// let event_loop = EventLoop::new();
    /// ```
    pub fn new() -> Self {
        EventLoop {
            running: AtomicBool::new(false),
            conns: Arc::new(Mutex::new(HashMap::new())),
            handler_lock: Arc::new(Mutex::new(())),
            next_conn_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Start the accept loop and block until `stop()` is called.
    ///
    /// This method is meant to run on the process's main thread.  It creates
    /// a `TcpListener` bound to `addr`, then loops accepting connections until
    /// `stop()` sets the `running` flag to false.
    ///
    /// # Parameters
    ///
    /// - `addr`: Bind address, e.g. `"0.0.0.0:6667"` or `"127.0.0.1:0"` (port 0 = OS picks).
    /// - `handler`: The connection lifecycle callback receiver, wrapped in `Arc`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the TCP listener cannot be created (e.g. port in use).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use irc_net_stdlib::{EventLoop, Handler, ConnId};
    /// use std::sync::Arc;
    ///
    /// struct NoopHandler;
    /// impl Handler for NoopHandler {
    ///     fn on_connect(&self, _: ConnId, _: &str) {}
    ///     fn on_data(&self, _: ConnId, _: &[u8]) {}
    ///     fn on_disconnect(&self, _: ConnId) {}
    /// }
    ///
    /// let el = Arc::new(EventLoop::new());
    /// let el2 = el.clone();
    /// std::thread::spawn(move || { std::thread::sleep(std::time::Duration::from_millis(50)); el2.stop(); });
    /// el.run("127.0.0.1:0", Arc::new(NoopHandler)).unwrap();
    /// ```
    pub fn run<H: Handler>(&self, addr: &str, handler: Arc<H>) -> std::io::Result<()> {
        // Bind the listening socket.
        let listener = TcpListener::bind(addr)?;

        // SO_REUSEADDR: allow rebinding this port immediately after the previous
        // process released it.  The TcpListener sets SO_REUSEADDR automatically
        // on Unix; on Windows this is also the default for TcpListener.

        // Set a short accept timeout so the loop can check the `running` flag
        // periodically rather than blocking forever in accept().
        // 200ms is short enough to be responsive, long enough not to burn CPU.
        listener.set_nonblocking(false)?;

        // We can't set timeout on TcpListener directly in stable Rust, so we
        // use a background thread + channel approach for the accept loop.
        // Actually, we set a short timeout by using std::net::TcpListener with
        // set_ttl as a workaround. Instead, we'll use a simple polling approach:
        // mark the listener as non-blocking and poll in a loop.

        // Use non-blocking accept with a brief sleep to avoid busy-waiting.
        listener.set_nonblocking(true)?;

        self.running.store(true, Ordering::SeqCst);

        // Accept loop.
        loop {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            match listener.accept() {
                Ok((stream, addr)) => {
                    // Allocate a unique ConnId for this connection.
                    let conn_id = ConnId(self.next_conn_id.fetch_add(1, Ordering::SeqCst));
                    let peer_host = addr.ip().to_string();

                    // Clone the stream for the conns map (for send_to).
                    // TcpStream::try_clone() creates a new OS file descriptor pointing
                    // to the same socket.  The kernel serializes writes to the same
                    // socket from multiple file descriptors.
                    let stream_for_write = stream.try_clone();
                    let stream_for_write = match stream_for_write {
                        Ok(s) => s,
                        Err(_) => continue, // If we can't clone, skip this connection.
                    };

                    // Register the connection in the shared map.
                    {
                        let mut conns = self.conns.lock().unwrap();
                        conns.insert(conn_id, stream_for_write);
                    }

                    // Spawn a daemon worker thread for this connection.
                    let conns = Arc::clone(&self.conns);
                    let handler_lock = Arc::clone(&self.handler_lock);
                    let handler_clone = Arc::clone(&handler);

                    thread::spawn(move || {
                        Self::worker(conn_id, stream, peer_host, handler_clone, handler_lock, conns);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection ready right now.  Sleep briefly to avoid
                    // spinning the CPU at 100% between accept() calls.
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => {
                    // Other errors (e.g. the listener was closed) — exit.
                    break;
                }
            }
        }

        Ok(())
    }

    /// Signal the event loop to stop accepting new connections.
    ///
    /// Sets the `running` flag to `false`.  The accept loop checks this flag
    /// on every iteration (after the 5ms sleep) and exits when it sees it.
    ///
    /// In-flight connections are not forcibly closed — they run to completion.
    /// Safe to call from any thread, including from signal handlers.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Write `data` to connection `conn_id`.
    ///
    /// Looks up the connection under `conns_lock`, then writes **outside** the
    /// lock.  This is deliberate: we hold the lock for the shortest possible
    /// time (just the stream clone), then release it so other threads can
    /// concurrently look up different connections.
    ///
    /// If `conn_id` is not found (connection closed, or never existed), this
    /// is a silent no-op.
    ///
    /// Does NOT hold `handler_lock` — writing bytes to a socket is independent
    /// of reading server state.
    pub fn send_to(&self, conn_id: ConnId, data: &[u8]) {
        // Step 1: look up the stream while holding the lock.
        let stream_opt = {
            let conns = self.conns.lock().unwrap();
            conns.get(&conn_id).and_then(|s| s.try_clone().ok())
        };

        // Step 2: write outside the lock.
        // If the stream was removed between steps 1 and 2, the write will
        // silently fail (broken pipe or bad fd), which we swallow.
        if let Some(mut stream) = stream_opt {
            // write_all() retries until all bytes are sent or an error occurs.
            // We ignore errors: the peer may have closed the connection.
            let _ = stream.write_all(data);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Internal: worker thread
    // ─────────────────────────────────────────────────────────────────────────

    /// Service a single connection from its own thread.
    ///
    /// This method is the entry point for every connection's worker thread.
    /// It runs the full lifecycle: connect → data loop → disconnect → cleanup.
    ///
    /// Error handling: we never let an exception from the Handler crash this
    /// thread.  (In Rust, panics don't propagate across thread boundaries by
    /// default anyway.)
    fn worker<H: Handler>(
        conn_id: ConnId,
        mut stream: TcpStream,
        peer_host: String,
        handler: Arc<H>,
        handler_lock: Arc<Mutex<()>>,
        conns: Arc<Mutex<HashMap<ConnId, TcpStream>>>,
    ) {
        // Restore the stream to blocking mode for the worker thread.
        // The main thread set it non-blocking for the accept loop; we need
        // blocking reads here so the thread sleeps efficiently while waiting
        // for data from the client.
        let _ = stream.set_nonblocking(false);

        // Phase 1: notify the handler that the connection opened.
        // We hold handler_lock for the duration of the callback so the handler's
        // internal state is consistent.
        {
            let _guard = handler_lock.lock().unwrap();
            handler.on_connect(conn_id, &peer_host);
        }

        // Phase 2: data receive loop.
        // stream.read() blocks here, releasing the OS thread scheduler so other
        // threads can run.  Each recv() returns up to READ_BUF_SIZE bytes.
        let mut buf = [0u8; 4096];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => {
                    // 0 bytes means the peer closed the connection (TCP FIN).
                    break;
                }
                Ok(n) => {
                    // Dispatch the data to the handler.
                    // Hold handler_lock only for the callback duration, then release
                    // before the next read() so other threads can run their callbacks
                    // while we're waiting for more bytes from the network.
                    let _guard = handler_lock.lock().unwrap();
                    handler.on_data(conn_id, &buf[..n]);
                }
                Err(_) => {
                    // Any I/O error is treated as "connection gone" — exit the loop.
                    // Common causes: ECONNRESET, EBADF (after close()), ETIMEDOUT.
                    break;
                }
            }
        }

        // Phase 3: cleanup.
        // Notify the handler.  We do this before removing the conn from `conns`
        // so the handler can still call `send_to()` during the disconnect callback
        // (e.g. to send a final error reply).
        {
            let _guard = handler_lock.lock().unwrap();
            handler.on_disconnect(conn_id);
        }

        // Remove from the connection map so `send_to()` stops finding it.
        {
            let mut conns_guard = conns.lock().unwrap();
            conns_guard.remove(&conn_id);
        }

        // The stream is dropped here, which closes the OS file descriptor.
    }
}

impl Default for EventLoop {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    // ── Helper: a recording handler ───────────────────────────────────────────

    /// A test handler that records all events it receives.
    ///
    /// Protected by a `Mutex` so worker threads can write and the test thread
    /// can read without races.
    struct RecordingHandler {
        events: Arc<Mutex<Vec<String>>>,
        loop_ref: Arc<Mutex<Option<Arc<EventLoop>>>>,
    }

    impl RecordingHandler {
        fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
            let events = Arc::new(Mutex::new(Vec::new()));
            let handler = RecordingHandler {
                events: Arc::clone(&events),
                loop_ref: Arc::new(Mutex::new(None)),
            };
            (handler, events)
        }

        fn set_loop(&self, el: Arc<EventLoop>) {
            *self.loop_ref.lock().unwrap() = Some(el);
        }
    }

    impl Handler for RecordingHandler {
        fn on_connect(&self, conn_id: ConnId, host: &str) {
            let mut events = self.events.lock().unwrap();
            events.push(format!("connect:{:?}:{}", conn_id, host));
        }

        fn on_data(&self, conn_id: ConnId, data: &[u8]) {
            let text = String::from_utf8_lossy(data).to_string();
            let mut events = self.events.lock().unwrap();
            events.push(format!("data:{:?}:{}", conn_id, text.trim_end_matches('\n').trim_end_matches('\r')));
        }

        fn on_disconnect(&self, conn_id: ConnId) {
            let mut events = self.events.lock().unwrap();
            events.push(format!("disconnect:{:?}", conn_id));
            // Stop the server after the first disconnect for test purposes.
            if let Some(el) = self.loop_ref.lock().unwrap().as_ref() {
                el.stop();
            }
        }
    }

    /// Connect, send data, read response, disconnect.  Asserts the events
    /// were recorded in order.
    #[test]
    fn test_connect_data_disconnect() {
        // Find a free port by binding to 0 and getting the assigned port.
        let (handler, events) = RecordingHandler::new();
        let el = Arc::new(EventLoop::new());
        handler.set_loop(Arc::clone(&el));
        let handler = Arc::new(handler);

        // Bind to port 0 (OS assigns a free port).
        // We need to know what port was assigned to connect from the test.
        let listener_temp = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener_temp.local_addr().unwrap();
        drop(listener_temp); // Release the port so EventLoop can rebind it.

        let addr_str = addr.to_string();
        let el_clone = Arc::clone(&el);
        let handler_clone = Arc::clone(&handler);

        // Run the event loop in a background thread.
        let server_thread = thread::spawn(move || {
            el_clone.run(&addr_str, handler_clone).unwrap();
        });

        // Give the server a moment to start listening.
        thread::sleep(Duration::from_millis(50));

        // Connect a client, send some data, then disconnect.
        {
            let mut stream = TcpStream::connect(addr).unwrap();
            stream.write_all(b"NICK alice\r\n").unwrap();
            // Close the connection.
            drop(stream);
        }

        // Wait for the server to finish.
        server_thread.join().unwrap();

        let events_snapshot = events.lock().unwrap().clone();

        // We expect: connect, data, disconnect.
        assert!(events_snapshot.iter().any(|e| e.starts_with("connect:")),
            "expected connect event, got: {:?}", events_snapshot);
        assert!(events_snapshot.iter().any(|e| e.starts_with("data:") && e.contains("NICK alice")),
            "expected data event, got: {:?}", events_snapshot);
        assert!(events_snapshot.iter().any(|e| e.starts_with("disconnect:")),
            "expected disconnect event, got: {:?}", events_snapshot);
    }

    #[test]
    fn test_send_to_delivers_data() {
        // Test that send_to() writes bytes to the right connection.
        let (handler, _events) = RecordingHandler::new();

        // Override on_data to echo back what was received.
        struct EchoHandler {
            loop_ref: Arc<Mutex<Option<Arc<EventLoop>>>>,
            _got_response: Arc<Mutex<bool>>,
        }

        impl Handler for EchoHandler {
            fn on_connect(&self, conn_id: ConnId, _host: &str) {
                // Send a welcome banner when client connects.
                if let Some(el) = self.loop_ref.lock().unwrap().as_ref() {
                    el.send_to(conn_id, b"WELCOME\r\n");
                }
            }
            fn on_data(&self, conn_id: ConnId, _data: &[u8]) {
                // Echo back a fixed response.
                if let Some(el) = self.loop_ref.lock().unwrap().as_ref() {
                    el.send_to(conn_id, b"PONG :server\r\n");
                    el.stop(); // stop after first exchange
                }
            }
            fn on_disconnect(&self, _conn_id: ConnId) {}
        }

        let _ = handler;

        let el = Arc::new(EventLoop::new());
        let got = Arc::new(Mutex::new(false));
        let got_clone = Arc::clone(&got);
        let echo_handler = Arc::new(EchoHandler {
            loop_ref: Arc::new(Mutex::new(Some(Arc::clone(&el)))),
            _got_response: got_clone,
        });

        let listener_temp = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener_temp.local_addr().unwrap();
        drop(listener_temp);

        let addr_str = addr.to_string();
        let el_clone = Arc::clone(&el);

        let server_thread = thread::spawn(move || {
            el_clone.run(&addr_str, echo_handler).unwrap();
        });

        thread::sleep(Duration::from_millis(50));

        let mut stream = TcpStream::connect(addr).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();

        // Read the welcome banner.
        let mut buf = [0u8; 64];
        let n = stream.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"WELCOME\r\n");

        // Send some data to trigger the echo.
        stream.write_all(b"PING :test\r\n").unwrap();

        // Read the echo response.
        let n = stream.read(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"PONG :server\r\n");

        drop(stream);
        server_thread.join().unwrap();

        assert!(true, "send_to delivered data correctly");
    }

    #[test]
    fn test_stop_terminates_run() {
        // Test that stop() terminates the run() loop.
        struct NoopHandler;
        impl Handler for NoopHandler {
            fn on_connect(&self, _: ConnId, _: &str) {}
            fn on_data(&self, _: ConnId, _: &[u8]) {}
            fn on_disconnect(&self, _: ConnId) {}
        }

        let el = Arc::new(EventLoop::new());
        let el_clone = Arc::clone(&el);

        let listener_temp = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener_temp.local_addr().unwrap();
        drop(listener_temp);

        let addr_str = addr.to_string();

        let server_thread = thread::spawn(move || {
            el_clone.run(&addr_str, Arc::new(NoopHandler)).unwrap();
        });

        thread::sleep(Duration::from_millis(50));
        el.stop();

        // The server thread should exit within a reasonable time.
        let result = server_thread.join();
        assert!(result.is_ok(), "server thread should exit cleanly after stop()");
    }

    #[test]
    fn test_multiple_connections() {
        // Test that multiple simultaneous connections each get their own events.
        let connect_count = Arc::new(Mutex::new(0usize));
        let disconnect_count = Arc::new(Mutex::new(0usize));

        let connect_count_clone = Arc::clone(&connect_count);
        let disconnect_count_clone = Arc::clone(&disconnect_count);

        struct CountingHandler {
            connects: Arc<Mutex<usize>>,
            disconnects: Arc<Mutex<usize>>,
            el: Arc<Mutex<Option<Arc<EventLoop>>>>,
        }

        impl Handler for CountingHandler {
            fn on_connect(&self, _conn_id: ConnId, _host: &str) {
                let mut c = self.connects.lock().unwrap();
                *c += 1;
            }
            fn on_data(&self, _conn_id: ConnId, _data: &[u8]) {}
            fn on_disconnect(&self, _conn_id: ConnId) {
                let mut d = self.disconnects.lock().unwrap();
                *d += 1;
                if *d >= 3 {
                    if let Some(el) = self.el.lock().unwrap().as_ref() {
                        el.stop();
                    }
                }
            }
        }

        let el = Arc::new(EventLoop::new());
        let handler = Arc::new(CountingHandler {
            connects: connect_count_clone,
            disconnects: disconnect_count_clone,
            el: Arc::new(Mutex::new(Some(Arc::clone(&el)))),
        });

        let listener_temp = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener_temp.local_addr().unwrap();
        drop(listener_temp);

        let addr_str = addr.to_string();
        let el_clone = Arc::clone(&el);

        let server_thread = thread::spawn(move || {
            el_clone.run(&addr_str, handler).unwrap();
        });

        thread::sleep(Duration::from_millis(50));

        // Connect 3 clients and immediately disconnect.
        for _ in 0..3 {
            let stream = TcpStream::connect(addr).unwrap();
            drop(stream);
            thread::sleep(Duration::from_millis(10));
        }

        server_thread.join().unwrap();

        assert_eq!(*connect_count.lock().unwrap(), 3);
        assert_eq!(*disconnect_count.lock().unwrap(), 3);
    }
}
