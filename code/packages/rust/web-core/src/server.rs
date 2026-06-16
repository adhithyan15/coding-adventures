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

use embeddable_http_server::{HttpServer, HttpServerOptions, ShardedHttpServer};
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

fn resolve_addr<A: ToSocketAddrs>(addr: A) -> Result<SocketAddr, PlatformError> {
    addr.to_socket_addrs()
        .map_err(PlatformError::from)?
        .next()
        .ok_or_else(|| PlatformError::Io("no socket addresses resolved".into()))
}
