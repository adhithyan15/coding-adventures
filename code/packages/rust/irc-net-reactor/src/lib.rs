//! # irc-net-reactor — an all-Rust IRC server on the home-grown reactor
//!
//! This crate hosts the pure [`irc_server::IRCServer`] state machine directly on
//! the repository's home-grown TCP runtime (`tcp-runtime` →
//! `transport-platform` → raw `kqueue`/`epoll`/IOCP).  Every line of logic —
//! transport *and* IRC protocol — lives in Rust.  Higher-level language bindings
//! (`python-bridge`, `ruby-bridge`, `node-bridge`, …) embed this engine and
//! expose only a three-call control surface: create, serve, stop.
//!
//! ## How it differs from `irc-net-stdlib`
//!
//! `irc-net-stdlib` (Level 1) dedicates one blocking OS thread to every
//! connection.  `irc-net-reactor` (this crate) uses a single event-loop thread
//! that the kernel wakes only when a socket is readable — the reactor pattern
//! behind nginx, Redis, and Node.js.  The IRC *logic* is byte-for-byte the same
//! (`irc-server`, `irc-proto`, `irc-framing` are reused unchanged); only the
//! transport changes.
//!
//! ## The broadcast problem and how the mailbox solves it
//!
//! IRC is a *broadcast* protocol.  When Alice sends `PRIVMSG #chan :hi`, the
//! bytes must be written to Bob's and Carol's sockets — connections *other than*
//! the one that produced the data.  A naive request/response handler (like
//! `mini-redis`) can only write back to the connection it is currently serving.
//!
//! The reactor solves this with the [`tcp_runtime::TcpMailbox`].  `mailbox.send(
//! connection_id, bytes)` enqueues a write to **any** connection by id, from any
//! thread.  `IRCServer` already returns a `Vec<Response>` where each
//! [`irc_server::Response`] names its own target `conn_id`; we simply serialize
//! each message and hand it to the mailbox.  Replies to the sender and fan-out
//! to other members travel the exact same path.
//!
//! ## Wiring diagram
//!
//! ```text
//!   TCP bytes
//!     ↓  (kqueue/epoll readiness)
//!   tcp-runtime read callback ──→ per-connection Framer.feed()
//!     ↓  Framer.frames()  → b"PRIVMSG #chan :hi"
//!   irc_proto::parse() → Message
//!     ↓
//!   IRCServer::on_message(conn_id, &msg) → Vec<Response>   (under a Mutex)
//!     ↓  for each Response: irc_proto::serialize(&msg)
//!   TcpMailbox.send(target_conn_id, wire_bytes)   ← fan-out to OTHER sockets
//!     ↓
//!   TCP bytes on the wire
//! ```
//!
//! ## Concurrency
//!
//! `irc-server` documents itself as *not* thread-safe, so a single
//! `Arc<Mutex<IRCServer>>` serializes every state mutation.  With the default
//! single reactor thread the mutex is essentially uncontended; the design also
//! stays correct under a future sharded runtime because mailbox sends are
//! themselves thread-safe.
//!
//! ## Resilience to a hostile client (panic isolation)
//!
//! The reactor runs the connection callbacks inline on its single event-loop
//! thread with no `catch_unwind` of its own, so an unhandled panic inside a
//! callback would tear down the **entire** server — every connected client, not
//! just the offender.  To keep one malicious or malformed message from becoming
//! a whole-server denial of service we apply two safeguards here:
//!
//! 1. **Panic containment** — each callback is wrapped in `catch_unwind`; a
//!    panic that escapes `IRCServer` is caught and turned into "close just this
//!    connection," leaving every other client untouched.
//! 2. **Poison tolerance** — a panic while the `IRCServer` mutex is held poisons
//!    it; we recover the guard with `into_inner()` instead of re-panicking, so a
//!    single contained failure does not permanently brick the shared state for
//!    everyone else.

use std::io;
use std::net::SocketAddr;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use irc_framing::Framer;
use irc_proto::{parse, serialize, ParseError};
use irc_server::{ConnId, IRCServer, Response};
use tcp_runtime::{
    ConnectionId, PlatformError, ShardedStopHandle, ShardedTcpRuntime, TcpConnectionInfo,
    TcpHandlerResult, TcpMailbox, TcpRuntime, TcpRuntimeOptions,
};

