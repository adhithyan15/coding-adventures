//! Completion engine: Windows I/O completion ports and Winsock overlapped I/O.
//!
//! # Readiness versus completion
//!
//! The Unix engine asks the kernel "which sockets are ready?" and then reads or
//! writes until the call would block. Windows turns that around. You *start* an
//! operation (`AcceptEx`, `WSARecv`, `WSASend`), handing the kernel a buffer and
//! an `OVERLAPPED` record, and later the completion port hands you the record
//! back once the work is done:
//!
//! ```text
//!   post WSARecv(buf) ──────────────► kernel fills buf from the network
//!                                          │
//!   GetQueuedCompletionStatus ◄────────────┘  "that WSARecv finished: n bytes"
//! ```
//!
//! # The one rule that shapes this file
//!
//! **The kernel owns an operation's buffer and its `OVERLAPPED` from the moment
//! the operation is posted until its completion packet has been dequeued.**
//! Closing the socket does not change that: it cancels the operation, but the
//! cancelled operation still completes (with `ERROR_OPERATION_ABORTED`), and
//! until then the kernel may still write into the buffer. Freeing it early is a
//! use-after-free that the kernel performs for us, silently.
//!
//! So every operation lives in a heap [`Op`] with a stable address, a connection
//! that is closing stays in the table with no new operations posted, and its
//! `Op`s are freed only after their completions have come back. If a drain
//! deadline passes with operations still outstanding, their memory is leaked on
//! purpose: a leak is harmless, a use-after-free is not.
//!
//! # Shape
//!
//! ```text
//!   listener ── ACCEPT_DEPTH × AcceptEx ──► adopt socket, post WSARecv, re-post AcceptEx
//!   connection ── 1 × WSARecv in flight ──► handler(bytes) ──► pending_write
//!              ── 1 × WSASend in flight ◄── pending_write moves into the send buffer
//! ```

use crate::{
    ConnectionInfo, Handler, StopHandle, DEFAULT_MAX_CONNECTIONS, DEFAULT_MAX_PENDING_WRITE_BYTES,
    POLL_INTERVAL_MS,
};
use iocp::{CompletionPacket, CompletionPort, Overlapped};
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::io;
use std::mem;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::os::windows::io::{AsRawSocket, FromRawSocket};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Hand-written Winsock declarations, in the style of the `iocp` crate: no
/// external bindings crate, just the handful of calls this engine needs.
mod sys {
    use super::{c_void, Overlapped};

    pub type Socket = usize;
    pub const INVALID_SOCKET: Socket = !0;
    pub const SOCKET_ERROR: i32 = -1;

    pub const AF_INET: i32 = 2;
    pub const AF_INET6: i32 = 23;
    pub const SOCK_STREAM: i32 = 1;
    pub const IPPROTO_TCP: i32 = 6;
    pub const WSA_FLAG_OVERLAPPED: u32 = 0x01;
    pub const WSA_FLAG_NO_HANDLE_INHERIT: u32 = 0x80;

    pub const SOL_SOCKET: i32 = 0xffff;
    pub const SO_UPDATE_ACCEPT_CONTEXT: i32 = 0x700B;

    pub const ERROR_OPERATION_ABORTED: i32 = 995;
    pub const ERROR_IO_PENDING: i32 = 997;

    /// `WSABUF`: a length and a pointer, in that order on Windows.
    #[repr(C)]
    pub struct WsaBuf {
        pub len: u32,
        pub buf: *mut u8,
    }

    #[link(name = "ws2_32")]
    unsafe extern "system" {
        pub fn WSASocketW(
            af: i32,
            kind: i32,
            protocol: i32,
            protocol_info: *mut c_void,
            group: u32,
            flags: u32,
        ) -> Socket;
        pub fn WSARecv(
            socket: Socket,
            buffers: *mut WsaBuf,
            buffer_count: u32,
            received: *mut u32,
            flags: *mut u32,
            overlapped: *mut Overlapped,
            completion_routine: *mut c_void,
        ) -> i32;
        pub fn WSASend(
            socket: Socket,
            buffers: *mut WsaBuf,
            buffer_count: u32,
            sent: *mut u32,
            flags: u32,
            overlapped: *mut Overlapped,
            completion_routine: *mut c_void,
        ) -> i32;
        pub fn closesocket(socket: Socket) -> i32;
        pub fn setsockopt(
            socket: Socket,
            level: i32,
            name: i32,
            value: *const u8,
            length: i32,
        ) -> i32;
        pub fn WSAGetLastError() -> i32;
    }

