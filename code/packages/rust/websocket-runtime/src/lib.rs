//! TCP adapters for the transport-independent `websocket-core` state machine.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_csprng::fill_random;
use core::fmt::{self, Display, Formatter};
use std::collections::VecDeque;
use std::net::{SocketAddr, ToSocketAddrs};

use tcp_client::{ConnectOptions, TcpConnection, TcpError};
use tcp_runtime::{
    BindAddress, PlatformError, TcpHandlerResult, TcpMailbox, TcpRuntime, TcpRuntimeOptions,
};
use transport_platform::TransportPlatform;
use websocket_core::{
    accept_server_request, build_client_request, control_reply, encode_frame,
    validate_client_response, EndpointRole, Frame, FrameDecoder, MessageAssembler, MessageEvent,
    WebSocketError, MAX_HANDSHAKE_BYTES,
};

pub use tcp_runtime::{ConnectionId, StopHandle, TcpConnectionInfo as WebSocketConnectionInfo};

const BAD_REQUEST: &[u8] =
    b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n";

/// A transport, entropy, option, or RFC 6455 session failure.
#[derive(Debug)]
pub enum WebSocketRuntimeError {
    /// A runtime size or buffer option was zero.
    InvalidOptions,
    /// A listener address could not be resolved.
    InvalidAddress,
    /// The repository TCP client failed.
    Tcp(TcpError),
    /// The repository TCP server platform failed.
    Platform(PlatformError),
    /// The portable WebSocket protocol core rejected input.
    Protocol(WebSocketError),
    /// The operating-system CSPRNG was unavailable.
    EntropyUnavailable,
    /// TCP ended without a completed WebSocket closing handshake.
    AbnormalEof,
    /// The caller tried to use a session after its closing handshake began.
    ClosedSession,
}

impl Display for WebSocketRuntimeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidOptions => "websocket runtime: invalid options",
            Self::InvalidAddress => "websocket runtime: invalid listener address",
            Self::Tcp(_) => "websocket runtime: tcp client failure",
            Self::Platform(_) => "websocket runtime: tcp server failure",
            Self::Protocol(_) => "websocket runtime: protocol failure",
            Self::EntropyUnavailable => "websocket runtime: OS entropy unavailable",
            Self::AbnormalEof => "websocket runtime: abnormal EOF",
            Self::ClosedSession => "websocket runtime: session is closing",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for WebSocketRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Tcp(error) => Some(error),
            Self::Platform(error) => Some(error),
            Self::Protocol(error) => Some(error),
            Self::InvalidOptions
            | Self::InvalidAddress
            | Self::EntropyUnavailable
            | Self::AbnormalEof
            | Self::ClosedSession => None,
        }
    }
}

impl From<TcpError> for WebSocketRuntimeError {
    fn from(error: TcpError) -> Self {
        Self::Tcp(error)
    }
}

impl From<PlatformError> for WebSocketRuntimeError {
    fn from(error: PlatformError) -> Self {
        Self::Platform(error)
    }
}

impl From<WebSocketError> for WebSocketRuntimeError {
    fn from(error: WebSocketError) -> Self {
        Self::Protocol(error)
    }
}

/// Entropy authority used for client handshake nonces and frame mask keys.
pub trait EntropySource {
    /// Completely overwrite `output` with unpredictable bytes.
    fn fill(&mut self, output: &mut [u8]) -> Result<(), WebSocketRuntimeError>;
}

/// Production entropy backed by the repository OS CSPRNG package.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsEntropy;

impl EntropySource for OsEntropy {
    fn fill(&mut self, output: &mut [u8]) -> Result<(), WebSocketRuntimeError> {
        fill_random(output).map_err(|_| WebSocketRuntimeError::EntropyUnavailable)
    }
}

/// Bounds and TCP policy for a many-connection WebSocket server.
#[derive(Clone, Debug)]
pub struct WebSocketServerOptions {
    /// Underlying listener, accepted-stream, and reactor policy.
    pub tcp: TcpRuntimeOptions,
    /// Maximum bytes accepted in one WebSocket frame payload.
    pub max_frame_payload: usize,
    /// Maximum bytes accepted in one assembled data message.
    pub max_message_payload: usize,
}

