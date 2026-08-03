//! # tcp-runtime
//!
//! TCP-specific runtime layer above `stream-reactor`.
//!
//! `stream-reactor` owns generic byte-stream progression. This crate adds the
//! TCP-facing surface that application servers want to depend on: listener
//! options, stream options, concrete connection metadata, and host-OS
//! convenience constructors.

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::Duration;

pub use stream_reactor::{ConnectionId, StopHandle};
use stream_reactor::{
    StreamConnectionInfo, StreamHandlerResult, StreamMailbox, StreamReactor, StreamReactorOptions,
};
use transport_platform::TransportPlatform;
pub use transport_platform::{BindAddress, ListenerOptions, PlatformError, StreamOptions};

/// `TcpConnectionInfo` is the TCP-flavored metadata that handlers see for each
/// read chunk. It includes both sides of the socket so protocols can log,
/// route, or enforce policy without consulting lower layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TcpConnectionInfo {
    pub id: ConnectionId,
    pub peer_addr: SocketAddr,
    pub local_addr: SocketAddr,
}

/// `TcpHandlerResult` keeps the phase-one handler contract intentionally small:
/// queue these bytes, and optionally close once the queued output drains.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TcpHandlerResult {
    pub write: Vec<u8>,
    pub close: bool,
    pub defer_read: bool,
}

impl TcpHandlerResult {
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

/// `TcpRuntimeOptions` is the TCP policy surface that callers configure.
/// Listener options feed binding defaults, stream options shape accepted socket
/// policy, and the remaining knobs forward into the generic stream runtime.
#[derive(Debug, Clone)]
pub struct TcpRuntimeOptions {
    pub listener: ListenerOptions,
    pub stream: StreamOptions,
    /// Shard index / shard-bit width baked into each `ConnectionId` so a
    /// `TcpMailbox` can route a write to the owning reactor.  Set automatically per
    /// worker by the sharded builders; both default to `0` for a single runtime
    /// (which leaves `ConnectionId`s identical to the unsharded scheme).
    pub shard_index: u64,
    pub shard_bits: u32,
    /// Whether the runtime accepts on its own listener (`true`, the default).  Set
    /// `false` automatically for fan-out worker reactors, which serve only adopted
    /// connections.
    pub accept_connections: bool,
    pub read_buffer_size: usize,
    pub max_connections: usize,
    pub max_pending_write_bytes: usize,
    pub poll_timeout: Duration,
}

impl Default for TcpRuntimeOptions {
    fn default() -> Self {
        let defaults = StreamReactorOptions::default();
        Self {
            listener: defaults.listener,
            stream: defaults.stream,
            shard_index: 0,
            shard_bits: 0,
            accept_connections: true,
            read_buffer_size: defaults.read_buffer_size,
            max_connections: defaults.max_connections,
            max_pending_write_bytes: defaults.max_pending_write_bytes,
            poll_timeout: defaults.poll_timeout,
        }
    }
}

impl PartialEq for TcpRuntimeOptions {
    fn eq(&self, other: &Self) -> bool {
        self.listener == other.listener
            && self.stream == other.stream
            && self.shard_index == other.shard_index
            && self.shard_bits == other.shard_bits
            && self.accept_connections == other.accept_connections
            && self.read_buffer_size == other.read_buffer_size
            && self.max_connections == other.max_connections
            && self.max_pending_write_bytes == other.max_pending_write_bytes
            && self.poll_timeout == other.poll_timeout
    }
}

impl Eq for TcpRuntimeOptions {}

impl From<TcpRuntimeOptions> for StreamReactorOptions {
    fn from(value: TcpRuntimeOptions) -> Self {
        Self {
            listener: value.listener,
            stream: value.stream,
            shard_index: value.shard_index,
            shard_bits: value.shard_bits,
            accept_connections: value.accept_connections,
            read_buffer_size: value.read_buffer_size,
            max_connections: value.max_connections,
            max_pending_write_bytes: value.max_pending_write_bytes,
            poll_timeout: value.poll_timeout,
        }
    }
}

pub struct TcpRuntime<P, S = ()> {
    reactor: StreamReactor<P, S>,
    local_addr: SocketAddr,
}

pub struct ShardedTcpRuntime<P, S = ()> {
    local_addr: SocketAddr,
    worker_count: usize,
    mailbox: TcpMailbox,
    stop_handles: Vec<StopHandle>,
    runtimes: Vec<TcpRuntime<P, S>>,
    /// The fan-out acceptor (macOS/BSD only).  `None` on platforms where
    /// `SO_REUSEPORT` already load-balances accepts across the per-shard listeners
    /// (Linux); there the workers accept on their own listeners and no separate
    /// acceptor is needed.  `take`n and run by [`serve`](Self::serve).
    acceptor: Option<FanoutAcceptor>,
    /// A clone of the acceptor's stop flag, kept so [`stop`](Self::stop) and the
    /// [`ShardedStopHandle`] can signal it after `serve` has moved the acceptor
    /// onto its thread.
    acceptor_stop: Option<Arc<AtomicBool>>,
}

#[derive(Clone)]
pub struct ShardedStopHandle {
    stop_handles: Arc<[StopHandle]>,
    acceptor_stop: Option<Arc<AtomicBool>>,
}

impl ShardedStopHandle {
    pub fn stop(&self) {
        for stop in self.stop_handles.iter() {
            stop.stop();
        }
        if let Some(acceptor_stop) = &self.acceptor_stop {
            acceptor_stop.store(true, Ordering::SeqCst);
        }
    }
}

impl<P, S> ShardedTcpRuntime<P, S> {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    pub fn mailbox(&self) -> TcpMailbox {
        self.mailbox.clone()
    }

    pub fn stop_handle(&self) -> ShardedStopHandle {
        ShardedStopHandle {
            stop_handles: Arc::from(self.stop_handles.clone().into_boxed_slice()),
            acceptor_stop: self.acceptor_stop.clone(),
        }
    }