    #[link(name = "mswsock")]
    unsafe extern "system" {
        pub fn AcceptEx(
            listen_socket: Socket,
            accept_socket: Socket,
            output_buffer: *mut c_void,
            receive_data_length: u32,
            local_address_length: u32,
            remote_address_length: u32,
            bytes_received: *mut u32,
            overlapped: *mut Overlapped,
        ) -> i32;
    }
}

use sys::{Socket, INVALID_SOCKET};

/// Completion key for the listener's `AcceptEx` operations.
const LISTENER_KEY: usize = 1;
/// `AcceptEx` operations kept posted at once. More than one so a burst of
/// connections does not have to wait for a single re-post round trip each.
const ACCEPT_DEPTH: usize = 8;
/// Room `AcceptEx` needs for one address: the largest `sockaddr` (IPv6, 28
/// bytes) plus the 16 bytes the API requires on top.
const ACCEPT_ADDRESS_LENGTH: usize = 64;
/// How long a stopping reactor waits for cancelled operations to complete
/// before it gives up and leaks whatever is still outstanding.
const DRAIN_DEADLINE: Duration = Duration::from_secs(5);
/// Same, for the accept operations of a reactor being dropped.
const DROP_DRAIN_DEADLINE: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OpKind {
    Accept,
    Recv,
    Send,
}

/// One overlapped operation. `#[repr(C)]` with `overlapped` first: the kernel
/// returns the address of that field, and the cast back to `*mut Op` relies on it
/// being at offset zero.
#[repr(C)]
struct Op {
    overlapped: Overlapped,
    kind: OpKind,
    /// The connection this operation belongs to (`0` for accepts).
    id: u64,
    /// True from a successful post until its completion packet is dequeued.
    in_flight: bool,
    /// The buffer the kernel reads or writes. Never resized while `in_flight`.
    buf: Vec<u8>,
    /// For an accept: the socket being accepted into.
    socket: Socket,
}

fn blank_overlapped() -> Overlapped {
    Overlapped {
        internal: 0,
        internal_high: 0,
        offset: 0,
        offset_high: 0,
        h_event: ptr::null_mut(),
    }
}

/// Allocate an operation with a stable heap address.
fn new_op(kind: OpKind, id: u64, buffer_len: usize) -> *mut Op {
    Box::into_raw(Box::new(Op {
        overlapped: blank_overlapped(),
        kind,
        id,
        in_flight: false,
        buf: vec![0u8; buffer_len],
        socket: INVALID_SOCKET,
    }))
}

/// Free an operation, or leak it if the kernel may still be using it.
///
/// # Safety
/// `op` must have come from [`new_op`] and not have been freed already.
unsafe fn release_op(op: *mut Op) {
    if !(*op).in_flight {
        drop(Box::from_raw(op));
    }
}

fn last_wsa_error() -> io::Error {
    io::Error::from_raw_os_error(unsafe { sys::WSAGetLastError() })
}

struct Conn {
    info: ConnectionInfo,
    /// Owns the socket. `None` once the connection is closing: dropping the
    /// stream is what calls `closesocket`, which cancels the operations in flight.
    stream: Option<TcpStream>,
    socket: Socket,
    recv: *mut Op,
    send: *mut Op,
    /// How much of `send.buf` the kernel has already sent.
    send_offset: usize,
    /// Output waiting for the in-flight send to finish.
    pending_write: Vec<u8>,
    peer_closed: bool,
    closing: bool,
}

impl Conn {
    /// Bytes accepted from the handler that have not reached the kernel yet:
    /// what is left of the in-flight send plus everything still pending.
    fn queued(&self) -> usize {
        let in_flight = unsafe {
            if (*self.send).in_flight {
                (*self.send).buf.len() - self.send_offset
            } else {
                0
            }
        };
        in_flight + self.pending_write.len()
    }

    fn has_operations_in_flight(&self) -> bool {
        unsafe { (*self.recv).in_flight || (*self.send).in_flight }
    }