impl Default for WebSocketServerOptions {
    fn default() -> Self {
        Self {
            tcp: TcpRuntimeOptions::default(),
            max_frame_payload: 1024 * 1024,
            max_message_payload: 8 * 1024 * 1024,
        }
    }
}

/// Application output for one complete inbound WebSocket event.
#[derive(Clone, Debug, Default)]
pub struct WebSocketHandlerResult {
    /// Validated outbound frames, serialized in order by the runtime.
    pub frames: Vec<Frame>,
    /// Whether TCP should close after all returned frames flush.
    pub close: bool,
}

impl WebSocketHandlerResult {
    /// Return one outbound frame while keeping the session open.
    pub fn send(frame: Frame) -> Self {
        Self {
            frames: vec![frame],
            close: false,
        }
    }

    /// Return several outbound frames while keeping the session open.
    pub fn send_many(frames: impl Into<Vec<Frame>>) -> Self {
        Self {
            frames: frames.into(),
            close: false,
        }
    }

    /// Construct and send a close frame, then close TCP after it flushes.
    pub fn close(code: Option<u16>, reason: &str) -> Result<Self, WebSocketRuntimeError> {
        Ok(Self {
            frames: vec![Frame::close(code, reason)?],
            close: true,
        })
    }
}

struct WebSocketSession {
    decoder: FrameDecoder,
    assembler: MessageAssembler,
}

impl WebSocketSession {
    fn new(role: EndpointRole, max_frame_payload: usize, max_message_payload: usize) -> Self {
        Self {
            decoder: FrameDecoder::new(role, max_frame_payload),
            assembler: MessageAssembler::new(max_message_payload),
        }
    }

    fn push(&mut self, input: &[u8]) -> Result<Vec<MessageEvent>, WebSocketError> {
        let mut events = Vec::new();
        for frame in self.decoder.push(input)? {
            if let Some(event) = self.assembler.push(frame)? {
                events.push(event);
            }
        }
        Ok(events)
    }
}

enum ServerPhase {
    Handshake(Vec<u8>),
    Open(WebSocketSession),
    Closing,
}

struct ServerConnectionState<S> {
    application: S,
    phase: ServerPhase,
}

/// Many-connection WebSocket server composed over `tcp-runtime`.
pub struct WebSocketRuntime<P, S = ()> {
    inner: TcpRuntime<P, ServerConnectionState<S>>,
}

impl<P: TransportPlatform> WebSocketRuntime<P, ()> {
    /// Bind a stateless WebSocket server to a caller-provided transport platform.
    pub fn bind<F>(
        platform: P,
        address: BindAddress,
        options: WebSocketServerOptions,
        handler: F,
    ) -> Result<Self, WebSocketRuntimeError>
    where
        F: Fn(WebSocketConnectionInfo, MessageEvent) -> WebSocketHandlerResult
            + Send
            + Sync
            + 'static,
    {
        Self::bind_with_state(
            platform,
            address,
            options,
            |_| (),
            move |info, _, event| handler(info, event),
            |_, _| {},
        )
    }
}

