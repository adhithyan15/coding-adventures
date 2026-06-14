//! # tls-platform
//!
//! TLS client substrate for secure outbound transports.
//!
//! The durable API is the `TlsConnector` / `TlsStream` trait pair from the
//! TLS platform spec. The first concrete backend in this crate is
//! `RustlsConnector`, which delegates TLS cryptography and certificate
//! verification to `rustls`.
//!
//! ```rust,no_run
//! use std::io::Write;
//! use tls_platform::{RustlsConnector, TlsConfig, TlsConnector};
//!
//! let connector = RustlsConnector::default();
//! let mut stream = connector.connect("api.weather.gov", 443, &TlsConfig::https_default())?;
//! stream.write_all(b"HEAD / HTTP/1.1\r\nHost: api.weather.gov\r\n\r\n")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{
    version, ClientConfig, ClientConnection, ProtocolVersion, RootCertStore, StreamOwned,
    SupportedProtocolVersion,
};
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

pub const VERSION: &str = "0.1.0";

/// Open TLS-encrypted streams for higher transport crates.
pub trait TlsConnector: Send + Sync {
    fn connect(
        &self,
        host: &str,
        port: u16,
        config: &TlsConfig,
    ) -> Result<Box<dyn TlsStream>, TlsError>;
}

/// A TLS-encrypted bidirectional byte stream.
pub trait TlsStream: Read + Write + Send {
    fn peer_certificates(&self) -> Result<Vec<Vec<u8>>, TlsError>;
    fn negotiated_alpn(&self) -> Option<String>;
    fn negotiated_version(&self) -> TlsVersion;
    fn close_notify(&mut self) -> Result<(), TlsError>;
    fn summary(&self) -> TlsConnectionSummary;
}

/// Return the default connector for this build.
///
/// The repository spec ultimately wants per-OS backends underneath this facade
/// (Schannel, Network.framework, OpenSSL). Until those land, the facade uses
/// the native Rustls backend.
pub fn default_connector() -> Box<dyn TlsConnector> {
    Box::new(RustlsConnector::default())
}

/// TLS configuration shared by client transports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsConfig {
    pub min_version: TlsVersion,
    pub max_version: TlsVersion,
    pub alpn_protocols: Vec<String>,
    pub root_store: RootStore,
    pub server_name: Option<String>,
    pub verify_mode: VerifyMode,
    pub connect_timeout: Duration,
    pub read_timeout: Option<Duration>,
    pub write_timeout: Option<Duration>,
    pub handshake_timeout: Duration,
}