    #[cfg(test)]
    fn has_operations_in_flight_send(&self) -> bool {
        unsafe { (*self.send).in_flight }
    }
}

impl Drop for Conn {
    fn drop(&mut self) {
        // An operation still in flight is leaked, never freed (see the module docs).
        unsafe {
            release_op(self.recv);
            release_op(self.send);
        }
    }
}

pub struct TcpReactor {
    port: CompletionPort,
    /// Held only to keep the listening socket open; the engine talks to it through
    /// `listener_socket`.
    #[allow(dead_code)]
    listener: TcpListener,
    listener_addr: SocketAddr,
    listener_socket: Socket,
    accepts: Vec<*mut Op>,
    connections: BTreeMap<u64, Conn>,
    live_connections: usize,
    next_id: AtomicU64,
    stop_flag: Arc<AtomicBool>,
    read_buffer_size: usize,
    max_connections: usize,
    max_pending_write_bytes: usize,
    handler: Handler,
}

// SAFETY: the raw pointers are `Op`s that this reactor alone allocates, posts and
// frees; the kernel touches them only while they are in flight and only until the
// reactor dequeues their completion. Every method takes `&mut self`, so only one
// thread runs the engine at a time, and moving the reactor to another thread
// moves that exclusive ownership with it. The completion port is `Send + Sync`.
unsafe impl Send for TcpReactor {}