impl<P: TransportPlatform, S: Send + 'static> WebSocketRuntime<P, S> {
    /// Bind a stateful WebSocket server to a caller-provided transport platform.
    pub fn bind_with_state<I, F, C>(
        platform: P,
        address: BindAddress,
        options: WebSocketServerOptions,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<Self, WebSocketRuntimeError>
    where
        I: Fn(WebSocketConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(WebSocketConnectionInfo, &mut S, MessageEvent) -> WebSocketHandlerResult
            + Send
            + Sync
            + 'static,
        C: Fn(WebSocketConnectionInfo, S) + Send + Sync + 'static,
    {
        validate_server_options(&options)?;
        let max_frame_payload = options.max_frame_payload;
        let max_message_payload = options.max_message_payload;
        let inner = TcpRuntime::bind_with_state(
            platform,
            address,
            options.tcp,
            move |info| ServerConnectionState {
                application: init(info),
                phase: ServerPhase::Handshake(Vec::new()),
            },
            move |info, state, bytes| {
                server_input(
                    info,
                    state,
                    bytes,
                    max_frame_payload,
                    max_message_payload,
                    &handler,
                )
            },
            move |info, state| on_close(info, state.application),
        )?;
        Ok(Self { inner })
    }

    /// Return the concrete local address selected by the TCP listener.
    pub fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }

    /// Return a cloneable cooperative stop handle.
    pub fn stop_handle(&self) -> StopHandle {
        self.inner.stop_handle()
    }

    /// Return the underlying TCP mailbox for advanced byte-level integrations.
    ///
    /// Bytes submitted here must already be complete server-role WebSocket
    /// frames. Normal application replies should use `WebSocketHandlerResult`.
    pub fn tcp_mailbox(&self) -> TcpMailbox {
        self.inner.mailbox()
    }

    /// Serve accepted connections until stopped or the listener fails.
    pub fn serve(&mut self) -> Result<(), WebSocketRuntimeError> {
        self.inner.serve().map_err(Into::into)
    }
}

fn validate_server_options(options: &WebSocketServerOptions) -> Result<(), WebSocketRuntimeError> {
    if options.max_frame_payload == 0 || options.max_message_payload == 0 {
        Err(WebSocketRuntimeError::InvalidOptions)
    } else {
        Ok(())
    }
}

enum HandshakeProgress {
    Pending,
    Accepted {
        response: Vec<u8>,
        leftover: Vec<u8>,
    },
    Rejected,
}

fn advance_server_handshake(buffer: &mut Vec<u8>, input: &[u8]) -> HandshakeProgress {
    if buffer.len().saturating_add(input.len()) > MAX_HANDSHAKE_BYTES {
        return HandshakeProgress::Rejected;
    }
    buffer.extend_from_slice(input);
    match accept_server_request(buffer) {
        Ok(handshake) => HandshakeProgress::Accepted {
            response: handshake.response().to_vec(),
            leftover: buffer[handshake.consumed()..].to_vec(),
        },
        Err(WebSocketError::IncompleteHandshake) => HandshakeProgress::Pending,
        Err(_) => HandshakeProgress::Rejected,
    }
}

fn server_input<S, F>(
    info: WebSocketConnectionInfo,
    state: &mut ServerConnectionState<S>,
    input: &[u8],
    max_frame_payload: usize,
    max_message_payload: usize,
    handler: &F,
) -> TcpHandlerResult
where
    F: Fn(WebSocketConnectionInfo, &mut S, MessageEvent) -> WebSocketHandlerResult,
{
    let progress = match &mut state.phase {
        ServerPhase::Handshake(buffer) => Some(advance_server_handshake(buffer, input)),
        ServerPhase::Open(session) => {
            let result = process_open_server(info, &mut state.application, session, input, handler);
            if result.close {
                state.phase = ServerPhase::Closing;
            }
            return result;
        }
        ServerPhase::Closing => return TcpHandlerResult::close(),
    };

    match progress.expect("handshake phase produces progress") {
        HandshakeProgress::Pending => TcpHandlerResult::default(),
        HandshakeProgress::Rejected => {
            state.phase = ServerPhase::Closing;
            TcpHandlerResult::write_and_close(BAD_REQUEST)
        }
        HandshakeProgress::Accepted {
            mut response,
            leftover,
        } => {
            state.phase = ServerPhase::Open(WebSocketSession::new(
                EndpointRole::Server,
                max_frame_payload,
                max_message_payload,
            ));
            if leftover.is_empty() {
                return TcpHandlerResult::write(response);
            }
            let result = match &mut state.phase {
                ServerPhase::Open(session) => {
                    process_open_server(info, &mut state.application, session, &leftover, handler)
                }
                _ => unreachable!("accepted handshake creates an open session"),
            };
            if result.close {
                state.phase = ServerPhase::Closing;
            }
            response.extend_from_slice(&result.write);
            TcpHandlerResult {
                write: response,
                close: result.close,
                defer_read: false,
            }
        }
    }
}