// ──────────────────────────────────────────────────────────────────────────────
// Platform type alias
//
// `tcp-runtime` is generic over the OS event backend.  We pick the right one per
// target so the rest of the crate is platform-agnostic.  The per-connection
// state parameter is `Framer` — each socket gets its own line reassembler.
//
// The runtime is a [`ShardedTcpRuntime`]: it runs *N* independent reactors on
// *N* OS threads (one kqueue/epoll/IOCP instance each), with the kernel
// load-balancing accepted connections across them via `SO_REUSEPORT`.  TCP
// accept, reads, CRLF framing, and parsing all run in parallel across cores; only
// the `IRCServer` state transition itself is serialized (by the shared mutex),
// and that critical section is small relative to the per-message I/O.  A response
// destined for a client on *another* shard is routed there by the shard-aware
// `TcpMailbox` (each `ConnectionId` encodes its owning reactor).
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
type IrcRuntime = ShardedTcpRuntime<transport_platform::bsd::KqueueTransportPlatform, Framer>;

#[cfg(target_os = "linux")]
type IrcRuntime = ShardedTcpRuntime<transport_platform::linux::EpollTransportPlatform, Framer>;

#[cfg(target_os = "windows")]
type IrcRuntime = ShardedTcpRuntime<transport_platform::windows::WindowsTransportPlatform, Framer>;

// ──────────────────────────────────────────────────────────────────────────────
// Configuration
// ──────────────────────────────────────────────────────────────────────────────

/// Runtime configuration for an [`IrcReactorServer`].
///
/// `port = 0` lets the OS assign a free ephemeral port — handy for tests, which
/// then read the real port back via [`IrcReactorServer::local_addr`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrcConfig {
    /// Bind address, e.g. `"127.0.0.1"` (loopback) or `"0.0.0.0"` (all interfaces).
    pub host: String,
    /// TCP port to listen on; `0` requests an OS-assigned ephemeral port.
    pub port: u16,
    /// Server name shown in the `001` welcome and as the prefix of server messages.
    pub server_name: String,
    /// Message of the Day lines (RFC 1459 §4.1 requires at least one).
    pub motd: Vec<String>,
    /// Password for the `OPER` command; an empty string disables `OPER`.
    pub oper_password: String,
    /// Maximum number of simultaneous connections.
    pub max_connections: usize,
}

impl Default for IrcConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 6667,
            server_name: "irc.local".to_string(),
            motd: vec!["Welcome.".to_string()],
            oper_password: String::new(),
            max_connections: TcpRuntimeOptions::default().max_connections,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// The server
// ──────────────────────────────────────────────────────────────────────────────

/// An IRC server running entirely in Rust on the home-grown reactor.
///
/// `bind` constructs the engine and binds the listener eagerly (so the port is
/// known immediately).  `serve` then runs the blocking event loop until `stop`
/// is called.  The handle is cheap to [`Clone`]; clones share the same
/// underlying runtime and stop signal, which lets one clone `serve()` on a
/// background thread while another calls `stop()`.
#[derive(Clone)]
pub struct IrcReactorServer {
    runtime: Arc<Mutex<Option<IrcRuntime>>>,
    local_addr: SocketAddr,
    stop_handle: ShardedStopHandle,
    serving: Arc<AtomicBool>,
    worker_count: usize,
}

/// The default number of reactor shards: one per available CPU (falling back to
/// 1 if the platform can't report it).  This is what [`IrcReactorServer::bind`]
/// uses; call [`IrcReactorServer::bind_with_worker_count`] to choose explicitly.
fn default_worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

impl IrcReactorServer {
    /// Build the engine, bind the listener, and wire the IRC state machine onto
    /// the reactor.  The listener is bound here, so [`local_addr`](Self::local_addr)
    /// is valid as soon as this returns — before [`serve`](Self::serve) is called.
    ///
    /// Uses one reactor shard per CPU.  For a fixed shard count (e.g. in tests, or
    /// to pin to a single thread) use [`bind_with_worker_count`](Self::bind_with_worker_count).
    pub fn bind(config: IrcConfig) -> io::Result<Self> {
        Self::bind_with_worker_count(config, default_worker_count())
    }

