//! # udp-client
//!
//! `udp-client` is a tiny, transport-layer wrapper around `std::net::UdpSocket`.
//! It sends and receives opaque datagrams; it does not parse DNS, resolve
//! hostnames, retry lost packets, or pretend UDP is reliable.
//!
//! UDP is a postcard, not a phone call. Each call to `send_to` drops one
//! self-contained message onto the network. Each call to `recv_from` picks up
//! one self-contained message plus the address of whoever sent it.

use std::fmt;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};
use std::time::Duration;

pub const VERSION: &str = "0.1.0";
pub const DEFAULT_MAX_DATAGRAM_SIZE: usize = 65_535;

/// Configuration for a UDP socket.
///
/// The payload remains opaque. A DNS resolver, game protocol, or test fixture
/// can all share the same transport because this layer only understands socket
/// addresses, timeouts, and byte limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpOptions {
    /// Optional local address. `None` binds an ephemeral IPv4 socket for
    /// `UdpClient::bind`; `send_and_receive` chooses IPv4 or IPv6 from the
    /// destination when no bind address is supplied.
    pub bind_addr: Option<SocketAddr>,
    /// Maximum caller-visible payload bytes for one receive or send.
    pub max_datagram_size: usize,
    /// Timeout for receive operations. `None` means use the platform default.
    pub read_timeout: Option<Duration>,
    /// Timeout for send operations where the platform supports it.
    pub write_timeout: Option<Duration>,
}

impl Default for UdpOptions {
    fn default() -> Self {
        Self {
            bind_addr: None,
            max_datagram_size: DEFAULT_MAX_DATAGRAM_SIZE,
            read_timeout: None,
            write_timeout: None,
        }
    }
}

/// One received UDP datagram plus its endpoint metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpDatagram {
    pub source: SocketAddr,
    pub destination: SocketAddr,
    pub payload: Vec<u8>,
}

/// Errors from binding, sending, and receiving UDP datagrams.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UdpError {
    BindFailed(String),
    ConnectFailed(String),
    SendFailed(String),
    ReceiveFailed(String),
    Timeout,
    NotConnected,
    InvalidDatagramSize { size: usize, max: usize },
    TruncatedDatagram,
}

impl fmt::Display for UdpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BindFailed(message) => write!(f, "failed to bind UDP socket: {message}"),
            Self::ConnectFailed(message) => write!(f, "failed to connect UDP socket: {message}"),
            Self::SendFailed(message) => write!(f, "failed to send UDP datagram: {message}"),
            Self::ReceiveFailed(message) => write!(f, "failed to receive UDP datagram: {message}"),
            Self::Timeout => write!(f, "UDP operation timed out"),
            Self::NotConnected => write!(f, "UDP socket has no connected peer"),
            Self::InvalidDatagramSize { size, max } => {
                write!(f, "invalid UDP datagram size {size}; maximum is {max}")
            }
            Self::TruncatedDatagram => write!(f, "UDP datagram exceeded receive buffer"),
        }
    }
}

impl std::error::Error for UdpError {}

/// A UDP socket with deterministic size checks and structured errors.
#[derive(Debug)]
pub struct UdpClient {
    socket: UdpSocket,
    options: UdpOptions,
    connected_peer: Option<SocketAddr>,
}

impl UdpClient {
    /// Open a UDP socket and apply the configured timeouts.
    pub fn bind(options: UdpOptions) -> Result<Self, UdpError> {
        validate_max_datagram_size(options.max_datagram_size)?;
        let bind_addr = options
            .bind_addr
            .unwrap_or_else(default_unspecified_ipv4_addr);
        let socket =
            UdpSocket::bind(bind_addr).map_err(|err| UdpError::BindFailed(err.to_string()))?;
        socket
            .set_read_timeout(options.read_timeout)
            .map_err(|err| UdpError::BindFailed(err.to_string()))?;
        socket
            .set_write_timeout(options.write_timeout)
            .map_err(|err| UdpError::BindFailed(err.to_string()))?;

        Ok(Self {
            socket,
            options,
            connected_peer: None,
        })
    }