fn process_open_server<S, F>(
    info: WebSocketConnectionInfo,
    application: &mut S,
    session: &mut WebSocketSession,
    input: &[u8],
    handler: &F,
) -> TcpHandlerResult
where
    F: Fn(WebSocketConnectionInfo, &mut S, MessageEvent) -> WebSocketHandlerResult,
{
    let events = match session.push(input) {
        Ok(events) => events,
        Err(error) => return protocol_close(error),
    };
    let mut wire = Vec::new();
    let mut close = false;
    for event in events {
        let peer_close = matches!(event, MessageEvent::Close(_));
        if let Some(reply) = control_reply(&event) {
            append_server_frame(&mut wire, &reply);
        }
        let output = handler(info, application, event);
        if peer_close {
            close = true;
            continue;
        }
        for frame in output.frames {
            append_server_frame(&mut wire, &frame);
        }
        close |= output.close;
    }
    if close {
        TcpHandlerResult::write_and_close(wire)
    } else {
        TcpHandlerResult::write(wire)
    }
}

fn append_server_frame(output: &mut Vec<u8>, frame: &Frame) {
    let encoded = encode_frame(EndpointRole::Server, frame, None)
        .expect("validated server frame must encode without a mask");
    output.extend_from_slice(&encoded);
}

fn protocol_close(error: WebSocketError) -> TcpHandlerResult {
    let code = protocol_close_code(error);
    let frame = Frame::close(Some(code), "").expect("standard close code is valid");
    let wire =
        encode_frame(EndpointRole::Server, &frame, None).expect("server close frame must encode");
    TcpHandlerResult::write_and_close(wire)
}

/// Bounds and TCP timeouts for the blocking WebSocket client.
#[derive(Clone, Debug)]
pub struct WebSocketClientOptions {
    /// Underlying TCP connect, read, write, and buffering policy.
    pub tcp: ConnectOptions,
    /// Maximum bytes accepted in one inbound frame payload.
    pub max_frame_payload: usize,
    /// Maximum bytes accepted in one assembled inbound message.
    pub max_message_payload: usize,
    /// Maximum bytes requested from one TCP read.
    pub read_chunk_size: usize,
}

impl Default for WebSocketClientOptions {
    fn default() -> Self {
        Self {
            tcp: ConnectOptions::default(),
            max_frame_payload: 1024 * 1024,
            max_message_payload: 8 * 1024 * 1024,
            read_chunk_size: 16 * 1024,
        }
    }
}

/// Blocking `ws` client over the repository TCP client adapter.
pub struct WebSocketClient<E = OsEntropy> {
    connection: TcpConnection,
    entropy: E,
    session: WebSocketSession,
    pending: VecDeque<MessageEvent>,
    read_chunk_size: usize,
    close_sent: bool,
    close_received: bool,
}

impl WebSocketClient<OsEntropy> {
    /// Connect, perform the HTTP upgrade, and return an open blocking client.
    pub fn connect(
        host: &str,
        port: u16,
        target: &str,
        options: WebSocketClientOptions,
    ) -> Result<Self, WebSocketRuntimeError> {
        Self::connect_with_entropy(host, port, target, options, OsEntropy)
    }
}

impl<E: EntropySource> WebSocketClient<E> {
    /// Connect using an explicit entropy authority.
    ///
    /// This constructor exists so deterministic tests and constrained hosts can
    /// inject an auditable entropy provider. Production callers should use
    /// `WebSocketClient::connect`.
    pub fn connect_with_entropy(
        host: &str,
        port: u16,
        target: &str,
        options: WebSocketClientOptions,
        mut entropy: E,
    ) -> Result<Self, WebSocketRuntimeError> {
        validate_client_options(&options)?;
        let mut nonce = [0_u8; 16];
        entropy.fill(&mut nonce)?;
        let handshake = build_client_request(host, target, nonce)?;
        let mut connection = tcp_client::connect(host, port, options.tcp)?;
        connection.write_all(handshake.bytes())?;
        connection.flush()?;
        read_client_upgrade(&mut connection, handshake.expected_accept())?;
        Ok(Self {
            connection,
            entropy,
            session: WebSocketSession::new(
                EndpointRole::Client,
                options.max_frame_payload,
                options.max_message_payload,
            ),
            pending: VecDeque::new(),
            read_chunk_size: options.read_chunk_size,
            close_sent: false,
            close_received: false,
        })
    }