    pub fn stop(&self) {
        for stop in &self.stop_handles {
            stop.stop();
        }
        if let Some(acceptor_stop) = &self.acceptor_stop {
            acceptor_stop.store(true, Ordering::SeqCst);
        }
    }
}

impl<P, S> ShardedTcpRuntime<P, S>
where
    P: TransportPlatform + Send + 'static,
    S: Send + 'static,
{
    pub fn serve(&mut self) -> Result<(), PlatformError> {
        // Start the fan-out acceptor (if any) first, so it is ready to distribute
        // connections the instant the workers begin serving.  Each worker reactor
        // serves on its own thread; the acceptor (macOS/BSD) runs on one more.
        let acceptor = self.acceptor.take().and_then(FanoutAcceptor::spawn);

        let runtimes = std::mem::take(&mut self.runtimes);
        let mut workers = Vec::with_capacity(runtimes.len());
        for mut runtime in runtimes {
            workers.push(thread::spawn(move || runtime.serve()));
        }

        let mut first_error = None;
        for worker in workers {
            let result = match worker.join() {
                Ok(result) => result,
                Err(_) => Err(PlatformError::ProviderFault(
                    "TCP runtime worker panicked".to_string(),
                )),
            };
            if first_error.is_none() {
                first_error = result.err();
            }
        }

        // The workers have exited (via `stop`); the acceptor's stop flag was set
        // alongside theirs, so it has exited its accept loop too — join it.
        if let Some(handle) = acceptor {
            let _ = handle.join();
        }

        if let Some(error) = first_error {
            Err(error)
        } else {
            Ok(())
        }
    }
}

/// The macOS/BSD accept fan-out: a single listener whose accepted connections are
/// round-robined to the worker reactors.  Plain `SO_REUSEPORT` does not
/// load-balance on Darwin/BSD (it delivers every connection to one socket), so
/// instead one acceptor owns the listener and hands each accepted socket to a
/// worker via [`StreamMailbox::adopt_connection`].  On Linux the kernel already
/// balances the reuseport group, so this is unused there.
#[cfg_attr(not(unix), allow(dead_code))]
struct FanoutAcceptor {
    /// The single listener clients connect to (set non-blocking so the loop can
    /// poll the stop flag).
    listener: std::net::TcpListener,
    /// One mailbox per worker reactor; accepted sockets are dealt out round-robin.
    mailboxes: Vec<StreamMailbox>,
    /// Set by `stop`/`ShardedStopHandle` to break the accept loop.
    stop: Arc<AtomicBool>,
}

impl FanoutAcceptor {
    /// Spawn the accept-and-distribute loop on its own thread.  Returns `None` on
    /// non-Unix targets (where the fan-out is never constructed).
    #[cfg(unix)]
    fn spawn(self) -> Option<thread::JoinHandle<()>> {
        Some(thread::spawn(move || self.run()))
    }

    #[cfg(not(unix))]
    fn spawn(self) -> Option<thread::JoinHandle<()>> {
        None
    }

    #[cfg(unix)]
    fn run(self) {
        use std::os::fd::OwnedFd;

        let worker_count = self.mailboxes.len().max(1);
        let mut next = 0usize;
        while !self.stop.load(Ordering::SeqCst) {
            match self.listener.accept() {
                Ok((stream, peer_addr)) => {
                    // Transfer the accepted socket to the next worker (round-robin).
                    // `adopt_connection` wakes that reactor, which adopts the fd and
                    // serves it.  `AdoptableFd` is `OwnedFd` on Unix.
                    let fd = OwnedFd::from(stream);
                    self.mailboxes[next % worker_count].adopt_connection(fd, peer_addr);
                    next = next.wrapping_add(1);
                }
                // No pending connection: nap briefly, then re-check the stop flag.
                // (The listener is non-blocking so `stop` can't be stranded behind
                // a blocked `accept`.)
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(std::time::Duration::from_millis(1));
                }
                // Listener error (e.g. closed on shutdown): stop accepting.
                Err(_) => break,
            }
        }
    }
}

impl<P, S> Drop for ShardedTcpRuntime<P, S> {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Clone)]
pub struct TcpMailbox {
    inners: Arc<[StreamMailbox]>,
    /// Number of low `ConnectionId` bits that name the owning shard — exactly the
    /// `shard_bits` the reactors were built with (`ceil(log2(worker_count))`).
    shard_bits: u32,
}

impl TcpMailbox {
    /// Map a `ConnectionId` to the index of the reactor that owns it.
    ///
    /// The shard index lives in the low `shard_bits` of the id (stamped there at
    /// accept time), so this is a single mask — no shared lookup table.  A single
    /// reactor has `shard_bits == 0`, so the mask is `0` and every id routes to
    /// `inners[0]`.  An out-of-range index (only possible from a stale/foreign id
    /// that never belonged to this runtime) yields `None` and the command is
    /// dropped, which is the same safe outcome as routing it to a reactor that
    /// doesn't know the connection.
    fn owner_of(&self, connection_id: ConnectionId) -> Option<&StreamMailbox> {
        let index = (connection_id.0 & ((1u64 << self.shard_bits) - 1)) as usize;
        self.inners.get(index)
    }

    pub fn send(&self, connection_id: ConnectionId, bytes: impl Into<Vec<u8>>) {
        if let Some(inner) = self.owner_of(connection_id) {
            inner.send(connection_id, bytes);
        }
    }

    pub fn send_and_close(&self, connection_id: ConnectionId, bytes: impl Into<Vec<u8>>) {
        if let Some(inner) = self.owner_of(connection_id) {
            inner.send_and_close(connection_id, bytes);
        }
    }

    pub fn close(&self, connection_id: ConnectionId) {
        if let Some(inner) = self.owner_of(connection_id) {
            inner.close(connection_id);
        }
    }

    pub fn pause_reads(&self, connection_id: ConnectionId) {
        if let Some(inner) = self.owner_of(connection_id) {
            inner.pause_reads(connection_id);
        }
    }

    pub fn resume_reads(&self, connection_id: ConnectionId) {
        if let Some(inner) = self.owner_of(connection_id) {
            inner.resume_reads(connection_id);
        }
    }

    /// `resume_all_reads` carries no `ConnectionId`, so it genuinely fans out to
    /// every reactor — each resumes the connections it owns.
    pub fn resume_all_reads(&self) {
        for inner in self.inners.iter() {
            inner.resume_all_reads();
        }
    }

    fn from_mailboxes(mailboxes: Vec<StreamMailbox>) -> Self {
        let shard_bits = shard_bits_for(mailboxes.len());
        Self {
            inners: Arc::from(mailboxes.into_boxed_slice()),
            shard_bits,
        }
    }
}

/// Low bits needed to name `worker_count` shards: `ceil(log2(worker_count))`.
///
/// `worker_count <= 1` → `0` (no bits — the single reactor owns everything and
/// `ConnectionId`s stay identical to the unsharded scheme).  `2 → 1`, `3 → 2`,
/// `4 → 2`, `5 → 3`, … (next-power-of-two rounding, so up to `2^bits` shards fit).
fn shard_bits_for(worker_count: usize) -> u32 {
    if worker_count <= 1 {
        0
    } else {
        // Bits required to represent the largest shard index, `worker_count - 1`.
        u64::BITS - (worker_count as u64 - 1).leading_zeros()
    }
}