impl TlsConfig {
    pub fn https_default() -> Self {
        Self {
            alpn_protocols: vec!["http/1.1".to_string()],
            ..Self::default()
        }
    }
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
            alpn_protocols: vec![],
            root_store: RootStore::Bundled,
            server_name: None,
            verify_mode: VerifyMode::Strict,
            connect_timeout: Duration::from_secs(30),
            read_timeout: Some(Duration::from_secs(30)),
            write_timeout: Some(Duration::from_secs(30)),
            handshake_timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TlsVersion {
    Tls12,
    Tls13,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootStore {
    /// The target platform trust store. The current Rustls backend resolves
    /// this to the bundled WebPKI roots; future OS backends should honor the
    /// true system store.
    SystemDefault,
    /// The bundled WebPKI root set.
    Bundled,
    /// Trust only the provided DER-encoded roots.
    Custom(Vec<Vec<u8>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyMode {
    Strict,
    NoHostname,
}

/// A validated TLS endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsEndpoint {
    server_name: String,
    port: u16,
}

impl TlsEndpoint {
    /// Construct an endpoint and validate that the server name is usable for
    /// SNI and certificate verification.
    pub fn new(server_name: impl Into<String>, port: u16) -> Result<Self, TlsError> {
        let server_name = server_name.into();
        validate_server_name(&server_name)?;

        if port == 0 {
            return Err(TlsError::InvalidPort { port });
        }

        Ok(Self { server_name, port })
    }

    /// Construct a default HTTPS endpoint on port 443.
    pub fn https(server_name: impl Into<String>) -> Result<Self, TlsError> {
        Self::new(server_name, 443)
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn summary(&self, config: &TlsConfig) -> TlsEndpointSummary {
        TlsEndpointSummary {
            server_name_len: self.server_name.len(),
            port: self.port,
            uses_default_https_port: self.port == 443,
            sni_enabled: true,
            alpn_protocol_count: config.alpn_protocols.len(),
            min_version: config.min_version,
            max_version: config.max_version,
        }
    }
}

/// Redacted endpoint facts for logs and supervision status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsEndpointSummary {
    pub server_name_len: usize,
    pub port: u16,
    pub uses_default_https_port: bool,
    pub sni_enabled: bool,
    pub alpn_protocol_count: usize,
    pub min_version: TlsVersion,
    pub max_version: TlsVersion,
}

/// Redacted facts captured after a completed TLS handshake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsConnectionSummary {
    pub server_name_len: usize,
    pub port: u16,
    pub protocol_version: TlsVersion,
    pub negotiated_alpn: Option<String>,
    pub handshake_complete: bool,
}

impl TlsConnectionSummary {
    fn from_connection(endpoint: &TlsEndpoint, connection: &ClientConnection) -> Self {
        Self {
            server_name_len: endpoint.server_name.len(),
            port: endpoint.port,
            protocol_version: tls_version_from_rustls(connection.protocol_version()),
            negotiated_alpn: connection
                .alpn_protocol()
                .map(|protocol| String::from_utf8_lossy(protocol).into_owned()),
            handshake_complete: !connection.is_handshaking(),
        }
    }
}

/// Rustls-backed TLS connector.
#[derive(Debug, Clone, Default)]
pub struct RustlsConnector;

impl RustlsConnector {
    pub fn connect_endpoint(
        &self,
        endpoint: TlsEndpoint,
        config: &TlsConfig,
    ) -> Result<RustlsTlsStream, TlsError> {
        let tcp_stream = connect_tcp(&endpoint.server_name, endpoint.port, config)?;
        self.connect_tcp_stream(endpoint, tcp_stream, config)
    }

    pub fn connect_tcp_stream(
        &self,
        endpoint: TlsEndpoint,
        mut stream: TcpStream,
        config: &TlsConfig,
    ) -> Result<RustlsTlsStream, TlsError> {
        apply_timeouts(
            &stream,
            Some(config.handshake_timeout),
            Some(config.handshake_timeout),
        )?;

        let rustls_config = rustls_config(config)?;
        let sni = config
            .server_name
            .as_deref()
            .unwrap_or(&endpoint.server_name);
        let server_name = server_name_for_rustls(sni)?;
        let mut connection =
            ClientConnection::new(Arc::new(rustls_config), server_name).map_err(|source| {
                TlsError::HandshakeFailed {
                    message: source.to_string(),
                    alert: None,
                }
            })?;

        while connection.is_handshaking() {
            let (read, written) = connection
                .complete_io(&mut stream)
                .map_err(|source| map_handshake_io(source, config))?;
            if read == 0 && written == 0 && connection.is_handshaking() {
                return Err(TlsError::HandshakeFailed {
                    message: "handshake stalled without I/O progress".to_string(),
                    alert: None,
                });
            }
        }

        apply_timeouts(&stream, config.read_timeout, config.write_timeout)?;

        let summary = TlsConnectionSummary::from_connection(&endpoint, &connection);
        Ok(RustlsTlsStream {
            stream: StreamOwned::new(connection, stream),
            summary,
        })
    }
}

impl TlsConnector for RustlsConnector {
    fn connect(
        &self,
        host: &str,
        port: u16,
        config: &TlsConfig,
    ) -> Result<Box<dyn TlsStream>, TlsError> {
        let endpoint = TlsEndpoint::new(host, port)?;
        Ok(Box::new(self.connect_endpoint(endpoint, config)?))
    }
}

/// A Rustls-protected byte stream.
pub struct RustlsTlsStream {
    stream: StreamOwned<ClientConnection, TcpStream>,
    summary: TlsConnectionSummary,
}

impl TlsStream for RustlsTlsStream {
    fn peer_certificates(&self) -> Result<Vec<Vec<u8>>, TlsError> {
        self.stream
            .conn
            .peer_certificates()
            .map(|certificates| {
                certificates
                    .iter()
                    .map(|certificate| certificate.as_ref().to_vec())
                    .collect()
            })
            .ok_or(TlsError::PeerCertificatesUnavailable)
    }

    fn negotiated_alpn(&self) -> Option<String> {
        self.stream
            .conn
            .alpn_protocol()
            .map(|protocol| String::from_utf8_lossy(protocol).into_owned())
    }

    fn negotiated_version(&self) -> TlsVersion {
        tls_version_from_rustls(self.stream.conn.protocol_version())
    }

    fn close_notify(&mut self) -> Result<(), TlsError> {
        self.stream.conn.send_close_notify();
        self.stream.flush().map_err(|source| TlsError::Io {
            phase: "close-notify",
            source,
        })
    }

    fn summary(&self) -> TlsConnectionSummary {
        self.summary.clone()
    }
}

impl Read for RustlsTlsStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for RustlsTlsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

/// Count of bundled WebPKI roots. Exposed for deterministic validation and
/// runtime health checks without leaking certificate material.
pub fn bundled_root_count() -> usize {
    webpki_roots::TLS_SERVER_ROOTS.len()
}

#[derive(Debug)]
pub enum TlsError {
    InvalidServerName {
        server_name: String,
    },
    InvalidPort {
        port: u16,
    },
    UnsupportedConfig {
        detail: String,
    },
    InvalidRootCertificate {
        index: usize,
        message: String,
    },
    DnsResolutionFailed {
        host: String,
        message: String,
    },
    TcpConnect {
        host: String,
        port: u16,
        source: io::Error,
    },
    HandshakeFailed {
        message: String,
        alert: Option<u8>,
    },
    CertVerifyFailed {
        message: String,
        chain_summary: String,
    },
    HostnameMismatch {
        requested: String,
        cert_names: Vec<String>,
    },
    Timeout {
        phase: &'static str,
        elapsed_ms: u64,
    },
    ClosedUnexpectedly,
    CapabilityDenied {
        detail: String,
    },
    Backend {
        code: i64,
        message: String,
    },
    Io {
        phase: &'static str,
        source: io::Error,
    },
    PeerCertificatesUnavailable,
}

impl fmt::Display for TlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TlsError::InvalidServerName { server_name } => {
                write!(f, "invalid TLS server name: {server_name:?}")
            }
            TlsError::InvalidPort { port } => write!(f, "invalid TLS port: {port}"),
            TlsError::UnsupportedConfig { detail } => {
                write!(f, "unsupported TLS config: {detail}")
            }
            TlsError::InvalidRootCertificate { index, message } => {
                write!(f, "invalid root certificate at index {index}: {message}")
            }
            TlsError::DnsResolutionFailed { host, message } => {
                write!(f, "DNS resolution failed for {host:?}: {message}")
            }
            TlsError::TcpConnect { host, port, source } => {
                write!(f, "TCP connect failed for {host:?}:{port}: {source}")
            }
            TlsError::HandshakeFailed { message, alert } => {
                write!(f, "TLS handshake failed: {message}")?;
                if let Some(alert) = alert {
                    write!(f, " (alert {alert})")?;
                }
                Ok(())
            }
            TlsError::CertVerifyFailed {
                message,
                chain_summary,
            } => write!(
                f,
                "certificate verification failed: {message}; {chain_summary}"
            ),
            TlsError::HostnameMismatch {
                requested,
                cert_names,
            } => write!(
                f,
                "hostname mismatch for {requested:?}; certificate names: {cert_names:?}"
            ),
            TlsError::Timeout { phase, elapsed_ms } => {
                write!(f, "{phase} timed out after {elapsed_ms} ms")
            }
            TlsError::ClosedUnexpectedly => write!(f, "TLS stream closed unexpectedly"),
            TlsError::CapabilityDenied { detail } => {
                write!(f, "capability denied TLS connect: {detail}")
            }
            TlsError::Backend { code, message } => {
                write!(f, "TLS backend error {code}: {message}")
            }
            TlsError::Io { phase, source } => write!(f, "{phase} I/O error: {source}"),
            TlsError::PeerCertificatesUnavailable => {
                write!(f, "peer certificates are unavailable")
            }
        }
    }
}