    /// Send one validated client-role frame with a fresh random mask key.
    pub fn send_frame(&mut self, frame: &Frame) -> Result<(), WebSocketRuntimeError> {
        if self.close_sent {
            return Err(WebSocketRuntimeError::ClosedSession);
        }
        self.write_frame(frame)
    }

    /// Send one complete UTF-8 text message.
    pub fn send_text(&mut self, text: impl Into<String>) -> Result<(), WebSocketRuntimeError> {
        self.send_frame(&Frame::text(text))
    }

    /// Send one complete binary message.
    pub fn send_binary(
        &mut self,
        payload: impl Into<Vec<u8>>,
    ) -> Result<(), WebSocketRuntimeError> {
        self.send_frame(&Frame::binary(payload))
    }

    /// Send one ping control frame.
    pub fn send_ping(&mut self, payload: impl Into<Vec<u8>>) -> Result<(), WebSocketRuntimeError> {
        let frame = Frame::ping(payload)?;
        self.send_frame(&frame)
    }

    /// Receive the next complete event, preserving coalesced later events.
    pub fn receive(&mut self) -> Result<MessageEvent, WebSocketRuntimeError> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }
        if self.close_received {
            return Err(WebSocketRuntimeError::ClosedSession);
        }
        loop {
            let bytes = self.connection.read_chunk(self.read_chunk_size)?;
            if bytes.is_empty() {
                return Err(WebSocketRuntimeError::AbnormalEof);
            }
            let events = match self.session.push(&bytes) {
                Ok(events) => events,
                Err(error) => {
                    self.send_protocol_close(error)?;
                    return Err(error.into());
                }
            };
            self.accept_client_events(events)?;
            if let Some(event) = self.pending.pop_front() {
                return Ok(event);
            }
        }
    }

    /// Begin a normal closing handshake with a validated status and reason.
    pub fn close(&mut self, code: Option<u16>, reason: &str) -> Result<(), WebSocketRuntimeError> {
        if self.close_sent {
            return Ok(());
        }
        let frame = Frame::close(code, reason)?;
        self.write_frame(&frame)?;
        self.close_sent = true;
        Ok(())
    }

    /// Whether a close frame has been sent or received.
    pub fn is_closing(&self) -> bool {
        self.close_sent || self.close_received
    }

    /// Return the connected peer TCP address.
    pub fn peer_addr(&self) -> Result<SocketAddr, WebSocketRuntimeError> {
        self.connection.peer_addr().map_err(Into::into)
    }

    /// Return the connected local TCP address.
    pub fn local_addr(&self) -> Result<SocketAddr, WebSocketRuntimeError> {
        self.connection.local_addr().map_err(Into::into)
    }

    fn write_frame(&mut self, frame: &Frame) -> Result<(), WebSocketRuntimeError> {
        let mut mask = [0_u8; 4];
        self.entropy.fill(&mut mask)?;
        let wire = encode_frame(EndpointRole::Client, frame, Some(mask))?;
        self.connection.write_all(&wire)?;
        self.connection.flush()?;
        Ok(())
    }

    fn accept_client_events(
        &mut self,
        events: Vec<MessageEvent>,
    ) -> Result<(), WebSocketRuntimeError> {
        for event in events {
            if let Some(reply) = control_reply(&event) {
                if !self.close_sent {
                    self.write_frame(&reply)?;
                }
            }
            if matches!(event, MessageEvent::Close(_)) {
                self.close_received = true;
                self.close_sent = true;
            }
            self.pending.push_back(event);
        }
        Ok(())
    }

    fn send_protocol_close(&mut self, error: WebSocketError) -> Result<(), WebSocketRuntimeError> {
        if !self.close_sent {
            let frame = Frame::close(Some(protocol_close_code(error)), "")?;
            self.write_frame(&frame)?;
            self.close_sent = true;
        }
        Ok(())
    }
}

