//! # tcp-reactor
//!
//! Small nonblocking TCP reactor: one listener, many connections, a
//! single-threaded event loop and a pluggable byte handler.
//!
//! The reactor is intentionally transport-focused. It accepts connections,
//! forwards the bytes it reads to a handler, buffers the handler's reply, and
//! flushes it as the network allows.
//!
//! ## Two engines, one behaviour
//!
//! Operating systems disagree about what "asynchronous I/O" means, so there are
//! two engines behind one public API:
//!
//! ```text
//!   Linux / macOS / BSD                      Windows
//!   ───────────────────                      ───────
//!   readiness: "socket is ready,             completion: "the read you started
//!   go and read it"                          has finished"
//!   native-event-core -> epoll / kqueue      iocp crate + Winsock overlapped I/O
//!   (src/readiness.rs)                       AcceptEx / WSARecv / WSASend
//!                                            (src/completion.rs)
//! ```
//!
//! Both honour the same rules: a cap on live connections, a cap on the bytes
//! queued for a slow reader (exceeding it closes the connection), half-close
//! handling (flush, then close), and cooperative shutdown through
//! [`StopHandle`]. Portable code uses [`NativeTcpReactor`] and
//! [`bind_native`]; the platform-specific constructors stay available too.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub type ConnectionId = u64;
pub type Handler = Arc<dyn Fn(ConnectionInfo, &[u8]) -> Vec<u8> + Send + Sync + 'static>;

/// Connection cap used until [`TcpReactor::set_max_connections`] is called.
const DEFAULT_MAX_CONNECTIONS: usize = 1_024;
/// Per-connection queued-output cap used until
/// [`TcpReactor::set_max_pending_write_bytes`] is called.
const DEFAULT_MAX_PENDING_WRITE_BYTES: usize = 64 * 1024;
/// How long one turn of the event loop waits before it re-checks the stop flag.
const POLL_INTERVAL_MS: u64 = 10;

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

#[cfg(unix)]
const LISTENER_TOKEN: native_event_core::Token = native_event_core::Token(1);

#[cfg(unix)]
mod readiness;
#[cfg(unix)]
pub use readiness::TcpReactor;

#[cfg(windows)]
mod completion;
#[cfg(windows)]
pub use completion::TcpReactor;

#[cfg(not(any(unix, windows)))]
compile_error!("tcp-reactor supports Unix-style file descriptors and Windows sockets only");

/// The reactor type backed by this platform's native engine.
#[cfg(target_os = "linux")]
pub type NativeTcpReactor = TcpReactor<native_event_core::linux::LinuxBackend>;
/// The reactor type backed by this platform's native engine.
#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
pub type NativeTcpReactor = TcpReactor<native_event_core::bsd::KqueueBackend>;
/// The reactor type backed by this platform's native engine.
#[cfg(windows)]
pub type NativeTcpReactor = TcpReactor;

/// Bind `addr` and return a reactor that uses this platform's native engine:
/// epoll on Linux, kqueue on macOS/BSD, an I/O completion port on Windows.
#[cfg(target_os = "linux")]
pub fn bind_native<A, F>(addr: A, handler: F) -> std::io::Result<NativeTcpReactor>
where
    A: std::net::ToSocketAddrs,
    F: Fn(ConnectionInfo, &[u8]) -> Vec<u8> + Send + Sync + 'static,
{
    TcpReactor::bind_epoll(addr, handler)
}

/// Bind `addr` and return a reactor that uses this platform's native engine:
/// epoll on Linux, kqueue on macOS/BSD, an I/O completion port on Windows.
#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
pub fn bind_native<A, F>(addr: A, handler: F) -> std::io::Result<NativeTcpReactor>
where
    A: std::net::ToSocketAddrs,
    F: Fn(ConnectionInfo, &[u8]) -> Vec<u8> + Send + Sync + 'static,
{
    TcpReactor::bind_kqueue(addr, handler)
}

/// Bind `addr` and return a reactor that uses this platform's native engine:
/// epoll on Linux, kqueue on macOS/BSD, an I/O completion port on Windows.
#[cfg(windows)]
pub fn bind_native<A, F>(addr: A, handler: F) -> std::io::Result<NativeTcpReactor>
where
    A: std::net::ToSocketAddrs,
    F: Fn(ConnectionInfo, &[u8]) -> Vec<u8> + Send + Sync + 'static,
{
    TcpReactor::bind_iocp(addr, handler)
}