impl TcpReactor {
    /// Bind `addr` and serve it through an I/O completion port.
    pub fn bind_iocp<A, F>(addr: A, handler: F) -> io::Result<Self>
    where
        A: std::net::ToSocketAddrs,
        F: Fn(ConnectionInfo, &[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        Self::with_listener(TcpListener::bind(addr)?, handler)
    }

    /// Serve an already-bound listener through an I/O completion port.
    pub fn with_listener<F>(listener: TcpListener, handler: F) -> io::Result<Self>
    where
        F: Fn(ConnectionInfo, &[u8]) -> Vec<u8> + Send + Sync + 'static,
    {
        let listener_addr = listener.local_addr()?;
        let listener_socket = listener.as_raw_socket() as Socket;

        // One thread drives this port, so one concurrent completion at a time.
        let port = CompletionPort::new(1)?;
        port.associate_handle(listener_socket as *mut c_void, LISTENER_KEY)?;

        let accepts = (0..ACCEPT_DEPTH)
            .map(|_| new_op(OpKind::Accept, 0, ACCEPT_ADDRESS_LENGTH * 2))
            .collect();

        Ok(Self {
            port,
            listener,
            listener_addr,
            listener_socket,
            accepts,
            connections: BTreeMap::new(),
            live_connections: 0,
            next_id: AtomicU64::new(2),
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
        self.arm_accepts()?;
        while !self.stop_flag.load(Ordering::SeqCst) {
            self.step(Duration::from_millis(POLL_INTERVAL_MS), false)?;
        }
        self.shutdown_all()
    }

    /// Wait up to `timeout` for one completion and handle it. Returns whether a
    /// completion was handled.
    fn step(&mut self, timeout: Duration, draining: bool) -> io::Result<bool> {
        match self.port.get(Some(timeout)) {
            Ok(packet) => {
                self.dispatch(packet, draining)?;
                Ok(true)
            }
            Err(err) if err.kind() == io::ErrorKind::TimedOut => Ok(false),
            Err(err) => Err(err),
        }
    }

    fn dispatch(&mut self, packet: CompletionPacket, draining: bool) -> io::Result<()> {
        if packet.overlapped.is_null() {
            return Ok(());
        }
        // SAFETY: every OVERLAPPED we post is the first field of a live `Op`
        // (`#[repr(C)]`), and an `Op` is freed only after its completion arrives
        // here, so the pointer is valid.
        let op = packet.overlapped as *mut Op;
        let (kind, id) = unsafe {
            (*op).in_flight = false;
            ((*op).kind, (*op).id)
        };
        let error = packet.error;
        let bytes = packet.bytes_transferred as usize;
        match kind {
            OpKind::Accept => self.accept_completed(op, error, draining),
            OpKind::Recv => self.recv_completed(id, bytes, error),
            OpKind::Send => self.send_completed(id, bytes, error),
        }
    }

    // ── accept ──────────────────────────────────────────────────────────────

    /// Post every accept operation that is not already in flight.
    fn arm_accepts(&mut self) -> io::Result<()> {
        let accepts = self.accepts.clone();
        for op in accepts {
            if unsafe { !(*op).in_flight } {
                self.post_accept(op)?;
            }
        }
        Ok(())
    }

    fn post_accept(&mut self, op: *mut Op) -> io::Result<()> {
        let family = if self.listener_addr.is_ipv4() {
            sys::AF_INET
        } else {
            sys::AF_INET6
        };
        unsafe {
            let accept_socket = sys::WSASocketW(
                family,
                sys::SOCK_STREAM,
                sys::IPPROTO_TCP,
                ptr::null_mut(),
                0,
                sys::WSA_FLAG_OVERLAPPED | sys::WSA_FLAG_NO_HANDLE_INHERIT,
            );
            if accept_socket == INVALID_SOCKET {
                return Err(last_wsa_error());
            }
            (*op).overlapped = blank_overlapped();
            (*op).socket = accept_socket;
            let mut received = 0u32;
            let ok = sys::AcceptEx(
                self.listener_socket,
                accept_socket,
                (*op).buf.as_mut_ptr() as *mut c_void,
                0,
                ACCEPT_ADDRESS_LENGTH as u32,
                ACCEPT_ADDRESS_LENGTH as u32,
                &mut received,
                &mut (*op).overlapped,
            );
            if ok == 0 {
                let error = sys::WSAGetLastError();
                if error != sys::ERROR_IO_PENDING {
                    sys::closesocket(accept_socket);
                    (*op).socket = INVALID_SOCKET;
                    return Err(io::Error::from_raw_os_error(error));
                }
            }
            // Even a synchronous success queues a completion packet (we never
            // ask for it to be skipped), so the operation is in flight either way.
            (*op).in_flight = true;
        }
        Ok(())
    }

    fn accept_completed(
        &mut self,
        op: *mut Op,
        error: Option<i32>,
        draining: bool,
    ) -> io::Result<()> {
        // Take ownership of the accepted socket out of the operation.
        let accept_socket = unsafe { mem::replace(&mut (*op).socket, INVALID_SOCKET) };

        match error {
            None if accept_socket != INVALID_SOCKET => self.adopt(accept_socket, draining)?,
            _ => {
                if accept_socket != INVALID_SOCKET {
                    unsafe { sys::closesocket(accept_socket) };
                }
                // An aborted accept means the listener is going away, so no
                // replacement is posted. Any other failure belongs to that one
                // attempt: fall through and post another.
                if error == Some(sys::ERROR_OPERATION_ABORTED) {
                    return Ok(());
                }
            }
        }

        if draining || self.stop_flag.load(Ordering::SeqCst) {
            // `serve` re-arms any accept that is not in flight when it starts.
            return Ok(());
        }
        self.post_accept(op)
    }

    /// Turn a completed accept socket into a connection (or close it).
    fn adopt(&mut self, accept_socket: Socket, draining: bool) -> io::Result<()> {
        // Inherit the listener's properties so getpeername/getsockname work.
        let listener = self.listener_socket;
        let updated = unsafe {
            sys::setsockopt(
                accept_socket,
                sys::SOL_SOCKET,
                sys::SO_UPDATE_ACCEPT_CONTEXT,
                &listener as *const Socket as *const u8,
                mem::size_of::<Socket>() as i32,
            )
        };
        // From here the `TcpStream` owns the socket and closes it on drop.
        let stream = unsafe { TcpStream::from_raw_socket(accept_socket as _) };
        if updated != 0 || draining || self.live_connections >= self.max_connections {
            let _ = stream.shutdown(Shutdown::Both);
            return Ok(());
        }
        let (peer_addr, local_addr) = match (stream.peer_addr(), stream.local_addr()) {
            (Ok(peer), Ok(local)) => (peer, local),
            // The peer already vanished; nothing to serve.
            _ => return Ok(()),
        };

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        if self
            .port
            .associate_handle(accept_socket as *mut c_void, id as usize)
            .is_err()
        {
            return Ok(());
        }
        let conn = Conn {
            info: ConnectionInfo {
                id,
                peer_addr,
                local_addr,
            },
            stream: Some(stream),
            socket: accept_socket,
            recv: new_op(OpKind::Recv, id, self.read_buffer_size),
            send: new_op(OpKind::Send, id, 0),
            send_offset: 0,
            pending_write: Vec::new(),
            peer_closed: false,
            closing: false,
        };
        self.connections.insert(id, conn);
        self.live_connections += 1;
        self.pump(id)
    }

    // ── per-connection completions ──────────────────────────────────────────

    fn recv_completed(&mut self, id: u64, bytes: usize, error: Option<i32>) -> io::Result<()> {
        let Some(conn) = self.connections.get_mut(&id) else {
            return Ok(());
        };
        if conn.closing {
            return self.reap(id);
        }
        if error.is_some() {
            // Reset, aborted, or anything else: it only ends this connection.
            return self.close_connection(id);
        }
        if bytes == 0 {
            // The peer shut down its sending side. Flush what we owe, then close.
            conn.peer_closed = true;
        } else {
            // SAFETY: the receive completed, so the kernel is done with the
            // buffer, and `bytes <= buf.len()` because it was the posted length.
            let receive_op = unsafe { &*conn.recv };
            let data = &receive_op.buf[..bytes];
            let response = (self.handler)(conn.info, data);
            if !response.is_empty() {
                if conn.queued().saturating_add(response.len()) > self.max_pending_write_bytes {
                    return self.close_connection(id);
                }
                conn.pending_write.extend_from_slice(&response);
            }
        }
        self.pump(id)
    }

    fn send_completed(&mut self, id: u64, bytes: usize, error: Option<i32>) -> io::Result<()> {
        let Some(conn) = self.connections.get_mut(&id) else {
            return Ok(());
        };
        if conn.closing {
            return self.reap(id);
        }
        if error.is_some() {
            return self.close_connection(id);
        }
        conn.send_offset += bytes;
        // SAFETY: the send completed, so the kernel is no longer reading `buf`.
        let send_buf_len = unsafe { (*conn.send).buf.len() };
        if conn.send_offset < send_buf_len {
            // A partial send: post the rest. Defensive: overlapped stream sends
            // normally complete whole, but the API does not promise it.
            if self.post_send(id).is_err() {
                return self.close_connection(id);
            }
            return Ok(());
        }
        // Fully sent: the buffer is free for the next batch.
        unsafe { (*conn.send).buf.clear() };
        conn.send_offset = 0;
        self.pump(id)
    }

    /// Decide what to post next for a connection that is not mid-operation:
    /// start a send if output is waiting, close it if it has nothing left to do,
    /// or post the next receive.
    fn pump(&mut self, id: u64) -> io::Result<()> {
        let max_pending = self.max_pending_write_bytes;
        let Some(conn) = self.connections.get_mut(&id) else {
            return Ok(());
        };
        if conn.closing {
            return Ok(());
        }

        let send_in_flight = unsafe { (*conn.send).in_flight };
        if !send_in_flight && !conn.pending_write.is_empty() {
            // SAFETY: no send is in flight, so the kernel is not using the buffer.
            // Swapping hands the pending bytes to the send op and keeps the
            // emptied buffer (and its capacity) for the next batch.
            unsafe { mem::swap(&mut (*conn.send).buf, &mut conn.pending_write) };
            conn.send_offset = 0;
            if self.post_send(id).is_err() {
                return self.close_connection(id);
            }
        }

        let Some(conn) = self.connections.get_mut(&id) else {
            return Ok(());
        };
        let send_in_flight = unsafe { (*conn.send).in_flight };
        let recv_in_flight = unsafe { (*conn.recv).in_flight };
        let flushed = !send_in_flight && conn.pending_write.is_empty();

        if conn.peer_closed && flushed {
            return self.close_connection(id);
        }
        // Stop reading while the peer is not draining our output: this is the
        // same backpressure the readiness engine applies by dropping READABLE.
        let want_recv = !conn.peer_closed && !recv_in_flight && conn.queued() < max_pending;
        if want_recv && self.post_recv(id).is_err() {
            return self.close_connection(id);
        }
        Ok(())
    }

    fn post_recv(&mut self, id: u64) -> io::Result<()> {
        let conn = self.connections.get_mut(&id).expect("connection");
        unsafe {
            let op = conn.recv;
            (*op).overlapped = blank_overlapped();
            let mut buffer = sys::WsaBuf {
                len: (*op).buf.len() as u32,
                buf: (*op).buf.as_mut_ptr(),
            };
            let mut received = 0u32;
            let mut flags = 0u32;
            let rc = sys::WSARecv(
                conn.socket,
                &mut buffer,
                1,
                &mut received,
                &mut flags,
                &mut (*op).overlapped,
                ptr::null_mut(),
            );
            if rc == sys::SOCKET_ERROR {
                let error = sys::WSAGetLastError();
                if error != sys::ERROR_IO_PENDING {
                    return Err(io::Error::from_raw_os_error(error));
                }
            }
            (*op).in_flight = true;
        }
        Ok(())
    }

    fn post_send(&mut self, id: u64) -> io::Result<()> {
        let conn = self.connections.get_mut(&id).expect("connection");
        unsafe {
            let op = &mut *conn.send;
            op.overlapped = blank_overlapped();
            let remaining = &mut op.buf[conn.send_offset..];
            let mut buffer = sys::WsaBuf {
                len: remaining.len() as u32,
                buf: remaining.as_mut_ptr(),
            };
            let mut sent = 0u32;
            let rc = sys::WSASend(
                conn.socket,
                &mut buffer,
                1,
                &mut sent,
                0,
                &mut op.overlapped,
                ptr::null_mut(),
            );
            if rc == sys::SOCKET_ERROR {
                let error = sys::WSAGetLastError();
                if error != sys::ERROR_IO_PENDING {
                    return Err(io::Error::from_raw_os_error(error));
                }
            }
            op.in_flight = true;
        }
        Ok(())
    }

    // ── closing ─────────────────────────────────────────────────────────────

    /// Begin closing a connection: close its socket (which cancels whatever is in
    /// flight) and keep the record until those operations have completed.
    fn close_connection(&mut self, id: u64) -> io::Result<()> {
        if let Some(conn) = self.connections.get_mut(&id) {
            if !conn.closing {
                conn.closing = true;
                self.live_connections -= 1;
                // Dropping the stream calls closesocket.
                drop(conn.stream.take());
            }
        }
        self.reap(id)
    }

    /// Remove a closing connection once the kernel has finished with all of its
    /// operations.
    fn reap(&mut self, id: u64) -> io::Result<()> {
        let done = self
            .connections
            .get(&id)
            .is_some_and(|conn| conn.closing && !conn.has_operations_in_flight());
        if done {
            self.connections.remove(&id);
        }
        Ok(())
    }

    fn shutdown_all(&mut self) -> io::Result<()> {
        let ids: Vec<u64> = self.connections.keys().copied().collect();
        for id in ids {
            self.close_connection(id)?;
        }
        self.drain(DRAIN_DEADLINE, |reactor| reactor.connections.is_empty())
    }

    /// Keep handling completions until `finished` is true or `deadline` passes.
    fn drain(&mut self, deadline: Duration, finished: impl Fn(&Self) -> bool) -> io::Result<()> {
        let started = Instant::now();
        while !finished(self) && started.elapsed() < deadline {
            self.step(Duration::from_millis(20), true)?;
        }
        if !finished(self) {
            // Operations are still outstanding. Their `Conn`s are dropped
            // without freeing the in-flight `Op`s, which are leaked on purpose.
            self.connections.clear();
        }
        Ok(())
    }
}

impl Drop for TcpReactor {
    fn drop(&mut self) {
        // Close every connection, then cancel the accepts by closing the sockets
        // they would accept into; each still completes, with an abort error.
        let ids: Vec<u64> = self.connections.keys().copied().collect();
        for id in ids {
            let _ = self.close_connection(id);
        }
        for &op in &self.accepts {
            unsafe {
                if (*op).in_flight && (*op).socket != INVALID_SOCKET {
                    sys::closesocket((*op).socket);
                    (*op).socket = INVALID_SOCKET;
                }
            }
        }
        let _ = self.drain(DROP_DRAIN_DEADLINE, |reactor| {
            reactor.connections.is_empty()
                && reactor
                    .accepts
                    .iter()
                    .all(|&op| unsafe { !(*op).in_flight })
        });
        // Free the accept operations that have completed; leak any that have not.
        for &op in &self.accepts {
            unsafe { release_op(op) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn overlapped_is_the_first_field_of_an_op() {
        // The kernel returns the address of `overlapped`; `dispatch` casts it back
        // to `*mut Op`, which is only valid at offset zero.
        assert_eq!(mem::offset_of!(Op, overlapped), 0);
    }

    #[test]
    fn wsabuf_matches_the_windows_layout() {
        // WSABUF is { ULONG len; CHAR *buf; } on 64-bit Windows: 4 + 4 pad + 8.
        assert_eq!(mem::size_of::<sys::WsaBuf>(), 16);
        assert_eq!(mem::offset_of!(sys::WsaBuf, buf), 8);
    }

    /// Run the reactor by hand until `done` holds, for tests that need to observe
    /// the engine's bookkeeping between completions.
    fn run_until(reactor: &mut TcpReactor, what: &str, done: impl Fn(&TcpReactor) -> bool) {
        let started = Instant::now();
        while !done(reactor) {
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "timed out waiting for: {what}"
            );
            reactor
                .step(Duration::from_millis(20), false)
                .expect("step");
        }
    }

    #[test]
    fn a_closing_connection_stays_until_its_operations_complete() {
        let mut reactor =
            TcpReactor::bind_iocp(("127.0.0.1", 0), |_, bytes| bytes.to_vec()).expect("bind");
        reactor.arm_accepts().expect("arm");
        let mut client = TcpStream::connect(reactor.local_addr()).expect("connect");

        run_until(&mut reactor, "connection adopted", |r| {
            r.connections.len() == 1
        });
        let id = *reactor.connections.keys().next().expect("id");
        // A receive is posted as soon as the connection is adopted.
        assert!(reactor.connections[&id].has_operations_in_flight());

        reactor.close_connection(id).expect("close");
        assert!(
            reactor.connections.contains_key(&id),
            "the record must outlive its in-flight receive"
        );
        assert_eq!(reactor.live_connections, 0);

        // The cancelled receive completes with an abort, and only then is the
        // record (and its buffer) released.
        run_until(&mut reactor, "cancelled receive reaped", |r| {
            r.connections.is_empty()
        });

        let mut byte = [0u8; 1];
        let closed = matches!(client.read(&mut byte), Ok(0) | Err(_));
        assert!(closed, "the client should see the connection closed");
    }

    #[test]
    fn a_large_response_drains_through_the_send_queue() {
        // A response far larger than the socket send buffer must go out through the
        // pending-write queue and arrive complete and in order.
        //
        // This does NOT exercise the "partial send" branch of `send_completed`:
        // an overlapped `WSASend` on a stream socket completes with every byte
        // or an error, so that branch is defensive and cannot be provoked here
        // (mutating it away leaves every test green; that was checked).
        const TOTAL: usize = 4 * 1024 * 1024;
        let handled = Arc::new(AtomicBool::new(false));
        let handler_flag = Arc::clone(&handled);
        let mut reactor = TcpReactor::bind_iocp(("127.0.0.1", 0), move |_, _| {
            handler_flag.store(true, Ordering::SeqCst);
            vec![7u8; TOTAL]
        })
        .expect("bind");
        reactor.set_max_pending_write_bytes(TOTAL * 2);
        reactor.arm_accepts().expect("arm");

        let mut client = TcpStream::connect(reactor.local_addr()).expect("connect");
        client.write_all(b"go").expect("write");

        let reader = std::thread::spawn(move || {
            let mut received = 0usize;
            let mut chunk = vec![0u8; 64 * 1024];
            client
                .set_read_timeout(Some(Duration::from_secs(10)))
                .expect("timeout");
            while received < TOTAL {
                let n = client.read(&mut chunk).expect("read");
                assert!(n > 0, "closed early at {received}");
                assert!(chunk[..n].iter().all(|&b| b == 7));
                received += n;
            }
            received
        });

        // Done once the handler has run and every queued byte has been sent.
        run_until(&mut reactor, "full response sent", |r| {
            handled.load(Ordering::SeqCst)
                && r.connections
                    .values()
                    .all(|conn| conn.queued() == 0 && !conn.has_operations_in_flight_send())
                && !r.connections.is_empty()
        });
        assert_eq!(reader.join().expect("reader"), TOTAL);
    }
}