    /// Record a default peer for connected-UDP mode.
    pub fn connect(&mut self, remote: SocketAddr) -> Result<(), UdpError> {
        self.socket
            .connect(remote)
            .map_err(|err| UdpError::ConnectFailed(err.to_string()))?;
        self.connected_peer = Some(remote);
        Ok(())
    }

    /// Send exactly one datagram to an explicit destination.
    pub fn send_to(&self, payload: &[u8], destination: SocketAddr) -> Result<usize, UdpError> {
        self.validate_payload_len(payload)?;
        self.socket
            .send_to(payload, destination)
            .map_err(map_send_error)
    }

    /// Send exactly one datagram to the connected peer.
    pub fn send(&self, payload: &[u8]) -> Result<usize, UdpError> {
        if self.connected_peer.is_none() {
            return Err(UdpError::NotConnected);
        }
        self.validate_payload_len(payload)?;
        self.socket.send(payload).map_err(map_send_error)
    }

    /// Receive exactly one datagram and return source/destination metadata.
    pub fn recv_from(&self) -> Result<UdpDatagram, UdpError> {
        let mut buffer = vec![0u8; receive_buffer_len(self.options.max_datagram_size)];
        let (received, source) = self
            .socket
            .recv_from(&mut buffer)
            .map_err(map_receive_error)?;
        if received > self.options.max_datagram_size {
            return Err(UdpError::TruncatedDatagram);
        }
        buffer.truncate(received);

        Ok(UdpDatagram {
            source,
            destination: self.local_addr()?,
            payload: buffer,
        })
    }

    /// Return the local socket address assigned by the OS.
    pub fn local_addr(&self) -> Result<SocketAddr, UdpError> {
        self.socket
            .local_addr()
            .map_err(|err| UdpError::ReceiveFailed(err.to_string()))
    }

    fn validate_payload_len(&self, payload: &[u8]) -> Result<(), UdpError> {
        if payload.len() > self.options.max_datagram_size {
            Err(UdpError::InvalidDatagramSize {
                size: payload.len(),
                max: self.options.max_datagram_size,
            })
        } else {
            Ok(())
        }
    }
}

/// Send one datagram to a peer and wait for one response from that same peer.
pub fn send_and_receive(
    destination: SocketAddr,
    payload: &[u8],
    mut options: UdpOptions,
) -> Result<UdpDatagram, UdpError> {
    if options.bind_addr.is_none() {
        options.bind_addr = Some(unspecified_addr_for(destination));
    }

    let mut client = UdpClient::bind(options)?;
    client.connect(destination)?;
    client.send(payload)?;
    client.recv_from()
}

fn validate_max_datagram_size(size: usize) -> Result<(), UdpError> {
    if size == 0 || size > DEFAULT_MAX_DATAGRAM_SIZE {
        Err(UdpError::InvalidDatagramSize {
            size,
            max: DEFAULT_MAX_DATAGRAM_SIZE,
        })
    } else {
        Ok(())
    }
}

fn receive_buffer_len(max_datagram_size: usize) -> usize {
    max_datagram_size
        .checked_add(1)
        .filter(|size| *size <= DEFAULT_MAX_DATAGRAM_SIZE)
        .unwrap_or(max_datagram_size)
}

fn default_unspecified_ipv4_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)
}

fn unspecified_addr_for(destination: SocketAddr) -> SocketAddr {
    match destination {
        SocketAddr::V4(_) => default_unspecified_ipv4_addr(),
        SocketAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    }
}

fn map_send_error(err: io::Error) -> UdpError {
    match err.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => UdpError::Timeout,
        _ => UdpError::SendFailed(err.to_string()),
    }
}

