//! `WebServer`: a thin wrapper around `HttpServer` that accepts a `WebApp`.
//!
//! Language packages typically expose their own server type that owns a
//! `WebServer` internally. `WebServer` itself is useful when writing pure Rust
//! consumers of `web-core`.
//!
//! `WebServer` fires the `on_server_start` hooks immediately after binding and
//! before returning control to the caller. `on_server_stop` hooks fire after
//! `serve()` returns. Both calls are synchronous on the calling thread.

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

use embeddable_http_server::{HttpServer, HttpServerOptions, MailboxHttpServer, ShardedHttpServer};
use tcp_runtime::{PlatformError, ShardedStopHandle};

use crate::app::WebApp;

/// HTTP server wired to a `WebApp` for request dispatch.
pub struct WebServer<P> {
    inner: HttpServer<P>,
    app: Arc<WebApp>,
}

impl<P> WebServer<P>
where
    P: transport_platform::TransportPlatform,
{
    /// Bind a server on the given platform and address.
    ///
    /// The `app`'s `on_server_start` hooks fire before this method returns.
    pub fn bind(
        platform: P,
        address: tcp_runtime::BindAddress,
        options: HttpServerOptions,
        app: Arc<WebApp>,
    ) -> Result<Self, PlatformError> {
        let app_clone = Arc::clone(&app);
        let inner = HttpServer::bind(platform, address, options, move |request| {
            app_clone.handle(request)
        })?;
        let local_addr = inner.local_addr();
        app.fire_server_start(local_addr);
        Ok(Self { inner, app })
    }

    /// The local socket address the server is bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }

    /// A handle that can stop the server from another thread.
    pub fn stop_handle(&self) -> tcp_runtime::StopHandle {
        self.inner.stop_handle()
    }

    /// Run the event loop until stopped.
    ///
    /// Blocks the calling thread. After this returns, the `on_server_stop`
    /// hooks fire.
    pub fn serve(&mut self) -> Result<(), PlatformError> {
        let result = self.inner.serve();
        self.app.fire_server_stop();
        result
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
impl WebServer<transport_platform::bsd::KqueueTransportPlatform> {
    /// Bind a kqueue-backed server (macOS / BSD).
    pub fn bind_kqueue<A: ToSocketAddrs>(
        addr: A,
        options: HttpServerOptions,
        app: Arc<WebApp>,
    ) -> Result<Self, PlatformError> {
        let address = resolve_addr(addr)?;
        let platform = transport_platform::bsd::KqueueTransportPlatform::new()?;
        Self::bind(
            platform,
            tcp_runtime::BindAddress::Ip(address),
            options,
            app,
        )
    }
}

#[cfg(target_os = "linux")]
impl WebServer<transport_platform::linux::EpollTransportPlatform> {
    /// Bind an epoll-backed server (Linux).
    pub fn bind_epoll<A: ToSocketAddrs>(
        addr: A,
        options: HttpServerOptions,
        app: Arc<WebApp>,
    ) -> Result<Self, PlatformError> {
        let address = resolve_addr(addr)?;
        let platform = transport_platform::linux::EpollTransportPlatform::new()?;
        Self::bind(
            platform,
            tcp_runtime::BindAddress::Ip(address),
            options,
            app,
        )
    }
}

#[cfg(target_os = "windows")]
impl WebServer<transport_platform::windows::WindowsTransportPlatform> {
    /// Bind a Windows IOCP-backed server.
    pub fn bind_windows<A: ToSocketAddrs>(
        addr: A,
        options: HttpServerOptions,
        app: Arc<WebApp>,
    ) -> Result<Self, PlatformError> {
        let address = resolve_addr(addr)?;
        let platform = transport_platform::windows::WindowsTransportPlatform::new()?;
        Self::bind(
            platform,
            tcp_runtime::BindAddress::Ip(address),
            options,
            app,
        )
    }
}

/// A **parallel** `WebApp` server (WEB01a-2): the sharded counterpart of
/// [`WebServer`].
///
/// [`WebServer`] drives every connection on one reactor thread, so a slow or
/// CPU-bound handler stalls every other connection. `ShardedWebServer` runs the
/// dispatch (`app.handle`) across `worker_count` reactor shards (a
/// [`ShardedHttpServer`]), so requests on different connections are handled
/// **concurrently**. The same `Arc<WebApp>` is shared across all shards — it is
/// immutable after construction, so no locking is needed; `WebApp::handle` is
/// `&self` and `Send + Sync`.
///
/// This is **opt-in**: the existing [`WebServer`] (single reactor) is unchanged
/// and remains the default. Callers choose parallelism explicitly by binding a
/// `ShardedWebServer` with `worker_count > 1`. Handler semantics are identical
/// (hooks, routing, `halt`, etc. all run inside `app.handle` on the owning
/// shard); only the degree of concurrency changes. HTTP/1.1 response ordering on
/// a single connection is preserved because each connection stays on one shard.
pub struct ShardedWebServer<P> {
    inner: ShardedHttpServer<P>,
    app: Arc<WebApp>,
}

impl<P> ShardedWebServer<P> {
    /// The local socket address the server is bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }

    /// The number of reactor shards (the handler-parallelism degree).
    pub fn worker_count(&self) -> usize {
        self.inner.worker_count()
    }

    /// A handle that can stop every shard from another thread.
    pub fn stop_handle(&self) -> ShardedStopHandle {
        self.inner.stop_handle()
    }
}

impl<P> ShardedWebServer<P>
where
    P: transport_platform::TransportPlatform + Send + 'static,
{
    /// Run all shard reactors until stopped. Blocks the calling thread; after it
    /// returns, the `on_server_stop` hooks fire (once, like [`WebServer::serve`]).
    pub fn serve(&mut self) -> Result<(), PlatformError> {
        let result = self.inner.serve();
        self.app.fire_server_stop();
        result
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
impl ShardedWebServer<transport_platform::bsd::KqueueTransportPlatform> {
    /// Bind a kqueue-backed sharded server (macOS / BSD) with `worker_count`
    /// reactor shards. The `app`'s `on_server_start` hooks fire before returning.
    pub fn bind_kqueue_sharded<A: ToSocketAddrs>(
        addr: A,
        options: HttpServerOptions,
        worker_count: usize,
        app: Arc<WebApp>,
    ) -> Result<Self, PlatformError> {
        let app_clone = Arc::clone(&app);
        let inner = ShardedHttpServer::bind_kqueue_sharded(
            addr,
            options,
            worker_count,
            move |request| app_clone.handle(request),
        )?;
        let local_addr = inner.local_addr();
        app.fire_server_start(local_addr);
        Ok(Self { inner, app })
    }
}

#[cfg(target_os = "linux")]
impl ShardedWebServer<transport_platform::linux::EpollTransportPlatform> {
    /// Bind an epoll-backed sharded server (Linux) with `worker_count` reactor
    /// shards. The `app`'s `on_server_start` hooks fire before returning.
    pub fn bind_epoll_sharded<A: ToSocketAddrs>(
        addr: A,
        options: HttpServerOptions,
        worker_count: usize,
        app: Arc<WebApp>,
    ) -> Result<Self, PlatformError> {
        let app_clone = Arc::clone(&app);
        let inner = ShardedHttpServer::bind_epoll_sharded(
            addr,
            options,
            worker_count,
            move |request| app_clone.handle(request),
        )?;
        let local_addr = inner.local_addr();
        app.fire_server_start(local_addr);
        Ok(Self { inner, app })
    }
}

#[cfg(target_os = "windows")]
impl ShardedWebServer<transport_platform::windows::WindowsTransportPlatform> {
    /// Bind a Windows IOCP-backed sharded server with `worker_count` reactor
    /// shards. The `app`'s `on_server_start` hooks fire before returning.
    ///
    /// # This constructor always fails on Windows
    ///
    /// It is kept for API symmetry with `bind_kqueue_sharded` /
    /// `bind_epoll_sharded`, but it **cannot succeed**: the sharded model gets
    /// its fan-out from `SO_REUSEPORT` (every shard binds the same address and
    /// the kernel spreads accepts across them), and the Windows TCP provider has
    /// no `SO_REUSEPORT`. `transport-platform`'s Windows
    /// `configure_listener_socket` therefore rejects `reuse_port` outright, so
    /// this returns
    /// `Err(PlatformError::Unsupported("SO_REUSEPORT is not supported by the Windows TCP provider"))`
    /// before a listener is ever created. (Windows' `SO_REUSEADDR` is *not* an
    /// equivalent — it permits rebinding, not load-balanced fan-out.)
    ///
    /// Use [`MailboxWebServer`] for parallel serving on Windows: it parallelises
    /// *by request* over a single listener, so it needs no `SO_REUSEPORT` and is
    /// genuinely cross-platform. `sharded_bind_is_unsupported_on_windows` in
    /// `tests/web_core_test.rs` pins this behaviour.
    pub fn bind_windows_sharded<A: ToSocketAddrs>(
        addr: A,
        options: HttpServerOptions,
        worker_count: usize,
        app: Arc<WebApp>,
    ) -> Result<Self, PlatformError> {
        let app_clone = Arc::clone(&app);
        let inner = ShardedHttpServer::bind_windows_sharded(
            addr,
            options,
            worker_count,
            move |request| app_clone.handle(request),
        )?;
        let local_addr = inner.local_addr();
        app.fire_server_start(local_addr);
        Ok(Self { inner, app })
    }
}

/// A **per-request-parallel** `WebApp` server (WEB01b-2): the mailbox counterpart
/// of [`WebServer`].
///
/// Where [`ShardedWebServer`] parallelises *by connection* (N reactors, each
/// running `app.handle` inline), `MailboxWebServer` parallelises *by request*: a
/// single reactor frames each request and submits it to a `worker_count`-thread
/// pool (a [`MailboxHttpServer`]); a worker runs `app.handle` off the reactor
/// thread and the pool's response router writes the reply back. This decouples
/// handler concurrency from the I/O thread, so even requests arriving on the
/// *same* connection (sequential keep-alive) do not serialise behind one another
/// in the dispatcher. The `Arc<WebApp>` is shared across all pool threads
/// unchanged (`WebApp::handle` is `&self` and `Send + Sync`).
///
/// This is **opt-in**, like [`ShardedWebServer`]: the default [`WebServer`] is
/// untouched. The platform (kqueue / epoll / IOCP) is selected internally by the
/// underlying `EmbeddableTcpServer`, so — unlike the per-platform sharded binds —
/// there is a single cross-platform [`bind`](MailboxWebServer::bind) and the
/// error type is `std::io::Result` (the mailbox stack is `io::Error`-based).
///
/// **Scope (WEB01b-2 / -1a):** correct and in order for one-request-and-close and
/// *sequential* keep-alive; gating and reordering a *pipelined* connection is
/// WEB01b-1b. See `code/specs/WEB01b-mailbox-parallelism.md`.
#[derive(Clone)]
pub struct MailboxWebServer {
    inner: MailboxHttpServer,
    app: Arc<WebApp>,
    /// Fires the `on_server_stop` hooks **exactly once**, even though this type is
    /// `Clone` and `serve` takes `&self`: two clones could each call `serve` and
    /// both return, so we gate the stop hooks behind a shared `Once` to keep them
    /// from double-firing (matching `WebServer`/`ShardedWebServer`, whose
    /// `&mut self` serve cannot be called twice).
    stop_hooks_fired: Arc<std::sync::Once>,
}

impl MailboxWebServer {
    /// Bind `host:port` with a `worker_count`-thread handler pool.
    ///
    /// The `app`'s `on_server_start` hooks fire before this method returns.
    pub fn bind(
        host: &str,
        port: u16,
        options: HttpServerOptions,
        worker_count: usize,
        app: Arc<WebApp>,
    ) -> std::io::Result<Self> {
        let app_clone = Arc::clone(&app);
        let inner = MailboxHttpServer::bind(host, port, options, worker_count, move |request| {
            app_clone.handle(request)
        })?;
        let local_addr = inner.local_addr();
        app.fire_server_start(local_addr);
        Ok(Self {
            inner,
            app,
            stop_hooks_fired: Arc::new(std::sync::Once::new()),
        })
    }

    /// The local socket address the server is bound to.
    pub fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }

    /// Whether the server is currently serving.
    pub fn is_running(&self) -> bool {
        self.inner.is_running()
    }

    /// Signal the server to stop (from this or another thread; the type is
    /// `Clone`, so a clone can serve on a worker thread while another stops it).
    pub fn stop(&self) {
        self.inner.stop();
    }

    /// Run the event loop until stopped. Blocks the calling thread; after it
    /// returns, the `on_server_stop` hooks fire (exactly once across all clones —
    /// see `stop_hooks_fired`).
    pub fn serve(&self) -> std::io::Result<()> {
        let result = self.inner.serve();
        let app = &self.app;
        self.stop_hooks_fired.call_once(|| app.fire_server_stop());
        result
    }
}

fn resolve_addr<A: ToSocketAddrs>(addr: A) -> Result<SocketAddr, PlatformError> {
    addr.to_socket_addrs()
        .map_err(PlatformError::from)?
        .next()
        .ok_or_else(|| PlatformError::Io("no socket addresses resolved".into()))
}