impl std::error::Error for TlsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TlsError::TcpConnect { source, .. } => Some(source),
            TlsError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn rustls_config(config: &TlsConfig) -> Result<ClientConfig, TlsError> {
    if config.verify_mode != VerifyMode::Strict {
        return Err(TlsError::UnsupportedConfig {
            detail: "the Rustls backend currently supports strict verification only".to_string(),
        });
    }

    let versions = protocol_versions(config)?;
    let root_store = root_store(config)?;
    let mut rustls_config = ClientConfig::builder_with_protocol_versions(&versions)
        .with_root_certificates(root_store)
        .with_no_client_auth();
    rustls_config.alpn_protocols = config
        .alpn_protocols
        .iter()
        .map(|protocol| protocol.as_bytes().to_vec())
        .collect();

    Ok(rustls_config)
}

fn root_store(config: &TlsConfig) -> Result<RootCertStore, TlsError> {
    let mut root_store = RootCertStore::empty();

    match &config.root_store {
        RootStore::SystemDefault | RootStore::Bundled => {
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        }
        RootStore::Custom(certificates) => {
            for (index, certificate) in certificates.iter().enumerate() {
                root_store
                    .add(CertificateDer::from(certificate.clone()))
                    .map_err(|source| TlsError::InvalidRootCertificate {
                        index,
                        message: source.to_string(),
                    })?;
            }
        }
    }

    Ok(root_store)
}