fn map_receive_error(err: io::Error) -> UdpError {
    match err.kind() {
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => UdpError::Timeout,
        _ => UdpError::ReceiveFailed(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn localhost(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    }

    fn test_options() -> UdpOptions {
        UdpOptions {
            bind_addr: Some(localhost(0)),
            max_datagram_size: 1024,
            read_timeout: Some(Duration::from_millis(500)),
            write_timeout: Some(Duration::from_millis(500)),
        }
    }

    fn bind_test_client() -> UdpClient {
        UdpClient::bind(test_options()).unwrap()
    }

    #[test]
    fn default_options_are_safe_and_transport_only() {
        let options = UdpOptions::default();

        assert_eq!(options.bind_addr, None);
        assert_eq!(options.max_datagram_size, DEFAULT_MAX_DATAGRAM_SIZE);
        assert_eq!(options.read_timeout, None);
        assert_eq!(options.write_timeout, None);
    }

    #[test]
    fn error_display_messages_name_the_failure_mode() {
        let cases = [
            (
                UdpError::BindFailed("addr in use".to_string()),
                "failed to bind UDP socket: addr in use",
            ),
            (
                UdpError::ConnectFailed("no route".to_string()),
                "failed to connect UDP socket: no route",
            ),
            (
                UdpError::SendFailed("closed".to_string()),
                "failed to send UDP datagram: closed",
            ),
            (
                UdpError::ReceiveFailed("closed".to_string()),
                "failed to receive UDP datagram: closed",
            ),
            (UdpError::Timeout, "UDP operation timed out"),
            (UdpError::NotConnected, "UDP socket has no connected peer"),
            (
                UdpError::InvalidDatagramSize { size: 9, max: 8 },
                "invalid UDP datagram size 9; maximum is 8",
            ),
            (
                UdpError::TruncatedDatagram,
                "UDP datagram exceeded receive buffer",
            ),
        ];

        for (error, message) in cases {
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn default_bind_uses_ephemeral_ipv4_socket() {
        let client = UdpClient::bind(UdpOptions {
            read_timeout: Some(Duration::from_millis(500)),
            write_timeout: Some(Duration::from_millis(500)),
            ..UdpOptions::default()
        })
        .unwrap();
        let local = client.local_addr().unwrap();

        assert!(local.is_ipv4());
        assert_ne!(local.port(), 0);
    }

    #[test]
    fn rejects_invalid_receive_buffer_sizes() {
        for size in [0, DEFAULT_MAX_DATAGRAM_SIZE + 1] {
            let result = UdpClient::bind(UdpOptions {
                max_datagram_size: size,
                ..test_options()
            });

            assert!(matches!(
                result,
                Err(UdpError::InvalidDatagramSize {
                    size: actual,
                    max: DEFAULT_MAX_DATAGRAM_SIZE
                }) if actual == size
            ));
        }
    }

    #[test]
    fn bind_localhost_assigns_ephemeral_port() {
        let client = bind_test_client();
        let local = client.local_addr().unwrap();

        assert_eq!(local.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_ne!(local.port(), 0);
    }

    #[test]
    fn send_without_connect_reports_not_connected() {
        let client = bind_test_client();

        assert_eq!(client.send(b"hello"), Err(UdpError::NotConnected));
    }

    #[test]
    fn rejects_oversized_send_before_calling_os() {
        let client = UdpClient::bind(UdpOptions {
            max_datagram_size: 2,
            ..test_options()
        })
        .unwrap();

        assert_eq!(
            client.send_to(b"abc", localhost(9)),
            Err(UdpError::InvalidDatagramSize { size: 3, max: 2 })
        );
    }

    #[test]
    fn send_to_and_recv_from_preserve_payload_and_source() {
        let server = bind_test_client();
        let client = bind_test_client();
        let client_addr = client.local_addr().unwrap();
        let server_addr = server.local_addr().unwrap();

        assert_eq!(client.send_to(b"abc", server_addr).unwrap(), 3);
        let datagram = server.recv_from().unwrap();

        assert_eq!(datagram.payload, b"abc");
        assert_eq!(datagram.source, client_addr);
        assert_eq!(datagram.destination, server_addr);
    }

    #[test]
    fn echo_round_trip_uses_one_datagram_each_way() {
        let server = bind_test_client();
        let server_addr = server.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let datagram = server.recv_from().unwrap();
            server
                .send_to(&datagram.payload, datagram.source)
                .expect("server echo send");
        });

        let client = bind_test_client();
        client.send_to(b"ping", server_addr).unwrap();
        let response = client.recv_from().unwrap();
        handle.join().unwrap();

        assert_eq!(response.source, server_addr);
        assert_eq!(response.payload, b"ping");
    }

    #[test]
    fn connected_udp_send_uses_recorded_peer() {
        let server = bind_test_client();
        let server_addr = server.local_addr().unwrap();
        let mut client = bind_test_client();

        client.connect(server_addr).unwrap();
        assert_eq!(client.send(b"connected").unwrap(), 9);

        let datagram = server.recv_from().unwrap();
        assert_eq!(datagram.payload, b"connected");
        assert_eq!(datagram.source, client.local_addr().unwrap());
    }

    #[test]
    fn receive_timeout_maps_to_timeout_error() {
        let idle = UdpClient::bind(UdpOptions {
            read_timeout: Some(Duration::from_millis(20)),
            ..test_options()
        })
        .unwrap();

        assert_eq!(idle.recv_from(), Err(UdpError::Timeout));
    }

    #[test]
    fn empty_datagrams_round_trip_on_loopback() {
        let server = bind_test_client();
        let client = bind_test_client();

        client.send_to(b"", server.local_addr().unwrap()).unwrap();
        let datagram = server.recv_from().unwrap();

        assert!(datagram.payload.is_empty());
    }

    #[test]
    fn send_and_receive_binds_for_destination_family() {
        let server = bind_test_client();
        let server_addr = server.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let datagram = server.recv_from().unwrap();
            server.send_to(b"pong", datagram.source).unwrap();
        });

        let response = send_and_receive(
            server_addr,
            b"ping",
            UdpOptions {
                max_datagram_size: 1024,
                read_timeout: Some(Duration::from_millis(500)),
                write_timeout: Some(Duration::from_millis(500)),
                ..UdpOptions::default()
            },
        )
        .unwrap();
        handle.join().unwrap();

        assert_eq!(response.payload, b"pong");
        assert_eq!(response.source, server_addr);
    }

    #[test]
    fn datagram_larger_than_receive_limit_reports_truncation() {
        let receiver = UdpClient::bind(UdpOptions {
            max_datagram_size: 3,
            ..test_options()
        })
        .unwrap();
        let sender = bind_test_client();

        sender
            .send_to(b"four", receiver.local_addr().unwrap())
            .unwrap();

        assert_eq!(receiver.recv_from(), Err(UdpError::TruncatedDatagram));
    }

    #[test]
    fn ipv6_loopback_works_when_available() {
        let options = UdpOptions {
            bind_addr: Some(SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 0)),
            ..test_options()
        };
        let server = match UdpClient::bind(options.clone()) {
            Ok(server) => server,
            Err(_) => return,
        };
        let client = match UdpClient::bind(options) {
            Ok(client) => client,
            Err(_) => return,
        };

        client.send_to(b"v6", server.local_addr().unwrap()).unwrap();
        let datagram = server.recv_from().unwrap();

        assert_eq!(datagram.payload, b"v6");
        assert!(datagram.source.is_ipv6());
    }

    #[test]
    fn multiple_ephemeral_sockets_do_not_collide() {
        let first = bind_test_client().local_addr().unwrap();
        let second = bind_test_client().local_addr().unwrap();
        let third = bind_test_client().local_addr().unwrap();

        assert_ne!(first, second);
        assert_ne!(second, third);
        assert_ne!(first, third);
    }

    #[test]
    fn internal_helpers_map_timeouts_and_buffer_caps() {
        assert_eq!(receive_buffer_len(3), 4);
        assert_eq!(
            receive_buffer_len(DEFAULT_MAX_DATAGRAM_SIZE),
            DEFAULT_MAX_DATAGRAM_SIZE
        );
        assert_eq!(
            unspecified_addr_for(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 7)),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)
        );
        assert_eq!(
            unspecified_addr_for(SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 7)),
            SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0)
        );
        assert_eq!(
            map_send_error(io::Error::new(io::ErrorKind::TimedOut, "slow")),
            UdpError::Timeout
        );
        assert_eq!(
            map_send_error(io::Error::new(io::ErrorKind::Other, "boom")),
            UdpError::SendFailed("boom".to_string())
        );
        assert_eq!(
            map_receive_error(io::Error::new(io::ErrorKind::WouldBlock, "wait")),
            UdpError::Timeout
        );
        assert_eq!(
            map_receive_error(io::Error::new(io::ErrorKind::Other, "boom")),
            UdpError::ReceiveFailed("boom".to_string())
        );
    }
}