impl From<StreamMailbox> for TcpMailbox {
    fn from(value: StreamMailbox) -> Self {
        Self::from_mailboxes(vec![value])
    }
}

impl<P: TransportPlatform> TcpRuntime<P, ()> {
    pub fn bind<F>(
        platform: P,
        address: BindAddress,
        options: TcpRuntimeOptions,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        F: Fn(TcpConnectionInfo, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
    {
        TcpRuntime::bind_with_state(
            platform,
            address,
            options,
            |_| (),
            move |info, _, bytes| handler(info, bytes),
            |_, _| {},
        )
    }
}

impl<P: TransportPlatform, S: Send + 'static> TcpRuntime<P, S> {
    pub fn bind_with_state<I, F, C>(
        platform: P,
        address: BindAddress,
        options: TcpRuntimeOptions,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<Self, PlatformError>
    where
        I: Fn(TcpConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(TcpConnectionInfo, &mut S, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
        C: Fn(TcpConnectionInfo, S) + Send + Sync + 'static,
    {
        // The listener address is only known after bind succeeds, so a small
        // one-time cell bridges that value into the per-read handler closure.
        let local_addr_cell = Arc::new(OnceLock::new());
        let handler_local_addr = Arc::clone(&local_addr_cell);
        let init_local_addr = Arc::clone(&local_addr_cell);
        let close_local_addr = Arc::clone(&local_addr_cell);

        let reactor = StreamReactor::bind_with_state(
            platform,
            address,
            options.into(),
            move |info: StreamConnectionInfo| {
                let local_addr = *init_local_addr
                    .get()
                    .expect("tcp-runtime initializes the local listener address before serving");
                init(TcpConnectionInfo {
                    id: info.id,
                    peer_addr: info.peer_addr,
                    local_addr,
                })
            },
            move |info: StreamConnectionInfo, state: &mut S, bytes: &[u8]| {
                let local_addr = *handler_local_addr
                    .get()
                    .expect("tcp-runtime initializes the local listener address before serving");
                let result = handler(
                    TcpConnectionInfo {
                        id: info.id,
                        peer_addr: info.peer_addr,
                        local_addr,
                    },
                    state,
                    bytes,
                );
                StreamHandlerResult {
                    write: result.write,
                    close: result.close,
                    defer_read: result.defer_read,
                }
            },
            move |info: StreamConnectionInfo, state: S| {
                let local_addr = *close_local_addr
                    .get()
                    .expect("tcp-runtime initializes the local listener address before serving");
                on_close(
                    TcpConnectionInfo {
                        id: info.id,
                        peer_addr: info.peer_addr,
                        local_addr,
                    },
                    state,
                );
            },
        )?;

        let local_addr = reactor.local_addr();
        local_addr_cell
            .set(local_addr)
            .expect("local listener address should be set exactly once");

        Ok(Self {
            reactor,
            local_addr,
        })
    }

    pub fn set_max_connections(&mut self, max_connections: usize) {
        self.reactor.set_max_connections(max_connections);
    }

    pub fn set_max_pending_write_bytes(&mut self, max_pending_write_bytes: usize) {
        self.reactor
            .set_max_pending_write_bytes(max_pending_write_bytes);
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn stop_handle(&self) -> StopHandle {
        self.reactor.stop_handle()
    }

    pub fn mailbox(&self) -> TcpMailbox {
        self.reactor.mailbox().into()
    }

    pub fn serve(&mut self) -> Result<(), PlatformError> {
        self.reactor.serve()
    }
}

fn bind_sharded_runtime_with_state<P, S, I, F, C>(
    address: SocketAddr,
    mut options: TcpRuntimeOptions,
    worker_count: usize,
    init: I,
    handler: F,
    on_close: C,
    new_platform: impl Fn() -> Result<P, PlatformError>,
) -> Result<ShardedTcpRuntime<P, S>, PlatformError>
where
    P: TransportPlatform + 'static,
    S: Send + 'static,
    I: Fn(TcpConnectionInfo) -> S + Send + Sync + 'static,
    F: Fn(TcpConnectionInfo, &mut S, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
    C: Fn(TcpConnectionInfo, S) + Send + Sync + 'static,
{
    let worker_count = worker_count.max(1);

    // Each reactor stamps its own shard index into the low `shard_bits` of every
    // ConnectionId it allocates, so the shared mailbox routes a write straight to
    // the owning reactor.  No cross-shard atomic seed is needed for uniqueness any
    // more — the shard bits plus each reactor's private sequence are unique.
    let shard_bits = shard_bits_for(worker_count);

    let init: Arc<dyn Fn(TcpConnectionInfo) -> S + Send + Sync> = Arc::new(init);
    // The handler is an owned callback trait object; its signature is the runtime's
    // public per-connection contract, not a type worth aliasing away.
    #[allow(clippy::type_complexity)]
    let handler: Arc<
        dyn Fn(TcpConnectionInfo, &mut S, &[u8]) -> TcpHandlerResult + Send + Sync,
    > = Arc::new(handler);
    let on_close: Arc<dyn Fn(TcpConnectionInfo, S) + Send + Sync> = Arc::new(on_close);

    // macOS/BSD: plain `SO_REUSEPORT` does not load-balance accepts, so instead of
    // N reuseport listeners we use an explicit acceptor fan-out — one listener
    // distributing accepted sockets to the workers.  (Linux's reuseport does
    // balance, so it keeps the simpler N-listener path below.)
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    {
        if worker_count > 1 {
            return build_fanout_runtime(
                address,
                options,
                worker_count,
                shard_bits,
                init,
                handler,
                on_close,
                &new_platform,
            );
        }
    }

    // Default path: each worker accepts on its own `SO_REUSEPORT` listener (Linux
    // balances them; a single worker needs no reuseport at all).
    if worker_count > 1 {
        options.listener.reuse_port = true;
    }

    let mut bind_address = address;
    let mut runtimes = Vec::with_capacity(worker_count);
    for shard_index in 0..worker_count {
        let mut worker_options = options.clone();
        worker_options.shard_index = shard_index as u64;
        worker_options.shard_bits = shard_bits;

        let init = Arc::clone(&init);
        let handler = Arc::clone(&handler);
        let on_close = Arc::clone(&on_close);
        let runtime = TcpRuntime::bind_with_state(
            new_platform()?,
            BindAddress::Ip(bind_address),
            worker_options,
            move |info| init(info),
            move |info, state, bytes| handler(info, state, bytes),
            move |info, state| on_close(info, state),
        )?;

        bind_address = runtime.local_addr();
        runtimes.push(runtime);
    }

    let local_addr = runtimes
        .first()
        .map(TcpRuntime::local_addr)
        .ok_or(PlatformError::InvalidResource)?;
    let stop_handles = runtimes
        .iter()
        .map(TcpRuntime::stop_handle)
        .collect::<Vec<_>>();
    let mailbox = TcpMailbox::from_mailboxes(
        runtimes
            .iter()
            .map(|runtime| runtime.reactor.mailbox())
            .collect(),
    );
    let worker_count = runtimes.len();

    Ok(ShardedTcpRuntime {
        local_addr,
        worker_count,
        mailbox,
        stop_handles,
        runtimes,
        acceptor: None,
        acceptor_stop: None,
    })
}

/// Build a sharded runtime using an explicit **accept fan-out** (macOS/BSD).
///
/// The worker reactors bind throwaway loopback listeners — they never accept real
/// traffic, they only *serve* connections handed to them by adoption — while a
/// single [`FanoutAcceptor`] owns the client-facing listener and round-robins each
/// accepted socket to a worker via `adopt_connection`.  This is what makes the
/// runtime actually scale across cores on Darwin/BSD, where plain `SO_REUSEPORT`
/// would deliver every connection to one socket.
#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn build_fanout_runtime<P, S>(
    address: SocketAddr,
    mut options: TcpRuntimeOptions,
    worker_count: usize,
    shard_bits: u32,
    init: Arc<dyn Fn(TcpConnectionInfo) -> S + Send + Sync>,
    handler: Arc<dyn Fn(TcpConnectionInfo, &mut S, &[u8]) -> TcpHandlerResult + Send + Sync>,
    on_close: Arc<dyn Fn(TcpConnectionInfo, S) + Send + Sync>,
    new_platform: impl Fn() -> Result<P, PlatformError>,
) -> Result<ShardedTcpRuntime<P, S>, PlatformError>
where
    P: TransportPlatform + 'static,
    S: Send + 'static,
{
    // Workers don't accept client traffic.  They still bind a listener (the
    // constructor requires one), but `accept_connections = false` means they never
    // enable accept interest on it — so a direct connect to a worker's throwaway
    // loopback port is left in the kernel backlog, never served, and can't be used
    // to bypass the acceptor.  reuse_port is irrelevant (each is a separate port).
    options.listener.reuse_port = false;
    options.accept_connections = false;
    let throwaway: SocketAddr = "127.0.0.1:0".parse().expect("valid loopback address");

    let mut runtimes = Vec::with_capacity(worker_count);
    for shard_index in 0..worker_count {
        let mut worker_options = options.clone();
        worker_options.shard_index = shard_index as u64;
        worker_options.shard_bits = shard_bits;

        let init = Arc::clone(&init);
        let handler = Arc::clone(&handler);
        let on_close = Arc::clone(&on_close);
        let runtime = TcpRuntime::bind_with_state(
            new_platform()?,
            BindAddress::Ip(throwaway),
            worker_options,
            move |info| init(info),
            move |info, state, bytes| handler(info, state, bytes),
            move |info, state| on_close(info, state),
        )?;
        runtimes.push(runtime);
    }

    // The acceptor's listener is the one clients actually connect to; bind it on
    // the requested address and report *its* address as the runtime's local_addr.
    // Non-blocking so the accept loop can poll its stop flag.
    let acceptor_listener = std::net::TcpListener::bind(address).map_err(PlatformError::from)?;
    acceptor_listener
        .set_nonblocking(true)
        .map_err(PlatformError::from)?;
    let local_addr = acceptor_listener
        .local_addr()
        .map_err(PlatformError::from)?;

    let worker_mailboxes: Vec<StreamMailbox> = runtimes
        .iter()
        .map(|runtime| runtime.reactor.mailbox())
        .collect();
    let stop_handles = runtimes
        .iter()
        .map(TcpRuntime::stop_handle)
        .collect::<Vec<_>>();
    // The write-routing mailbox is still keyed by the shard bits in each
    // ConnectionId, which adopted connections carry just like accepted ones.
    let mailbox = TcpMailbox::from_mailboxes(worker_mailboxes.clone());
    let acceptor_stop = Arc::new(AtomicBool::new(false));
    let acceptor = FanoutAcceptor {
        listener: acceptor_listener,
        mailboxes: worker_mailboxes,
        stop: Arc::clone(&acceptor_stop),
    };
    let worker_count = runtimes.len();

    Ok(ShardedTcpRuntime {
        local_addr,
        worker_count,
        mailbox,
        stop_handles,
        runtimes,
        acceptor: Some(acceptor),
        acceptor_stop: Some(acceptor_stop),
    })
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
impl TcpRuntime<transport_platform::bsd::KqueueTransportPlatform, ()> {
    pub fn bind_kqueue<A, F>(
        addr: A,
        options: TcpRuntimeOptions,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(TcpConnectionInfo, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
    {
        let address = resolve_first_socket_addr(addr)?;
        let platform = transport_platform::bsd::KqueueTransportPlatform::new()?;
        Self::bind(platform, BindAddress::Ip(address), options, handler)
    }

    pub fn bind_kqueue_sharded<A, F>(
        addr: A,
        options: TcpRuntimeOptions,
        worker_count: usize,
        handler: F,
    ) -> Result<ShardedTcpRuntime<transport_platform::bsd::KqueueTransportPlatform>, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(TcpConnectionInfo, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
    {
        Self::bind_kqueue_sharded_with_state(
            addr,
            options,
            worker_count,
            |_| (),
            move |info, _, bytes| handler(info, bytes),
            |_, _| {},
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
impl<S: Send + 'static> TcpRuntime<transport_platform::bsd::KqueueTransportPlatform, S> {
    pub fn bind_kqueue_with_state<A, I, F, C>(
        addr: A,
        options: TcpRuntimeOptions,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        I: Fn(TcpConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(TcpConnectionInfo, &mut S, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
        C: Fn(TcpConnectionInfo, S) + Send + Sync + 'static,
    {
        let address = resolve_first_socket_addr(addr)?;
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

    pub fn bind_kqueue_sharded_with_state<A, I, F, C>(
        addr: A,
        options: TcpRuntimeOptions,
        worker_count: usize,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<ShardedTcpRuntime<transport_platform::bsd::KqueueTransportPlatform, S>, PlatformError>
    where
        A: ToSocketAddrs,
        I: Fn(TcpConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(TcpConnectionInfo, &mut S, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
        C: Fn(TcpConnectionInfo, S) + Send + Sync + 'static,
    {
        bind_sharded_runtime_with_state(
            resolve_first_socket_addr(addr)?,
            options,
            worker_count,
            init,
            handler,
            on_close,
            transport_platform::bsd::KqueueTransportPlatform::new,
        )
    }
}

#[cfg(target_os = "linux")]
impl TcpRuntime<transport_platform::linux::EpollTransportPlatform, ()> {
    pub fn bind_epoll<A, F>(
        addr: A,
        options: TcpRuntimeOptions,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(TcpConnectionInfo, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
    {
        let address = resolve_first_socket_addr(addr)?;
        let platform = transport_platform::linux::EpollTransportPlatform::new()?;
        Self::bind(platform, BindAddress::Ip(address), options, handler)
    }

    pub fn bind_epoll_sharded<A, F>(
        addr: A,
        options: TcpRuntimeOptions,
        worker_count: usize,
        handler: F,
    ) -> Result<ShardedTcpRuntime<transport_platform::linux::EpollTransportPlatform>, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(TcpConnectionInfo, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
    {
        Self::bind_epoll_sharded_with_state(
            addr,
            options,
            worker_count,
            |_| (),
            move |info, _, bytes| handler(info, bytes),
            |_, _| {},
        )
    }
}

#[cfg(target_os = "linux")]
impl<S: Send + 'static> TcpRuntime<transport_platform::linux::EpollTransportPlatform, S> {
    pub fn bind_epoll_with_state<A, I, F, C>(
        addr: A,
        options: TcpRuntimeOptions,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        I: Fn(TcpConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(TcpConnectionInfo, &mut S, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
        C: Fn(TcpConnectionInfo, S) + Send + Sync + 'static,
    {
        let address = resolve_first_socket_addr(addr)?;
        let platform = transport_platform::linux::EpollTransportPlatform::new()?;
        Self::bind_with_state(
            platform,
            BindAddress::Ip(address),
            options,
            init,
            handler,
            on_close,
        )
    }

    pub fn bind_epoll_sharded_with_state<A, I, F, C>(
        addr: A,
        options: TcpRuntimeOptions,
        worker_count: usize,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<
        ShardedTcpRuntime<transport_platform::linux::EpollTransportPlatform, S>,
        PlatformError,
    >
    where
        A: ToSocketAddrs,
        I: Fn(TcpConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(TcpConnectionInfo, &mut S, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
        C: Fn(TcpConnectionInfo, S) + Send + Sync + 'static,
    {
        bind_sharded_runtime_with_state(
            resolve_first_socket_addr(addr)?,
            options,
            worker_count,
            init,
            handler,
            on_close,
            transport_platform::linux::EpollTransportPlatform::new,
        )
    }
}

#[cfg(target_os = "windows")]
impl TcpRuntime<transport_platform::windows::WindowsTransportPlatform, ()> {
    pub fn bind_windows<A, F>(
        addr: A,
        options: TcpRuntimeOptions,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(TcpConnectionInfo, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
    {
        Self::bind_iocp(addr, options, handler)
    }

    pub fn bind_iocp<A, F>(
        addr: A,
        options: TcpRuntimeOptions,
        handler: F,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        F: Fn(TcpConnectionInfo, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
    {
        let address = resolve_first_socket_addr(addr)?;
        let platform = transport_platform::windows::IocpTransportPlatform::new()?;
        Self::bind(platform, BindAddress::Ip(address), options, handler)
    }

    pub fn bind_windows_sharded<A, F>(
        addr: A,
        options: TcpRuntimeOptions,
        worker_count: usize,
        handler: F,
    ) -> Result<
        ShardedTcpRuntime<transport_platform::windows::WindowsTransportPlatform>,
        PlatformError,
    >
    where
        A: ToSocketAddrs,
        F: Fn(TcpConnectionInfo, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
    {
        Self::bind_windows_sharded_with_state(
            addr,
            options,
            worker_count,
            |_| (),
            move |info, _, bytes| handler(info, bytes),
            |_, _| {},
        )
    }
}

#[cfg(target_os = "windows")]
impl<S: Send + 'static> TcpRuntime<transport_platform::windows::WindowsTransportPlatform, S> {
    pub fn bind_windows_with_state<A, I, F, C>(
        addr: A,
        options: TcpRuntimeOptions,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        I: Fn(TcpConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(TcpConnectionInfo, &mut S, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
        C: Fn(TcpConnectionInfo, S) + Send + Sync + 'static,
    {
        Self::bind_iocp_with_state(addr, options, init, handler, on_close)
    }

    pub fn bind_iocp_with_state<A, I, F, C>(
        addr: A,
        options: TcpRuntimeOptions,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<Self, PlatformError>
    where
        A: ToSocketAddrs,
        I: Fn(TcpConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(TcpConnectionInfo, &mut S, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
        C: Fn(TcpConnectionInfo, S) + Send + Sync + 'static,
    {
        let address = resolve_first_socket_addr(addr)?;
        let platform = transport_platform::windows::IocpTransportPlatform::new()?;
        Self::bind_with_state(
            platform,
            BindAddress::Ip(address),
            options,
            init,
            handler,
            on_close,
        )
    }

    pub fn bind_windows_sharded_with_state<A, I, F, C>(
        addr: A,
        options: TcpRuntimeOptions,
        worker_count: usize,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<
        ShardedTcpRuntime<transport_platform::windows::WindowsTransportPlatform, S>,
        PlatformError,
    >
    where
        A: ToSocketAddrs,
        I: Fn(TcpConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(TcpConnectionInfo, &mut S, &[u8]) -> TcpHandlerResult + Send + Sync + 'static,
        C: Fn(TcpConnectionInfo, S) + Send + Sync + 'static,
    {
        bind_sharded_runtime_with_state(
            resolve_first_socket_addr(addr)?,
            options,
            worker_count,
            init,
            handler,
            on_close,
            transport_platform::windows::WindowsTransportPlatform::new,
        )
    }
}

fn resolve_first_socket_addr<A: ToSocketAddrs>(addr: A) -> Result<SocketAddr, PlatformError> {
    addr.to_socket_addrs()
        .map_err(PlatformError::from)?
        .next()
        .ok_or_else(|| PlatformError::Io("no socket addresses resolved".into()))
}

#[cfg(all(test, target_os = "windows"))]
mod windows_tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpStream};
    use std::thread;

    const DEFAULT_STRESS_CLIENT_COUNT: usize = 10_000;
    const STRESS_PROGRESS_INTERVAL: usize = 1_000;

    #[test]
    fn bind_iocp_serves_echo_clients() {
        let mut runtime = TcpRuntime::bind_iocp(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            |_, bytes| TcpHandlerResult::write(bytes.to_vec()),
        )
        .expect("bind iocp runtime");
        let addr = runtime.local_addr();
        let stop = runtime.stop_handle();

        let server = thread::spawn(move || runtime.serve());

        let mut client = TcpStream::connect(addr).expect("connect client");
        client.write_all(b"iocp-echo").expect("write request");
        client.shutdown(Shutdown::Write).expect("shutdown write");

        let mut response = Vec::new();
        client
            .read_to_end(&mut response)
            .expect("read echoed response");
        assert_eq!(response, b"iocp-echo");

        stop.stop();
        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");
    }

    #[test]
    #[ignore]
    fn bind_iocp_sustains_ten_thousand_concurrent_connections() {
        let client_count = std::env::var("TCP_RUNTIME_STRESS_CLIENTS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|count| *count > 0)
            .unwrap_or(DEFAULT_STRESS_CLIENT_COUNT);
        let backlog = client_count.saturating_add(1_024).min(u32::MAX as usize) as u32;
        let options = TcpRuntimeOptions {
            listener: ListenerOptions {
                backlog,
                ..ListenerOptions::default()
            },
            max_connections: client_count.saturating_add(1_024),
            read_buffer_size: 1024,
            poll_timeout: Duration::from_millis(1),
            ..TcpRuntimeOptions::default()
        };
        let mut runtime = TcpRuntime::bind_iocp(("127.0.0.1", 0), options, |_, bytes| {
            TcpHandlerResult::write(bytes.to_vec())
        })
        .expect("bind iocp runtime");
        let addr = runtime.local_addr();
        let stop = runtime.stop_handle();

        let server = thread::spawn(move || runtime.serve());

        let connect_started = std::time::Instant::now();
        let mut clients = Vec::with_capacity(client_count);
        for index in 0..client_count {
            let stream = connect_client_with_retry(addr);
            stream.set_nodelay(true).expect("set nodelay");
            stream
                .set_read_timeout(Some(Duration::from_secs(20)))
                .expect("set read timeout");
            clients.push(stream);
            if (index + 1) % STRESS_PROGRESS_INTERVAL == 0 || index + 1 == client_count {
                eprintln!(
                    "connected {}/{} clients in {:?}",
                    index + 1,
                    client_count,
                    connect_started.elapsed()
                );
            }
        }

        let round_trip_started = std::time::Instant::now();
        for (index, stream) in clients.iter_mut().enumerate() {
            let payload = (index as u32).to_le_bytes();
            stream.write_all(&payload).expect("write payload");
        }

        let mut echoed = [0u8; 4];
        for (index, stream) in clients.iter_mut().enumerate() {
            let payload = (index as u32).to_le_bytes();
            stream.read_exact(&mut echoed).expect("read echoed payload");
            assert_eq!(echoed, payload, "echo mismatch for client {index}");
            if (index + 1) % STRESS_PROGRESS_INTERVAL == 0 || index + 1 == client_count {
                eprintln!(
                    "echoed {}/{} clients in {:?}",
                    index + 1,
                    client_count,
                    round_trip_started.elapsed()
                );
            }
        }

        drop(clients);
        stop.stop();
        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");
    }

    fn connect_client_with_retry(addr: SocketAddr) -> TcpStream {
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        let mut last_error = None;
        while std::time::Instant::now() < deadline {
            match TcpStream::connect(addr) {
                Ok(stream) => return stream,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionRefused
                            | std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::ConnectionAborted
                            | std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::AddrNotAvailable
                    ) =>
                {
                    last_error = Some(error);
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("connect client: {error}"),
            }
        }
        panic!(
            "connect client before deadline: {:?}",
            last_error.expect("last transient connect error")
        );
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
    use std::io::{self, Read, Write};
    use std::net::{Shutdown, TcpStream};
    use std::sync::{Arc, Barrier, Mutex, OnceLock};
    use std::thread;

    #[test]
    fn serves_many_concurrent_echo_clients_without_crashing() {
        let mut runtime = TcpRuntime::bind_kqueue(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            |_, bytes| TcpHandlerResult::write(bytes.to_vec()),
        )
        .expect("bind");
        let addr = runtime.local_addr();
        let stop = runtime.stop_handle();

        let server = thread::spawn(move || runtime.serve());

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
    fn sharded_runtime_serves_clients_across_multiple_reactors() {
        let seen_ids = Arc::new(Mutex::new(Vec::<ConnectionId>::new()));
        let seen_in_handler = Arc::clone(&seen_ids);
        let mut runtime = TcpRuntime::bind_kqueue_sharded(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            3,
            move |info, bytes| {
                seen_in_handler
                    .lock()
                    .expect("seen ids mutex poisoned")
                    .push(info.id);
                TcpHandlerResult::write(bytes.to_vec())
            },
        )
        .expect("bind sharded runtime");
        let addr = runtime.local_addr();
        assert_eq!(runtime.worker_count(), 3);

        let stop = runtime.stop_handle();
        let mailbox = runtime.mailbox();
        let server = thread::spawn(move || runtime.serve());

        let client_count = 24usize;
        let barrier = Arc::new(Barrier::new(client_count));
        let mut clients = Vec::new();

        for i in 0..client_count {
            let barrier = Arc::clone(&barrier);
            clients.push(thread::spawn(move || -> Result<Vec<u8>, String> {
                barrier.wait();
                let mut stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
                let payload = format!("sharded-{i}").into_bytes();
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
            assert_eq!(echoed, format!("sharded-{i}").into_bytes());
        }

        let ids = seen_ids.lock().expect("seen ids mutex poisoned").clone();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(ids.len(), client_count);
        assert_eq!(unique.len(), client_count);

        mailbox.resume_all_reads();
        stop.stop();
        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");
    }

    #[test]
    fn sharded_runtime_distributes_connections_across_shards() {
        // With several shards and many clients, connections must land on MORE than
        // one shard.  On macOS/BSD plain SO_REUSEPORT fails to provide this (every
        // connection goes to one listener), so the sharded runtime uses the accept
        // fan-out — a single acceptor round-robining accepts across the workers.
        // Each ConnectionId encodes its owning shard in its low bits, so the
        // handler can report which reactor accepted (or adopted) it.  Against the
        // old all-on-one-shard behavior this test would see a single shard and
        // fail.
        let worker_count = 3usize;
        let shard_mask = (1u64 << shard_bits_for(worker_count)) - 1;
        let shards_seen = Arc::new(Mutex::new(std::collections::BTreeSet::<u64>::new()));
        let shards_in_handler = Arc::clone(&shards_seen);
        let mut runtime = TcpRuntime::bind_kqueue_sharded(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            worker_count,
            move |info, bytes| {
                shards_in_handler
                    .lock()
                    .expect("shards mutex poisoned")
                    .insert(info.id.0 & shard_mask);
                TcpHandlerResult::write(bytes.to_vec())
            },
        )
        .expect("bind sharded runtime");
        assert_eq!(runtime.worker_count(), worker_count);
        let addr = runtime.local_addr();
        let stop = runtime.stop_handle();
        let server = thread::spawn(move || runtime.serve());

        // Drive enough clients that distribution must touch more than one shard.
        let client_count = 12usize;
        let mut clients = Vec::new();
        for i in 0..client_count {
            // Retry briefly while the background acceptor/reactors come up.
            let mut stream = None;
            for _ in 0..200 {
                if let Ok(s) = TcpStream::connect(addr) {
                    stream = Some(s);
                    break;
                }
                thread::sleep(Duration::from_millis(5));
            }
            let mut stream = stream.expect("connect to sharded runtime");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("read timeout");
            let payload = (i as u32).to_le_bytes();
            stream.write_all(&payload).expect("write payload");
            let mut echoed = [0u8; 4];
            stream.read_exact(&mut echoed).expect("read echo");
            assert_eq!(echoed, payload, "echo mismatch for client {i}");
            clients.push(stream);
        }

        let distinct = shards_seen.lock().expect("shards mutex poisoned").len();
        assert!(
            distinct >= 2,
            "connections should spread across shards, but all landed on {distinct} shard(s)"
        );

        drop(clients);
        stop.stop();
        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");
    }

    #[test]
    fn handler_receives_the_listener_local_address() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_in_handler = Arc::clone(&seen);
        let mut runtime = TcpRuntime::bind_kqueue(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            move |info, bytes| {
                seen_in_handler
                    .lock()
                    .expect("seen mutex poisoned")
                    .push(info.local_addr);
                TcpHandlerResult::write(bytes.to_vec())
            },
        )
        .expect("bind");
        let addr = runtime.local_addr();
        let stop = runtime.stop_handle();

        let server = thread::spawn(move || runtime.serve());

        let mut client = TcpStream::connect(addr).expect("connect");
        client.write_all(b"local-address").expect("write request");
        client.shutdown(Shutdown::Write).expect("shutdown write");

        let mut echoed = Vec::new();
        client.read_to_end(&mut echoed).expect("read echoed bytes");
        assert_eq!(echoed, b"local-address");

        stop.stop();
        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");

        let seen = seen.lock().expect("seen mutex poisoned");
        assert!(
            !seen.is_empty(),
            "handler should record at least one local address"
        );
        assert!(seen.iter().all(|candidate| *candidate == addr));
    }

    #[test]
    fn stateful_handlers_preserve_session_state_across_reads() {
        let mut runtime = TcpRuntime::bind_kqueue_with_state(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            |_| Vec::<u8>::new(),
            |_, state, bytes| {
                state.extend_from_slice(bytes);
                if state.ends_with(b"\n") {
                    TcpHandlerResult::write(state.clone())
                } else {
                    TcpHandlerResult::default()
                }
            },
            |_, _| {},
        )
        .expect("bind");
        let addr = runtime.local_addr();
        let stop = runtime.stop_handle();

        let server = thread::spawn(move || runtime.serve());

        let mut client = TcpStream::connect(addr).expect("connect");
        client
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("read timeout");

        client.write_all(b"hel").expect("write first fragment");
        let mut probe = [0u8; 16];
        let err = client.read(&mut probe).expect_err("no response yet");
        assert!(
            matches!(
                err.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ),
            "expected no response before newline, got {err}"
        );

        client.write_all(b"lo\n").expect("write second fragment");
        let mut echoed = [0u8; 6];
        client.read_exact(&mut echoed).expect("read response");
        assert_eq!(&echoed, b"hello\n");

        stop.stop();
        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");
    }

    #[test]
    fn mailbox_can_send_delayed_response_after_handler_returns() {
        let seen = Arc::new(Mutex::new(Vec::<ConnectionId>::new()));
        let seen_in_handler = Arc::clone(&seen);
        let mut runtime = TcpRuntime::bind_kqueue(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            move |info, _| {
                seen_in_handler
                    .lock()
                    .expect("seen mutex poisoned")
                    .push(info.id);
                TcpHandlerResult::default()
            },
        )
        .expect("bind");
        let addr = runtime.local_addr();
        let mailbox = runtime.mailbox();
        let stop = runtime.stop_handle();

        let server = thread::spawn(move || runtime.serve());

        let mut client = TcpStream::connect(addr).expect("connect");
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("read timeout");
        client.write_all(b"request").expect("write request");

        let mut connection_id = None;
        for _ in 0..200 {
            if let Some(id) = seen.lock().expect("seen mutex poisoned").first().copied() {
                connection_id = Some(id);
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let connection_id = connection_id.expect("handler should observe the request");

        mailbox.send(connection_id, b"delayed".to_vec());
        let mut response = [0u8; 7];
        client
            .read_exact(&mut response)
            .expect("read delayed response");
        assert_eq!(&response, b"delayed");

        stop.stop();
        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");
    }

    #[test]
    fn shard_bits_for_is_ceil_log2() {
        // 1 reactor needs no shard bits; otherwise we need enough bits to hold the
        // largest shard index (worker_count - 1), rounding up to a power of two.
        assert_eq!(shard_bits_for(0), 0);
        assert_eq!(shard_bits_for(1), 0);
        assert_eq!(shard_bits_for(2), 1);
        assert_eq!(shard_bits_for(3), 2);
        assert_eq!(shard_bits_for(4), 2);
        assert_eq!(shard_bits_for(5), 3);
        assert_eq!(shard_bits_for(8), 3);
        assert_eq!(shard_bits_for(9), 4);
        // Every shard index in `0..worker_count` must fit in `shard_bits` bits, so
        // the low-bit encoding never collides two distinct shards.
        for worker_count in 1..=64usize {
            let bits = shard_bits_for(worker_count);
            assert!((worker_count as u64 - 1) < (1u64 << bits.max(1)) || worker_count == 1);
            assert!((worker_count as u64) <= (1u64 << bits) || bits == 0);
        }
    }

    #[test]
    fn sharded_mailbox_routes_replies_through_the_owning_shard() {
        // Replies are sent back ONLY through the routed `TcpMailbox` (not through
        // `TcpHandlerResult`), so every reply must be routed to the shard that
        // accepted the connection.  A wrong shard mask would enqueue the reply on a
        // reactor that never saw the connection — its owner would never write it
        // and the client read would time out.  So a green run is end-to-end proof
        // that `ConnectionId` shard-encoding + `TcpMailbox` routing agree.
        let mailbox_slot: Arc<OnceLock<TcpMailbox>> = Arc::new(OnceLock::new());
        let mailbox_for_handler = Arc::clone(&mailbox_slot);
        let mut runtime = TcpRuntime::bind_kqueue_sharded(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            4,
            move |info, bytes| {
                if let Some(mailbox) = mailbox_for_handler.get() {
                    mailbox.send(info.id, bytes.to_vec());
                }
                TcpHandlerResult::default()
            },
        )
        .expect("bind sharded runtime");
        let addr = runtime.local_addr();
        // Publish the mailbox the handlers route through (set before serve starts).
        mailbox_slot.set(runtime.mailbox()).ok();
        let stop = runtime.stop_handle();
        let server = thread::spawn(move || runtime.serve());

        let client_count = 24usize;
        let barrier = Arc::new(Barrier::new(client_count));
        let mut clients = Vec::new();
        for i in 0..client_count {
            let barrier = Arc::clone(&barrier);
            clients.push(thread::spawn(move || -> Result<Vec<u8>, String> {
                barrier.wait();
                let mut stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .map_err(|e| e.to_string())?;
                let payload = format!("shard-reply-{i}").into_bytes();
                stream.write_all(&payload).map_err(|e| e.to_string())?;
                let mut echoed = vec![0u8; payload.len()];
                stream.read_exact(&mut echoed).map_err(|e| e.to_string())?;
                Ok(echoed)
            }));
        }

        for (i, client) in clients.into_iter().enumerate() {
            let echoed = client.join().expect("client thread").expect("client io");
            assert_eq!(echoed, format!("shard-reply-{i}").into_bytes());
        }

        stop.stop();
        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");
    }

    #[test]
    fn close_callback_receives_final_connection_state_once() {
        let closed = Arc::new(Mutex::new(Vec::<(ConnectionId, Vec<u8>)>::new()));
        let closed_observer = Arc::clone(&closed);
        let mut runtime = TcpRuntime::bind_kqueue_with_state(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            |_| Vec::<u8>::new(),
            |_, state, bytes| {
                state.extend_from_slice(bytes);
                TcpHandlerResult::close()
            },
            move |info, state| {
                closed_observer
                    .lock()
                    .expect("close observer mutex poisoned")
                    .push((info.id, state));
            },
        )
        .expect("bind");
        let addr = runtime.local_addr();
        let stop = runtime.stop_handle();

        let server = thread::spawn(move || runtime.serve());

        let mut client = TcpStream::connect(addr).expect("connect");
        client.write_all(b"bye").expect("write request");
        let mut sink = [0u8; 1];
        let _ = client.read(&mut sink);

        stop.stop();
        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");

        let observed = closed.lock().expect("close observer mutex poisoned");
        assert_eq!(observed.len(), 1, "close callback should run exactly once");
        assert_eq!(observed[0].1, b"bye".to_vec());
    }

    #[test]
    fn rejects_connections_above_the_configured_cap() {
        let options = TcpRuntimeOptions {
            max_connections: 2,
            ..TcpRuntimeOptions::default()
        };
        let mut runtime = TcpRuntime::bind_kqueue(("127.0.0.1", 0), options, |_, bytes| {
            TcpHandlerResult::write(bytes.to_vec())
        })
        .expect("bind");
        let addr = runtime.local_addr();
        let stop = runtime.stop_handle();

        let server = thread::spawn(move || runtime.serve());

        let _client_a = TcpStream::connect(addr).expect("connect a");
        let _client_b = TcpStream::connect(addr).expect("connect b");
        thread::sleep(Duration::from_millis(50));

        let mut client_c = TcpStream::connect(addr).expect("connect c");
        let outcome = attempt_round_trip(&mut client_c, b"overflow");
        match outcome {
            Ok(response) => assert_ne!(response, b"overflow"),
            Err(err) => assert!(matches!(
                err.kind(),
                io::ErrorKind::BrokenPipe
                    | io::ErrorKind::ConnectionReset
                    | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::NotConnected
            )),
        }

        stop.stop();
        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");
    }

    #[test]
    fn closes_connections_that_exceed_the_pending_write_budget() {
        let options = TcpRuntimeOptions {
            max_pending_write_bytes: 32,
            ..TcpRuntimeOptions::default()
        };
        let mut runtime = TcpRuntime::bind_kqueue(("127.0.0.1", 0), options, |_, _| {
            TcpHandlerResult::write(vec![1u8; 64])
        })
        .expect("bind");
        let addr = runtime.local_addr();
        let stop = runtime.stop_handle();

        let server = thread::spawn(move || runtime.serve());

        let mut client = TcpStream::connect(addr).expect("connect");
        let outcome = attempt_round_trip(&mut client, b"trigger");
        match outcome {
            Ok(response) => assert_ne!(response, vec![1u8; 64]),
            Err(err) => assert!(matches!(
                err.kind(),
                io::ErrorKind::BrokenPipe
                    | io::ErrorKind::ConnectionReset
                    | io::ErrorKind::ConnectionAborted
                    | io::ErrorKind::NotConnected
            )),
        }

        stop.stop();
        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");
    }

    #[test]
    fn stop_handle_shuts_down_the_server() {
        let mut runtime = TcpRuntime::bind_kqueue(
            ("127.0.0.1", 0),
            TcpRuntimeOptions::default(),
            |_, bytes| TcpHandlerResult::write(bytes.to_vec()),
        )
        .expect("bind");
        let stop = runtime.stop_handle();

        let server = thread::spawn(move || runtime.serve());
        thread::sleep(Duration::from_millis(20));
        stop.stop();

        let result = server.join().expect("server thread");
        assert!(result.is_ok(), "server should exit cleanly: {result:?}");
    }

    fn attempt_round_trip(stream: &mut TcpStream, payload: &[u8]) -> io::Result<Vec<u8>> {
        stream.write_all(payload)?;
        stream.shutdown(Shutdown::Write)?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response)?;
        Ok(response)
    }
}