    /// Like [`bind`](Self::bind), but with an explicit number of reactor shards.
    ///
    /// `worker_count` is clamped to at least 1.  With `1`, the server runs a
    /// single reactor (no `SO_REUSEPORT`), behaving like the original
    /// single-threaded engine. On Windows the count is currently clamped to 1
    /// because the TCP provider does not support `SO_REUSEPORT`; on supported
    /// Unix platforms, `N > 1` distributes connections across `N` reactor threads.
    pub fn bind_with_worker_count(config: IrcConfig, worker_count: usize) -> io::Result<Self> {
        #[cfg(target_os = "windows")]
        let worker_count = {
            let _ = worker_count;
            1
        };
        #[cfg(not(target_os = "windows"))]
        let worker_count = worker_count.max(1);

        // The IRC brain.  A single shared state machine (behind a Mutex) serves
        // *all* shards, so nick/channel namespaces stay server-global; the reactor
        // threads contend on it only for the brief state transition per message.
        let server = Arc::new(Mutex::new(IRCServer::new(
            &config.server_name,
            config.motd.clone(),
            &config.oper_password,
        )));

        // The mailbox is how callbacks fan out to *other* connections.  It only
        // exists *after* the runtime is built, yet the callbacks (which need it)
        // are moved *into* the build.  A `OnceLock` breaks this cycle: callbacks
        // capture an empty cell now, and we publish the mailbox into it the
        // instant `bind` returns — strictly before `serve` accepts any
        // connection, so the cell is always populated when a callback fires.
        let mailbox_cell: Arc<OnceLock<TcpMailbox>> = Arc::new(OnceLock::new());

        let runtime = build_runtime(
            &config,
            Arc::clone(&server),
            Arc::clone(&mailbox_cell),
            worker_count,
        )
        .map_err(into_io_error)?;

        let local_addr = runtime.local_addr();
        let stop_handle = runtime.stop_handle();
        // Read back the actual shard count the runtime settled on.
        let worker_count = runtime.worker_count();

        // Publish the mailbox.  `set` only fails if already set, which cannot
        // happen here — we own the sole writer.  The mailbox is shard-aware, so a
        // send to any connection is routed to the reactor that owns it.
        let _ = mailbox_cell.set(runtime.mailbox());

        Ok(Self {
            runtime: Arc::new(Mutex::new(Some(runtime))),
            local_addr,
            stop_handle,
            serving: Arc::new(AtomicBool::new(false)),
            worker_count,
        })
    }

    /// The socket address the listener is bound to.  After `bind` with `port = 0`
    /// this reports the OS-assigned port.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Run the event loop.  Blocks the calling thread until [`stop`](Self::stop)
    /// is invoked (from a signal handler, another thread, or a test).
    ///
    /// The runtime is consumed on first `serve`; a second call returns an error
    /// rather than racing two event loops on one listener.
    pub fn serve(&self) -> io::Result<()> {
        let mut runtime = self
            .runtime
            .lock()
            // Recover rather than re-panic if a prior holder panicked: this mutex
            // only guards the runtime handoff and is never touched on the remote
            // data path, but recovering keeps `serve` callable regardless.
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "irc-net-reactor server is already serving or has already served",
                )
            })?;

        self.serving.store(true, Ordering::SeqCst);
        let result = runtime.serve().map_err(into_io_error);
        self.serving.store(false, Ordering::SeqCst);
        result
    }

    /// Signal the event loop to stop; a blocked [`serve`](Self::serve) returns.
    /// Safe to call from any thread, and idempotent.
    pub fn stop(&self) {
        self.stop_handle.stop();
    }

    /// Whether the event loop is currently running.
    pub fn is_running(&self) -> bool {
        self.serving.load(Ordering::SeqCst)
    }

    /// The number of reactor shards (one OS thread each) this server runs.
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Connection callbacks — the bridge between the reactor and IRCServer
//
// These are free functions (rather than inline closures) so the three
// platform-specific `build_runtime` variants can share one implementation.  The
// macro that wires them into the reactor wraps each in `catch_unwind`, so a
// panic inside `IRCServer` closes only the offending connection instead of
// crashing the shared event-loop thread.
// ──────────────────────────────────────────────────────────────────────────────