fn protocol_versions(
    config: &TlsConfig,
) -> Result<Vec<&'static SupportedProtocolVersion>, TlsError> {
    if config.min_version > config.max_version {
        return Err(TlsError::UnsupportedConfig {
            detail: "min_version must be <= max_version".to_string(),
        });
    }

    match (config.min_version, config.max_version) {
        (TlsVersion::Tls12, TlsVersion::Tls12) => Ok(vec![&version::TLS12]),
        (TlsVersion::Tls13, TlsVersion::Tls13) => Ok(vec![&version::TLS13]),
        (TlsVersion::Tls12, TlsVersion::Tls13) => Ok(vec![&version::TLS13, &version::TLS12]),
        (TlsVersion::Unknown, _) | (_, TlsVersion::Unknown) => Err(TlsError::UnsupportedConfig {
            detail: "Unknown is a negotiated-version sentinel, not a config value".to_string(),
        }),
        _ => Err(TlsError::UnsupportedConfig {
            detail: "unsupported TLS version range".to_string(),
        }),
    }
}

fn connect_tcp(host: &str, port: u16, config: &TlsConfig) -> Result<TcpStream, TlsError> {
    let addresses: Vec<_> = (host, port)
        .to_socket_addrs()
        .map_err(|source| TlsError::DnsResolutionFailed {
            host: host.to_string(),
            message: source.to_string(),
        })?
        .collect();

    if addresses.is_empty() {
        return Err(TlsError::DnsResolutionFailed {
            host: host.to_string(),
            message: "no socket addresses returned".to_string(),
        });
    }

    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, config.connect_timeout) {
            Ok(stream) => return Ok(stream),
            Err(source) => last_error = Some(source),
        }
    }

    let source = last_error.unwrap_or_else(|| io::Error::other("no connection attempt was made"));
    if source.kind() == io::ErrorKind::TimedOut {
        return Err(TlsError::Timeout {
            phase: "tcp-connect",
            elapsed_ms: config.connect_timeout.as_millis() as u64,
        });
    }

    Err(TlsError::TcpConnect {
        host: host.to_string(),
        port,
        source,
    })
}

fn apply_timeouts(
    stream: &TcpStream,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
) -> Result<(), TlsError> {
    stream
        .set_read_timeout(read_timeout)
        .map_err(|source| TlsError::Io {
            phase: "set-read-timeout",
            source,
        })?;
    stream
        .set_write_timeout(write_timeout)
        .map_err(|source| TlsError::Io {
            phase: "set-write-timeout",
            source,
        })?;
    Ok(())
}

fn map_handshake_io(source: io::Error, config: &TlsConfig) -> TlsError {
    if source.kind() == io::ErrorKind::TimedOut || source.kind() == io::ErrorKind::WouldBlock {
        TlsError::Timeout {
            phase: "tls-handshake",
            elapsed_ms: config.handshake_timeout.as_millis() as u64,
        }
    } else if source.kind() == io::ErrorKind::UnexpectedEof {
        TlsError::ClosedUnexpectedly
    } else {
        TlsError::Io {
            phase: "tls-handshake",
            source,
        }
    }
}

fn validate_server_name(server_name: &str) -> Result<(), TlsError> {
    server_name_for_rustls(server_name).map(|_| ())
}