fn validate_client_options(options: &WebSocketClientOptions) -> Result<(), WebSocketRuntimeError> {
    if options.max_frame_payload == 0
        || options.max_message_payload == 0
        || options.read_chunk_size == 0
    {
        Err(WebSocketRuntimeError::InvalidOptions)
    } else {
        Ok(())
    }
}

fn read_client_upgrade(
    connection: &mut TcpConnection,
    expected_accept: &str,
) -> Result<(), WebSocketRuntimeError> {
    let mut head = Vec::new();
    loop {
        let remaining = MAX_HANDSHAKE_BYTES.saturating_sub(head.len());
        if remaining == 0 {
            return Err(WebSocketError::HandshakeTooLarge.into());
        }
        let line = connection.read_until_limit(b'\n', remaining)?;
        if line.is_empty() {
            return Err(WebSocketRuntimeError::AbnormalEof);
        }
        head.extend_from_slice(&line);
        match validate_client_response(&head, expected_accept) {
            Ok(_) => return Ok(()),
            Err(WebSocketError::IncompleteHandshake) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

fn protocol_close_code(error: WebSocketError) -> u16 {
    match error {
        WebSocketError::InvalidUtf8 => 1007,
        WebSocketError::FrameTooLarge | WebSocketError::MessageTooLarge => 1009,
        _ => 1002,
    }
}

fn resolve_first<A: ToSocketAddrs>(address: A) -> Result<SocketAddr, WebSocketRuntimeError> {
    address
        .to_socket_addrs()
        .map_err(|_| WebSocketRuntimeError::InvalidAddress)?
        .next()
        .ok_or(WebSocketRuntimeError::InvalidAddress)
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
impl WebSocketRuntime<transport_platform::bsd::KqueueTransportPlatform, ()> {
    /// Bind a stateless server through the host kqueue transport.
    pub fn bind_kqueue<A, F>(
        address: A,
        options: WebSocketServerOptions,
        handler: F,
    ) -> Result<Self, WebSocketRuntimeError>
    where
        A: ToSocketAddrs,
        F: Fn(WebSocketConnectionInfo, MessageEvent) -> WebSocketHandlerResult
            + Send
            + Sync
            + 'static,
    {
        Self::bind_kqueue_with_state(
            address,
            options,
            |_| (),
            move |info, _, event| handler(info, event),
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
impl<S: Send + 'static> WebSocketRuntime<transport_platform::bsd::KqueueTransportPlatform, S> {
    /// Bind a stateful server through the host kqueue transport.
    pub fn bind_kqueue_with_state<A, I, F, C>(
        address: A,
        options: WebSocketServerOptions,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<Self, WebSocketRuntimeError>
    where
        A: ToSocketAddrs,
        I: Fn(WebSocketConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(WebSocketConnectionInfo, &mut S, MessageEvent) -> WebSocketHandlerResult
            + Send
            + Sync
            + 'static,
        C: Fn(WebSocketConnectionInfo, S) + Send + Sync + 'static,
    {
        let platform = transport_platform::bsd::KqueueTransportPlatform::new()?;
        Self::bind_with_state(
            platform,
            BindAddress::Ip(resolve_first(address)?),
            options,
            init,
            handler,
            on_close,
        )
    }
}

#[cfg(target_os = "linux")]
impl WebSocketRuntime<transport_platform::linux::EpollTransportPlatform, ()> {
    /// Bind a stateless server through the host epoll transport.
    pub fn bind_epoll<A, F>(
        address: A,
        options: WebSocketServerOptions,
        handler: F,
    ) -> Result<Self, WebSocketRuntimeError>
    where
        A: ToSocketAddrs,
        F: Fn(WebSocketConnectionInfo, MessageEvent) -> WebSocketHandlerResult
            + Send
            + Sync
            + 'static,
    {
        Self::bind_epoll_with_state(
            address,
            options,
            |_| (),
            move |info, _, event| handler(info, event),
            |_, _| {},
        )
    }
}

#[cfg(target_os = "linux")]
impl<S: Send + 'static> WebSocketRuntime<transport_platform::linux::EpollTransportPlatform, S> {
    /// Bind a stateful server through the host epoll transport.
    pub fn bind_epoll_with_state<A, I, F, C>(
        address: A,
        options: WebSocketServerOptions,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<Self, WebSocketRuntimeError>
    where
        A: ToSocketAddrs,
        I: Fn(WebSocketConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(WebSocketConnectionInfo, &mut S, MessageEvent) -> WebSocketHandlerResult
            + Send
            + Sync
            + 'static,
        C: Fn(WebSocketConnectionInfo, S) + Send + Sync + 'static,
    {
        let platform = transport_platform::linux::EpollTransportPlatform::new()?;
        Self::bind_with_state(
            platform,
            BindAddress::Ip(resolve_first(address)?),
            options,
            init,
            handler,
            on_close,
        )
    }
}

#[cfg(target_os = "windows")]
impl WebSocketRuntime<transport_platform::windows::WindowsTransportPlatform, ()> {
    /// Bind a stateless server through the host Windows IOCP transport.
    pub fn bind_windows<A, F>(
        address: A,
        options: WebSocketServerOptions,
        handler: F,
    ) -> Result<Self, WebSocketRuntimeError>
    where
        A: ToSocketAddrs,
        F: Fn(WebSocketConnectionInfo, MessageEvent) -> WebSocketHandlerResult
            + Send
            + Sync
            + 'static,
    {
        Self::bind_windows_with_state(
            address,
            options,
            |_| (),
            move |info, _, event| handler(info, event),
            |_, _| {},
        )
    }
}

#[cfg(target_os = "windows")]
impl<S: Send + 'static> WebSocketRuntime<transport_platform::windows::WindowsTransportPlatform, S> {
    /// Bind a stateful server through the host Windows IOCP transport.
    pub fn bind_windows_with_state<A, I, F, C>(
        address: A,
        options: WebSocketServerOptions,
        init: I,
        handler: F,
        on_close: C,
    ) -> Result<Self, WebSocketRuntimeError>
    where
        A: ToSocketAddrs,
        I: Fn(WebSocketConnectionInfo) -> S + Send + Sync + 'static,
        F: Fn(WebSocketConnectionInfo, &mut S, MessageEvent) -> WebSocketHandlerResult
            + Send
            + Sync
            + 'static,
        C: Fn(WebSocketConnectionInfo, S) + Send + Sync + 'static,
    {
        let platform = transport_platform::windows::WindowsTransportPlatform::new()?;
        Self::bind_with_state(
            platform,
            BindAddress::Ip(resolve_first(address)?),
            options,
            init,
            handler,
            on_close,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> WebSocketConnectionInfo {
        WebSocketConnectionInfo {
            id: ConnectionId(1),
            peer_addr: "127.0.0.1:1234".parse().unwrap(),
            local_addr: "127.0.0.1:4321".parse().unwrap(),
        }
    }

    fn state() -> ServerConnectionState<Vec<MessageEvent>> {
        ServerConnectionState {
            application: Vec::new(),
            phase: ServerPhase::Handshake(Vec::new()),
        }
    }

    fn request() -> Vec<u8> {
        build_client_request("localhost", "/chief", [7; 16])
            .unwrap()
            .bytes()
            .to_vec()
    }

    #[test]
    fn fragmented_upgrade_then_coalesced_frame_is_processed() {
        let mut state = state();
        let request = request();
        let split = request.len() - 3;
        let first = server_input(
            info(),
            &mut state,
            &request[..split],
            1024,
            1024,
            &|_, _, _| WebSocketHandlerResult::default(),
        );
        assert!(first.write.is_empty());

        let frame = encode_frame(
            EndpointRole::Client,
            &Frame::text("hello"),
            Some([1, 2, 3, 4]),
        )
        .unwrap();
        let mut suffix = request[split..].to_vec();
        suffix.extend_from_slice(&frame);
        let second = server_input(
            info(),
            &mut state,
            &suffix,
            1024,
            1024,
            &|_, events, event| {
                events.push(event);
                WebSocketHandlerResult::send(Frame::text("echo"))
            },
        );
        assert!(second.write.starts_with(b"HTTP/1.1 101"));
        assert_eq!(state.application, vec![MessageEvent::Text("hello".into())]);
        assert!(second.write.ends_with(&[0x81, 4, b'e', b'c', b'h', b'o']));
    }

    #[test]
    fn invalid_and_oversized_upgrades_are_bounded_and_closed() {
        for input in [
            b"GET / HTTP/1.0\r\n\r\n".as_slice(),
            &vec![b'x'; MAX_HANDSHAKE_BYTES + 1],
        ] {
            let mut state = state();
            let result = server_input(info(), &mut state, input, 1024, 1024, &|_, _, _| {
                WebSocketHandlerResult::default()
            });
            assert!(result.close);
            assert_eq!(result.write, BAD_REQUEST);
        }
    }

    #[test]
    fn ping_and_close_receive_automatic_replies() {
        let mut state = state();
        let request = request();
        let accepted = server_input(info(), &mut state, &request, 1024, 1024, &|_, _, _| {
            WebSocketHandlerResult::default()
        });
        assert!(accepted.write.starts_with(b"HTTP/1.1 101"));

        let ping = encode_frame(
            EndpointRole::Client,
            &Frame::ping(b"ok".to_vec()).unwrap(),
            Some([1; 4]),
        )
        .unwrap();
        let pong = server_input(
            info(),
            &mut state,
            &ping,
            1024,
            1024,
            &|_, events, event| {
                events.push(event);
                WebSocketHandlerResult::default()
            },
        );
        assert_eq!(pong.write, vec![0x8a, 2, b'o', b'k']);

        let close = encode_frame(
            EndpointRole::Client,
            &Frame::close(Some(1000), "bye").unwrap(),
            Some([2; 4]),
        )
        .unwrap();
        let reply = server_input(
            info(),
            &mut state,
            &close,
            1024,
            1024,
            &|_, events, event| {
                events.push(event);
                WebSocketHandlerResult::send(Frame::text("must-not-follow-close"))
            },
        );
        assert!(reply.close);
        assert_eq!(reply.write[0], 0x88);
        assert_eq!(reply.write.len(), 7);
        let after_close = server_input(info(), &mut state, b"ignored", 1024, 1024, &|_, _, _| {
            WebSocketHandlerResult::default()
        });
        assert!(after_close.close);
        assert!(after_close.write.is_empty());
    }

    #[test]
    fn protocol_error_codes_are_specific() {
        for (error, code) in [
            (WebSocketError::InvalidUtf8, 1007_u16),
            (WebSocketError::FrameTooLarge, 1009),
            (WebSocketError::InvalidOpcode, 1002),
        ] {
            let result = protocol_close(error);
            assert!(result.close);
            assert_eq!(&result.write[2..4], &code.to_be_bytes());
        }
    }

    #[test]
    fn options_and_error_displays_are_payload_free() {
        let invalid_server = WebSocketServerOptions {
            max_frame_payload: 0,
            ..WebSocketServerOptions::default()
        };
        assert!(matches!(
            validate_server_options(&invalid_server),
            Err(WebSocketRuntimeError::InvalidOptions)
        ));
        let invalid_client = WebSocketClientOptions {
            read_chunk_size: 0,
            ..WebSocketClientOptions::default()
        };
        assert!(matches!(
            validate_client_options(&invalid_client),
            Err(WebSocketRuntimeError::InvalidOptions)
        ));
        assert_eq!(
            WebSocketHandlerResult::send_many(vec![Frame::text("a"), Frame::binary(vec![1])])
                .frames
                .len(),
            2
        );
        assert!(
            WebSocketHandlerResult::close(Some(1000), "done")
                .unwrap()
                .close
        );
        let error = WebSocketRuntimeError::Protocol(WebSocketError::InvalidHandshake);
        assert_eq!(error.to_string(), "websocket runtime: protocol failure");
    }
}