/// Lock the shared `IRCServer`, recovering the guard if the mutex was poisoned
/// by an earlier (contained) panic.  Re-panicking on a poisoned mutex would turn
/// one bad message into a permanent outage for every client, so we deliberately
/// keep serving with the recovered state instead.
fn lock_server(server: &Mutex<IRCServer>) -> MutexGuard<'_, IRCServer> {
    server
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A new connection opened: tell the IRC state machine, deliver any responses
/// (normally none until the client registers), and hand back a fresh `Framer`
/// to become this connection's per-socket line reassembler.
fn handle_connect(
    server: &Mutex<IRCServer>,
    mailbox: &OnceLock<TcpMailbox>,
    info: TcpConnectionInfo,
) -> Framer {
    // The peer's IP becomes the host part of the client's `nick!user@host` mask.
    let host = info.peer_addr.ip().to_string();
    let responses = lock_server(server).on_connect(ConnId(info.id.0), &host);
    deliver(mailbox, responses);
    Framer::new()
}

/// Raw bytes arrived: reassemble complete lines, parse each, run it through the
/// IRC state machine, and fan out the resulting responses through the mailbox.
fn handle_data(
    server: &Mutex<IRCServer>,
    mailbox: &OnceLock<TcpMailbox>,
    info: TcpConnectionInfo,
    framer: &mut Framer,
    bytes: &[u8],
) -> TcpHandlerResult {
    framer.feed(bytes);

    for raw_line in framer.frames() {
        // IRC is nominally ASCII but UTF-8 is universally accepted.  Lossy
        // decoding substitutes U+FFFD for bad bytes rather than dropping the
        // connection over a single stray byte.
        let line = String::from_utf8_lossy(&raw_line).into_owned();

        let msg = match parse(&line) {
            Ok(msg) => msg,
            // Malformed or empty line — skip silently, as IRC servers
            // traditionally ignore garbage rather than disconnecting.
            Err(ParseError(_)) => continue,
        };

        let responses = lock_server(server).on_message(ConnId(info.id.0), &msg);
        deliver(mailbox, responses);
    }

    // All writes (including replies to this very sender) are delivered through
    // the mailbox above, so the handler itself queues nothing.
    TcpHandlerResult::default()
}

/// A connection closed: let the state machine clean up and broadcast the QUIT to
/// every channel the client was in.
fn handle_close(
    server: &Mutex<IRCServer>,
    mailbox: &OnceLock<TcpMailbox>,
    info: TcpConnectionInfo,
    _framer: Framer,
) {
    let responses = lock_server(server).on_disconnect(ConnId(info.id.0));
    deliver(mailbox, responses);
}