fn server_name_for_rustls(server_name: &str) -> Result<ServerName<'static>, TlsError> {
    ServerName::try_from(server_name.to_string()).map_err(|_| TlsError::InvalidServerName {
        server_name: server_name.to_string(),
    })
}

fn tls_version_from_rustls(version: Option<ProtocolVersion>) -> TlsVersion {
    match version {
        Some(ProtocolVersion::TLSv1_2) => TlsVersion::Tls12,
        Some(ProtocolVersion::TLSv1_3) => TlsVersion::Tls13,
        _ => TlsVersion::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn default_config_uses_bundled_roots_and_strict_verification() {
        let config = TlsConfig::default();

        assert_eq!(config.min_version, TlsVersion::Tls12);
        assert_eq!(config.max_version, TlsVersion::Tls13);
        assert_eq!(config.root_store, RootStore::Bundled);
        assert_eq!(config.verify_mode, VerifyMode::Strict);
        assert!(bundled_root_count() > 100);
    }

    #[test]
    fn https_default_advertises_http1_alpn() {
        let config = TlsConfig::https_default();

        assert_eq!(config.alpn_protocols, vec!["http/1.1".to_string()]);
    }

    #[test]
    fn endpoint_defaults_to_https_port() {
        let endpoint = TlsEndpoint::https("api.weather.gov").unwrap();

        assert_eq!(endpoint.server_name(), "api.weather.gov");
        assert_eq!(endpoint.port(), 443);
    }

    #[test]
    fn endpoint_rejects_invalid_server_name() {
        let error = TlsEndpoint::https("not a host").unwrap_err();

        assert!(matches!(error, TlsError::InvalidServerName { .. }));
    }

    #[test]
    fn endpoint_rejects_zero_port() {
        let error = TlsEndpoint::new("api.weather.gov", 0).unwrap_err();

        assert!(matches!(error, TlsError::InvalidPort { port: 0 }));
    }

    #[test]
    fn endpoint_summary_redacts_target_name() {
        let endpoint = TlsEndpoint::https("api.weather.gov").unwrap();
        let summary = endpoint.summary(&TlsConfig::https_default());
        let rendered = format!("{summary:?}");

        assert_eq!(summary.server_name_len, "api.weather.gov".len());
        assert_eq!(summary.port, 443);
        assert!(summary.uses_default_https_port);
        assert!(summary.sni_enabled);
        assert_eq!(summary.alpn_protocol_count, 1);
        assert!(!rendered.contains("api.weather.gov"));
    }

    #[test]
    fn rejects_invalid_version_range() {
        let config = TlsConfig {
            min_version: TlsVersion::Tls13,
            max_version: TlsVersion::Tls12,
            ..TlsConfig::default()
        };

        let error = rustls_config(&config).unwrap_err();
        assert!(matches!(error, TlsError::UnsupportedConfig { .. }));
    }

    #[test]
    fn rejects_invalid_custom_root_der() {
        let config = TlsConfig {
            root_store: RootStore::Custom(vec![b"not a cert".to_vec()]),
            ..TlsConfig::default()
        };

        let error = rustls_config(&config).unwrap_err();
        assert!(matches!(error, TlsError::InvalidRootCertificate { .. }));
    }

    #[test]
    #[ignore = "performs a live TLS handshake against api.weather.gov"]
    fn live_tls_handshake_can_write_http_head() {
        let connector = RustlsConnector;
        let mut connection = connector
            .connect("api.weather.gov", 443, &TlsConfig::https_default())
            .unwrap();

        let summary = connection.summary();
        assert!(summary.handshake_complete);
        assert_eq!(summary.port, 443);
        assert_eq!(connection.negotiated_alpn(), Some("http/1.1".to_string()));
        assert!(matches!(
            connection.negotiated_version(),
            TlsVersion::Tls12 | TlsVersion::Tls13
        ));
        assert!(!connection.peer_certificates().unwrap().is_empty());

        connection
            .write_all(
                b"HEAD / HTTP/1.1\r\nHost: api.weather.gov\r\nUser-Agent: coding-adventures-tls-platform-test\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
        connection.flush().unwrap();

        let mut response = [0_u8; 32];
        let bytes_read = connection.read(&mut response).unwrap();
        assert!(bytes_read > 0);
        assert!(response.starts_with(b"HTTP/1.1") || response.starts_with(b"HTTP/2"));
    }
}