/// Serialize each [`Response`] and enqueue it to its target connection.
///
/// The connection id inside each `Response` may be *any* connection — this is
/// exactly where IRC broadcast happens.  If the mailbox is not yet published
/// (impossible in practice, since connections are only accepted after `bind`
/// publishes it) the responses are simply dropped rather than panicking.
fn deliver(mailbox: &OnceLock<TcpMailbox>, responses: Vec<Response>) {
    let Some(mailbox) = mailbox.get() else {
        return;
    };
    for response in responses {
        let wire = serialize(&response.msg);
        mailbox.send(ConnectionId(response.conn_id.0), wire);
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Platform-specific binding
//
// The only difference between platforms is which `bind_*_with_state` constructor
// we call.  Each wires the same three callbacks.
// ──────────────────────────────────────────────────────────────────────────────

fn runtime_options(config: &IrcConfig) -> TcpRuntimeOptions {
    TcpRuntimeOptions {
        // At least one connection slot, even if a caller passes 0.
        max_connections: config.max_connections.max(1),
        ..TcpRuntimeOptions::default()
    }
}

/// Build the three reactor callbacks, each carrying its own clones of the shared
/// `IRCServer` and mailbox cell, and bind the sharded runtime.  The same callbacks
/// run on every shard (they capture `Arc`s, so all shards share one brain and one
/// mailbox cell), so no IRC logic is duplicated per reactor.
macro_rules! bind_with {
    ($bind:path, $config:expr, $worker_count:expr, $server:expr, $mailbox:expr) => {{
        let host = $config.host.clone();

        let connect_server = Arc::clone(&$server);
        let connect_mailbox = Arc::clone(&$mailbox);
        let data_server = Arc::clone(&$server);
        let data_mailbox = Arc::clone(&$mailbox);
        let close_server = Arc::clone(&$server);
        let close_mailbox = Arc::clone(&$mailbox);

        // Each callback is wrapped in `catch_unwind` so that a panic inside
        // `IRCServer` (on some crafted message) is contained to the offending
        // connection rather than unwinding into — and killing — the reactor's
        // single event-loop thread.  `AssertUnwindSafe` is sound here: on a
        // caught panic the shared `IRCServer` mutex may be poisoned, but
        // `lock_server` recovers it, so observing post-panic state is acceptable.
        $bind(
            (host.as_str(), $config.port),
            runtime_options($config),
            $worker_count,
            move |info| {
                catch_unwind(AssertUnwindSafe(|| {
                    handle_connect(&connect_server, &connect_mailbox, info)
                }))
                // A panic during connect setup: hand back a fresh framer and let
                // the connection proceed (it has no IRC state yet).
                .unwrap_or_else(|_| Framer::new())
            },
            move |info, framer, bytes| {
                catch_unwind(AssertUnwindSafe(|| {
                    handle_data(&data_server, &data_mailbox, info, framer, bytes)
                }))
                // A panic while handling this client's data closes *only* this
                // connection; every other client keeps running.
                .unwrap_or_else(|_| TcpHandlerResult::close())
            },
            move |info, framer| {
                let _ = catch_unwind(AssertUnwindSafe(|| {
                    handle_close(&close_server, &close_mailbox, info, framer)
                }));
            },
        )
    }};
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn build_runtime(
    config: &IrcConfig,
    server: Arc<Mutex<IRCServer>>,
    mailbox: Arc<OnceLock<TcpMailbox>>,
    worker_count: usize,
) -> Result<IrcRuntime, PlatformError> {
    bind_with!(
        TcpRuntime::bind_kqueue_sharded_with_state,
        config,
        worker_count,
        server,
        mailbox
    )
}

#[cfg(target_os = "linux")]
fn build_runtime(
    config: &IrcConfig,
    server: Arc<Mutex<IRCServer>>,
    mailbox: Arc<OnceLock<TcpMailbox>>,
    worker_count: usize,
) -> Result<IrcRuntime, PlatformError> {
    bind_with!(
        TcpRuntime::bind_epoll_sharded_with_state,
        config,
        worker_count,
        server,
        mailbox
    )
}

#[cfg(target_os = "windows")]
fn build_runtime(
    config: &IrcConfig,
    server: Arc<Mutex<IRCServer>>,
    mailbox: Arc<OnceLock<TcpMailbox>>,
    worker_count: usize,
) -> Result<IrcRuntime, PlatformError> {
    bind_with!(
        TcpRuntime::bind_windows_sharded_with_state,
        config,
        worker_count,
        server,
        mailbox
    )
}

/// Translate a transport-layer [`PlatformError`] into a standard [`io::Error`]
/// so callers (and the language bindings) only deal with `io::Result`.
fn into_io_error(error: PlatformError) -> io::Error {
    use io::ErrorKind;

    let kind = match error {
        PlatformError::AddressInUse => ErrorKind::AddrInUse,
        PlatformError::AddressNotAvailable => ErrorKind::AddrNotAvailable,
        PlatformError::PermissionDenied => ErrorKind::PermissionDenied,
        PlatformError::ConnectionRefused => ErrorKind::ConnectionRefused,
        PlatformError::ConnectionReset => ErrorKind::ConnectionReset,
        PlatformError::BrokenPipe => ErrorKind::BrokenPipe,
        PlatformError::TimedOut => ErrorKind::TimedOut,
        PlatformError::Interrupted => ErrorKind::Interrupted,
        PlatformError::InvalidResource => ErrorKind::InvalidInput,
        PlatformError::ResourceClosed => ErrorKind::BrokenPipe,
        PlatformError::Unsupported(_) => ErrorKind::Unsupported,
        PlatformError::Io(_) | PlatformError::ProviderFault(_) => ErrorKind::Other,
    };

    io::Error::new(kind, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{ErrorKind, Read, Write};
    use std::net::{Shutdown, TcpStream};
    use std::thread;
    use std::time::{Duration, Instant};

    /// Start a server on an OS-assigned port, serving on a background thread.
    fn start_server() -> (
        IrcReactorServer,
        thread::JoinHandle<io::Result<()>>,
        SocketAddr,
    ) {
        let server = IrcReactorServer::bind(IrcConfig {
            port: 0,
            ..IrcConfig::default()
        })
        .expect("server binds");
        let addr = server.local_addr();
        let background = server.clone();
        let handle = thread::spawn(move || background.serve());
        (server, handle, addr)
    }

    /// Like `start_server`, but with an explicit number of reactor shards so a
    /// test can deterministically exercise the multi-shard path regardless of how
    /// many CPUs the runner has.
    #[cfg(not(target_os = "windows"))]
    fn start_server_with_workers(
        worker_count: usize,
    ) -> (
        IrcReactorServer,
        thread::JoinHandle<io::Result<()>>,
        SocketAddr,
    ) {
        let server = IrcReactorServer::bind_with_worker_count(
            IrcConfig {
                port: 0,
                ..IrcConfig::default()
            },
            worker_count,
        )
        .expect("server binds");
        let addr = server.local_addr();
        let background = server.clone();
        let handle = thread::spawn(move || background.serve());
        (server, handle, addr)
    }

    fn connect(addr: SocketAddr) -> TcpStream {
        let mut last_error = None;
        for _ in 0..40 {
            match TcpStream::connect(addr) {
                Ok(stream) => {
                    stream
                        .set_read_timeout(Some(Duration::from_millis(200)))
                        .expect("set read timeout");
                    return stream;
                }
                Err(err) => {
                    last_error = Some(err);
                    thread::sleep(Duration::from_millis(10));
                }
            }
        }
        panic!("server did not accept connections in time: {last_error:?}");
    }

    /// Read from `stream` until the accumulated text contains `needle` or a
    /// deadline elapses.  Returns everything read so the caller can assert on it.
    fn read_until(stream: &mut TcpStream, needle: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 4096];
        while Instant::now() < deadline {
            match stream.read(&mut chunk) {
                Ok(0) => break, // peer closed
                Ok(n) => {
                    buffer.extend_from_slice(&chunk[..n]);
                    if String::from_utf8_lossy(&buffer).contains(needle) {
                        break;
                    }
                }
                Err(err) if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    continue;
                }
                Err(_) => break,
            }
        }
        String::from_utf8_lossy(&buffer).into_owned()
    }

    /// Register a client (NICK + USER) and wait for the `001` welcome numeric.
    fn register(stream: &mut TcpStream, nick: &str) {
        let line = format!("NICK {nick}\r\nUSER {nick} 0 * :{nick}\r\n");
        stream
            .write_all(line.as_bytes())
            .expect("write registration");
        let welcome = read_until(stream, "001");
        assert!(
            welcome.contains("001"),
            "expected 001 welcome for {nick}, got: {welcome:?}"
        );
    }

    #[test]
    fn registration_yields_welcome() {
        let (server, handle, addr) = start_server();
        let mut alice = connect(addr);
        register(&mut alice, "alice");
        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }

    #[test]
    fn ping_gets_pong() {
        let (server, handle, addr) = start_server();
        let mut alice = connect(addr);
        register(&mut alice, "alice");

        alice.write_all(b"PING :liveness\r\n").expect("write ping");
        let pong = read_until(&mut alice, "PONG");
        assert!(pong.contains("PONG"), "expected PONG, got: {pong:?}");

        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }

    #[test]
    fn privmsg_broadcasts_to_other_channel_member() {
        let (server, handle, addr) = start_server();

        // Two independent clients register and join the same channel.
        let mut alice = connect(addr);
        let mut bob = connect(addr);
        register(&mut alice, "alice");
        register(&mut bob, "bob");

        alice.write_all(b"JOIN #test\r\n").expect("alice joins");
        bob.write_all(b"JOIN #test\r\n").expect("bob joins");
        // Make sure both are in the channel before the broadcast.
        let _ = read_until(&mut alice, "JOIN");
        let _ = read_until(&mut bob, "JOIN");

        // Alice speaks; Bob must receive it (fan-out through the mailbox to a
        // DIFFERENT connection than the sender).
        alice
            .write_all(b"PRIVMSG #test :hello bob\r\n")
            .expect("alice privmsg");
        let received = read_until(&mut bob, "hello bob");
        assert!(
            received.contains("PRIVMSG") && received.contains("hello bob"),
            "bob should have received alice's broadcast, got: {received:?}"
        );

        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn broadcast_works_across_multiple_shards() {
        // Force 4 reactor shards so this exercises cross-shard fan-out even on a
        // single-core CI runner: alice and bob may be accepted by DIFFERENT
        // reactors, yet alice's PRIVMSG must still reach bob.  This is the
        // end-to-end proof that the shard-routed mailbox delivers a response to
        // the reactor that owns the target connection while one shared IRCServer
        // brain keeps the channel membership consistent across shards.
        let (server, handle, addr) = start_server_with_workers(4);
        assert_eq!(server.worker_count(), 4);

        let mut alice = connect(addr);
        let mut bob = connect(addr);
        register(&mut alice, "alice");
        register(&mut bob, "bob");

        alice.write_all(b"JOIN #shards\r\n").expect("alice joins");
        bob.write_all(b"JOIN #shards\r\n").expect("bob joins");
        let _ = read_until(&mut alice, "JOIN");
        let _ = read_until(&mut bob, "JOIN");

        alice
            .write_all(b"PRIVMSG #shards :hello across shards\r\n")
            .expect("alice privmsg");
        let received = read_until(&mut bob, "hello across shards");
        assert!(
            received.contains("PRIVMSG") && received.contains("hello across shards"),
            "bob should receive alice's broadcast across shards, got: {received:?}"
        );

        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }

    #[test]
    fn worker_count_is_configured_and_clamped() {
        let bind = |workers| {
            IrcReactorServer::bind_with_worker_count(
                IrcConfig {
                    port: 0,
                    ..IrcConfig::default()
                },
                workers,
            )
            .expect("server binds")
        };
        // 0 clamps to a single reactor. Supported Unix transports honour an
        // explicit count; Windows remains single-shard until it has accept fan-out.
        assert_eq!(bind(0).worker_count(), 1);
        assert_eq!(bind(1).worker_count(), 1);
        #[cfg(not(target_os = "windows"))]
        assert_eq!(bind(3).worker_count(), 3);
        #[cfg(target_os = "windows")]
        assert_eq!(bind(3).worker_count(), 1);
        // The default constructor picks at least one shard (one per CPU where
        // sharding is supported, one on Windows).
        let default = IrcReactorServer::bind(IrcConfig {
            port: 0,
            ..IrcConfig::default()
        })
        .expect("server binds");
        #[cfg(not(target_os = "windows"))]
        assert!(default.worker_count() >= 1);
        #[cfg(target_os = "windows")]
        assert_eq!(default.worker_count(), 1);
    }

    #[test]
    fn server_survives_a_client_connection_reset() {
        // A client that vanishes with unread data in its socket buffer triggers
        // a TCP RST.  The reactor must close only that connection and keep
        // serving everyone else — a single hostile/crashed client must never
        // take the whole server down.  (Regression: the read path used to
        // propagate ECONNRESET out of serve(), killing the event loop.)
        let (server, handle, addr) = start_server();

        let mut alice = connect(addr);
        let mut bob = connect(addr);
        register(&mut alice, "alice");
        register(&mut bob, "bob");

        // Make the server send Alice data she never reads, then drop her socket
        // with that data still buffered → the OS sends an RST on close.
        alice.write_all(b"JOIN #test\r\n").expect("alice joins");
        drop(alice);

        // Bob must still be served: the event loop survived Alice's reset.
        bob.write_all(b"PING :still-alive\r\n").expect("bob pings");
        let pong = read_until(&mut bob, "PONG");
        assert!(
            pong.contains("PONG"),
            "server should keep serving bob after alice's RST, got: {pong:?}"
        );

        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }

    #[test]
    fn stop_called_before_serve_starts_is_not_lost() {
        // A caller may request stop() before the background serve thread has
        // actually entered the event loop (common via FFI bindings that flip a
        // "running" flag before spawning the serve thread).  The stop must not be
        // swallowed — otherwise serve() runs forever and join() hangs.  We loop
        // to exercise the race repeatedly; a regression would hang this test.
        for _ in 0..25 {
            let server = IrcReactorServer::bind(IrcConfig {
                port: 0,
                ..IrcConfig::default()
            })
            .expect("bind");
            let background = server.clone();
            let handle = thread::spawn(move || background.serve());
            // Stop immediately, without waiting for the loop to start.
            server.stop();
            handle.join().expect("server thread").expect("server exit");
        }
    }

    #[test]
    fn quit_command_broadcasts_to_channel() {
        let (server, handle, addr) = start_server();

        let mut alice = connect(addr);
        let mut bob = connect(addr);
        register(&mut alice, "alice");
        register(&mut bob, "bob");
        alice.write_all(b"JOIN #test\r\n").expect("alice joins");
        bob.write_all(b"JOIN #test\r\n").expect("bob joins");
        let _ = read_until(&mut bob, "JOIN");

        // Alice sends a graceful IRC QUIT; the server broadcasts it to channel peers.
        alice
            .write_all(b"QUIT :leaving now\r\n")
            .expect("alice quits");
        let quit = read_until(&mut bob, "QUIT");
        assert!(
            quit.contains("QUIT") && quit.contains("leaving now"),
            "bob should see alice's QUIT broadcast, got: {quit:?}"
        );

        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }

    #[test]
    fn abrupt_disconnect_broadcasts_quit() {
        let (server, handle, addr) = start_server();

        let mut alice = connect(addr);
        let mut bob = connect(addr);
        register(&mut alice, "alice");
        register(&mut bob, "bob");
        alice.write_all(b"JOIN #test\r\n").expect("alice joins");
        bob.write_all(b"JOIN #test\r\n").expect("bob joins");
        let _ = read_until(&mut bob, "JOIN");

        // Simulate a client vanishing without an IRC QUIT.  `shutdown(Write)`
        // sends a clean FIN (unlike `drop` with unread data, which can RST), so
        // the reactor observes a graceful half-close and runs its close callback
        // → `IRCServer::on_disconnect` → QUIT broadcast to remaining members.
        alice
            .shutdown(Shutdown::Write)
            .expect("half-close alice's write side");
        let quit = read_until(&mut bob, "QUIT");
        assert!(
            quit.contains("QUIT"),
            "bob should see alice's QUIT after an abrupt disconnect, got: {quit:?}"
        );

        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }

    #[test]
    fn lock_server_recovers_from_a_poisoned_mutex() {
        // A panic while the IRCServer lock is held poisons the mutex.  The
        // resilience contract is that the next lock recovers the state and keeps
        // serving, rather than re-panicking and bricking the server for everyone.
        let server = Mutex::new(IRCServer::new("irc.local", vec!["hi".to_string()], ""));

        // Poison the mutex deliberately, swallowing the panic (and its noisy
        // default hook) so the test output stays clean.
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = server.lock().unwrap();
            panic!("boom while holding the IRCServer lock");
        }));
        std::panic::set_hook(prev_hook);
        assert!(server.is_poisoned(), "mutex should be poisoned");

        // lock_server hands back a usable guard, and the recovered server still works.
        let responses = lock_server(&server).on_connect(ConnId(1), "127.0.0.1");
        assert!(
            responses.is_empty(),
            "on_connect returns no immediate responses"
        );
    }

    #[test]
    fn config_defaults_are_sane() {
        let config = IrcConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 6667);
        assert_eq!(config.server_name, "irc.local");
        assert!(!config.motd.is_empty());
        assert!(config.oper_password.is_empty());
        assert!(config.max_connections >= 1);
    }

    #[test]
    fn second_serve_is_rejected() {
        let (server, handle, _addr) = start_server();

        // Wait until the background thread has actually entered serve() and taken
        // the runtime.  Without this we'd race: whichever serve() locks the mutex
        // first takes the runtime and blocks, and the loser errors — so the test
        // could otherwise block the main thread forever instead of erroring.
        let deadline = Instant::now() + Duration::from_secs(5);
        while !server.is_running() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(server.is_running(), "background serve never started");

        // The background thread now owns the runtime; a second serve on this
        // handle must error rather than racing two event loops.
        let err = server.serve().expect_err("second serve must fail");
        assert_eq!(err.kind(), ErrorKind::AlreadyExists);

        server.stop();
        handle.join().expect("server thread").expect("server exit");
    }
}
