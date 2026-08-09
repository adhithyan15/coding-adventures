//! Pinned authenticated HTTP snapshot delivery for camera-media leases.

#![forbid(unsafe_code)]

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use coding_adventures_csprng::random_array;
use coding_adventures_zeroize::Zeroizing;
use http1::parse_response_head;
use http_core::{find_header, BodyKind, Header};
use http_digest_auth::{DigestAlgorithm, DigestChallenge};
use smart_home_camera_media::{
    CameraMediaExecution, CameraMediaExecutionError, CameraMediaExecutionResult,
    CameraMediaExecutor, CameraMediaKind,
};
use smart_home_core::EntityId;
use std::collections::BTreeMap;
use std::fmt;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::Duration;
use tls_platform::{default_connector, TlsConfig, TlsConnector, VerifyMode};
use url_parser::Url;

pub const VERSION: &str = "0.1.0";
pub const DEFAULT_MAX_HEADER_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_CREDENTIALS: usize = 128;
pub const DEFAULT_TIMEOUT_MS: u64 = 10_000;
pub const MAX_ENDPOINT_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMediaHttpPolicy {
    pub timeout_ms: u64,
    pub max_header_bytes: usize,
    pub max_credentials: usize,
    /// Explicit fixture-only escape hatch. Production defaults remain HTTPS.
    pub allow_plaintext_loopback: bool,
}

impl Default for CameraMediaHttpPolicy {
    fn default() -> Self {
        Self {
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_header_bytes: DEFAULT_MAX_HEADER_BYTES,
            max_credentials: DEFAULT_MAX_CREDENTIALS,
            allow_plaintext_loopback: false,
        }
    }
}

pub struct CameraMediaHttpCredentials {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl CameraMediaHttpCredentials {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Result<Self, String> {
        let username = Zeroizing::new(username.into());
        let password = Zeroizing::new(password.into());
        if username.is_empty()
            || username.contains(':')
            || username.contains(['\r', '\n', '\0'])
            || password.contains(['\r', '\n', '\0'])
        {
            return Err("camera media credentials are empty or contain unsafe text".to_string());
        }
        Ok(Self { username, password })
    }
}

impl fmt::Debug for CameraMediaHttpCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CameraMediaHttpCredentials")
            .field("username", &"[REDACTED]")
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMediaHttpCredentialError {
    CredentialQuotaExceeded { maximum: usize },
}

impl fmt::Display for CameraMediaHttpCredentialError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CredentialQuotaExceeded { maximum } => {
                write!(
                    formatter,
                    "camera media credential quota of {maximum} is exhausted"
                )
            }
        }
    }
}

pub struct CameraMediaHttpExecutor {
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    policy: CameraMediaHttpPolicy,
    credentials: BTreeMap<EntityId, CameraMediaHttpCredentials>,
}

impl fmt::Debug for CameraMediaHttpExecutor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CameraMediaHttpExecutor")
            .field("policy", &self.policy)
            .field("credential_count", &self.credentials.len())
            .field("credentials", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl Default for CameraMediaHttpExecutor {
    fn default() -> Self {
        Self::new(
            default_connector(),
            TlsConfig::https_default(),
            CameraMediaHttpPolicy::default(),
        )
    }
}

impl CameraMediaHttpExecutor {
    pub fn new(
        connector: Box<dyn TlsConnector>,
        mut tls_config: TlsConfig,
        mut policy: CameraMediaHttpPolicy,
    ) -> Self {
        tls_config.verify_mode = VerifyMode::Strict;
        tls_config.alpn_protocols = vec!["http/1.1".to_string()];
        policy.timeout_ms = policy.timeout_ms.max(1);
        policy.max_header_bytes = policy.max_header_bytes.max(1);
        policy.max_credentials = policy.max_credentials.max(1);
        Self {
            connector,
            tls_config,
            policy,
            credentials: BTreeMap::new(),
        }
    }

    pub fn register_credentials(
        &mut self,
        entity_id: EntityId,
        credentials: CameraMediaHttpCredentials,
    ) -> Result<(), CameraMediaHttpCredentialError> {
        if !self.credentials.contains_key(&entity_id)
            && self.credentials.len() >= self.policy.max_credentials
        {
            return Err(CameraMediaHttpCredentialError::CredentialQuotaExceeded {
                maximum: self.policy.max_credentials,
            });
        }
        self.credentials.insert(entity_id, credentials);
        Ok(())
    }

    pub fn unregister_credentials(&mut self, entity_id: &EntityId) -> bool {
        self.credentials.remove(entity_id).is_some()
    }

    pub fn credential_count(&self) -> usize {
        self.credentials.len()
    }

    fn fetch_snapshot(
        &self,
        execution: CameraMediaExecution<'_>,
    ) -> Result<Vec<u8>, CameraMediaExecutionError> {
        self.fetch_snapshot_parts(
            execution.entity_id(),
            execution.endpoint_uri(),
            execution.connection_target(),
            execution.max_snapshot_bytes(),
        )
    }

    fn fetch_snapshot_parts(
        &self,
        entity_id: &EntityId,
        endpoint_uri: &str,
        connection_target: Option<&smart_home_camera_media::CameraMediaConnectionTarget>,
        maximum_snapshot_bytes: usize,
    ) -> Result<Vec<u8>, CameraMediaExecutionError> {
        let endpoint = parse_endpoint(endpoint_uri, connection_target, &self.policy)?;
        let mut response = self.exchange(&endpoint, None, maximum_snapshot_bytes)?;
        if response.status == 401 {
            let credentials = self
                .credentials
                .get(entity_id)
                .ok_or(CameraMediaExecutionError::Rejected)?;
            let auth = select_auth(&response.headers)?;
            let auth_header = authorization(&auth, credentials, &endpoint.target)?;
            response = self.exchange(
                &endpoint,
                Some(auth_header.as_str()),
                maximum_snapshot_bytes,
            )?;
            if response.status == 401 {
                let refreshed = select_auth(&response.headers)?;
                if !matches!(refreshed, SelectedAuth::Digest(_)) {
                    return Err(CameraMediaExecutionError::Rejected);
                }
                let auth_header = authorization(&refreshed, credentials, &endpoint.target)?;
                response = self.exchange(
                    &endpoint,
                    Some(auth_header.as_str()),
                    maximum_snapshot_bytes,
                )?;
            }
        }
        validate_snapshot(response, maximum_snapshot_bytes)
    }

    fn exchange(
        &self,
        endpoint: &ReviewedEndpoint,
        authorization: Option<&str>,
        maximum_body_bytes: usize,
    ) -> Result<WireResponse, CameraMediaExecutionError> {
        let request = encode_request(endpoint, authorization)?;
        let timeout = Duration::from_millis(self.policy.timeout_ms);
        let maximum_wire_bytes = maximum_body_bytes
            .checked_add(self.policy.max_header_bytes)
            .ok_or(CameraMediaExecutionError::ResourceLimit)?;
        let response = match endpoint.scheme {
            EndpointScheme::Http => {
                let mut stream = TcpStream::connect_timeout(&endpoint.address, timeout)
                    .map_err(|_| CameraMediaExecutionError::Unavailable)?;
                stream
                    .set_read_timeout(Some(timeout))
                    .and_then(|_| stream.set_write_timeout(Some(timeout)))
                    .map_err(|_| CameraMediaExecutionError::Unavailable)?;
                write_request(&mut stream, request.as_slice())?;
                read_bounded(&mut stream, maximum_wire_bytes)?
            }
            EndpointScheme::Https => {
                let mut config = self.tls_config.clone();
                config.connect_timeout = timeout;
                config.read_timeout = Some(timeout);
                config.write_timeout = Some(timeout);
                let mut stream = self
                    .connector
                    .connect_addr(&endpoint.server_name, endpoint.address, &config)
                    .map_err(|_| CameraMediaExecutionError::Unavailable)?;
                write_request(&mut stream, request.as_slice())?;
                let response = read_bounded(&mut stream, maximum_wire_bytes)?;
                stream
                    .close_notify()
                    .map_err(|_| CameraMediaExecutionError::Unavailable)?;
                response
            }
        };
        decode_response(
            response.as_slice(),
            self.policy.max_header_bytes,
            maximum_body_bytes,
        )
    }
}

impl CameraMediaExecutor for CameraMediaHttpExecutor {
    type Stream = ();

    fn deliver(
        &mut self,
        execution: CameraMediaExecution<'_>,
    ) -> Result<CameraMediaExecutionResult<Self::Stream>, CameraMediaExecutionError> {
        if execution.kind() != CameraMediaKind::Snapshot {
            return Err(CameraMediaExecutionError::Rejected);
        }
        self.fetch_snapshot(execution)
            .map(CameraMediaExecutionResult::snapshot)
    }

    fn close_stream(
        &mut self,
        _stream: &mut Self::Stream,
    ) -> Result<(), CameraMediaExecutionError> {
        Err(CameraMediaExecutionError::Rejected)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndpointScheme {
    Http,
    Https,
}

struct ReviewedEndpoint {
    scheme: EndpointScheme,
    server_name: String,
    address: SocketAddr,
    host_header: String,
    target: Zeroizing<String>,
}

fn parse_endpoint(
    endpoint_uri: &str,
    connection_target: Option<&smart_home_camera_media::CameraMediaConnectionTarget>,
    policy: &CameraMediaHttpPolicy,
) -> Result<ReviewedEndpoint, CameraMediaExecutionError> {
    if endpoint_uri.len() > MAX_ENDPOINT_BYTES {
        return Err(CameraMediaExecutionError::ResourceLimit);
    }
    let url = Url::parse(endpoint_uri).map_err(|_| CameraMediaExecutionError::Protocol)?;
    let target = connection_target.ok_or(CameraMediaExecutionError::Rejected)?;
    let host = url
        .host
        .as_deref()
        .ok_or(CameraMediaExecutionError::Protocol)?;
    let port = url
        .effective_port()
        .ok_or(CameraMediaExecutionError::Protocol)?;
    if url.userinfo.is_some()
        || url.fragment.is_some()
        || host.contains(['\r', '\n', '\0'])
        || !host.eq_ignore_ascii_case(target.canonical_host())
        || port != target.pinned_address().port()
    {
        return Err(CameraMediaExecutionError::Rejected);
    }
    let scheme = match url.scheme.as_str() {
        "https" => EndpointScheme::Https,
        "http"
            if policy.allow_plaintext_loopback
                && target.pinned_address().ip().is_loopback()
                && is_loopback_host(host) =>
        {
            EndpointScheme::Http
        }
        _ => return Err(CameraMediaExecutionError::Rejected),
    };
    if host
        .trim_matches(['[', ']'])
        .parse::<IpAddr>()
        .is_ok_and(|literal| literal != target.pinned_address().ip())
    {
        return Err(CameraMediaExecutionError::Rejected);
    }
    let target_text = match &url.query {
        Some(query) => format!("{}?{query}", url.path),
        None => url.path.clone(),
    };
    if !target_text.starts_with('/')
        || target_text.is_empty()
        || target_text.bytes().any(|byte| !byte.is_ascii_graphic())
    {
        return Err(CameraMediaExecutionError::Protocol);
    }
    let default_port = if scheme == EndpointScheme::Https {
        443
    } else {
        80
    };
    let host_text = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    let host_header = if port == default_port {
        host_text
    } else {
        format!("{host_text}:{port}")
    };
    Ok(ReviewedEndpoint {
        scheme,
        server_name: target.canonical_host().trim_matches(['[', ']']).to_string(),
        address: target.pinned_address(),
        host_header,
        target: Zeroizing::new(target_text),
    })
}

fn encode_request(
    endpoint: &ReviewedEndpoint,
    authorization: Option<&str>,
) -> Result<Zeroizing<Vec<u8>>, CameraMediaExecutionError> {
    if endpoint.host_header.contains(['\r', '\n', '\0'])
        || authorization.is_some_and(|value| value.contains(['\r', '\n', '\0']))
    {
        return Err(CameraMediaExecutionError::Protocol);
    }
    let mut request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nAccept: image/jpeg, image/png, image/webp\r\nConnection: close\r\n",
        endpoint.target.as_str(), endpoint.host_header
    )
    .into_bytes();
    if let Some(value) = authorization {
        request.extend_from_slice(format!("Authorization: {value}\r\n").as_bytes());
    }
    request.extend_from_slice(b"\r\n");
    Ok(Zeroizing::new(request))
}

fn write_request(writer: &mut dyn Write, request: &[u8]) -> Result<(), CameraMediaExecutionError> {
    writer
        .write_all(request)
        .and_then(|_| writer.flush())
        .map_err(|_| CameraMediaExecutionError::Unavailable)
}

fn read_bounded(
    reader: &mut dyn Read,
    maximum: usize,
) -> Result<Zeroizing<Vec<u8>>, CameraMediaExecutionError> {
    let mut bytes = Zeroizing::new(Vec::new());
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| CameraMediaExecutionError::Unavailable)?;
        if read == 0 {
            return Ok(bytes);
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(CameraMediaExecutionError::ResourceLimit);
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
}

struct WireResponse {
    status: u16,
    headers: Vec<Header>,
    body: Zeroizing<Vec<u8>>,
}

fn decode_response(
    bytes: &[u8],
    maximum_header_bytes: usize,
    maximum_body_bytes: usize,
) -> Result<WireResponse, CameraMediaExecutionError> {
    let parsed = parse_response_head(bytes).map_err(|_| CameraMediaExecutionError::Protocol)?;
    if parsed.body_offset > maximum_header_bytes {
        return Err(CameraMediaExecutionError::ResourceLimit);
    }
    let content_lengths = parsed
        .head
        .headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case("Content-Length"))
        .count();
    let transfer_encodings = parsed
        .head
        .headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case("Transfer-Encoding"))
        .count();
    if content_lengths > 1
        || transfer_encodings > 1
        || (content_lengths > 0 && transfer_encodings > 0)
    {
        return Err(CameraMediaExecutionError::Protocol);
    }
    if let Some(value) = find_header(&parsed.head.headers, "Transfer-Encoding") {
        if !value.trim().eq_ignore_ascii_case("chunked") {
            return Err(CameraMediaExecutionError::Protocol);
        }
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if expected > maximum_body_bytes {
                return Err(CameraMediaExecutionError::ResourceLimit);
            }
            if input.len() != expected {
                return Err(CameraMediaExecutionError::Protocol);
            }
            input.to_vec()
        }
        BodyKind::UntilEof => {
            if input.len() > maximum_body_bytes {
                return Err(CameraMediaExecutionError::ResourceLimit);
            }
            input.to_vec()
        }
        BodyKind::Chunked => decode_chunked(input, maximum_body_bytes)?,
    };
    Ok(WireResponse {
        status: parsed.head.status,
        headers: parsed.head.headers,
        body: Zeroizing::new(body),
    })
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, CameraMediaExecutionError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input
            .get(cursor..)
            .ok_or(CameraMediaExecutionError::Protocol)?
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or(CameraMediaExecutionError::Protocol)?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| CameraMediaExecutionError::Protocol)?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| CameraMediaExecutionError::Protocol)?;
        cursor = end + 2;
        if size == 0 {
            if input.get(cursor..) != Some(b"\r\n") {
                return Err(CameraMediaExecutionError::Protocol);
            }
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(CameraMediaExecutionError::ResourceLimit);
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or(CameraMediaExecutionError::ResourceLimit)?;
        if input.get(chunk_end..chunk_end + 2) != Some(b"\r\n") {
            return Err(CameraMediaExecutionError::Protocol);
        }
        output.extend_from_slice(&input[cursor..chunk_end]);
        cursor = chunk_end + 2;
    }
}

fn validate_snapshot(
    response: WireResponse,
    maximum: usize,
) -> Result<Vec<u8>, CameraMediaExecutionError> {
    match response.status {
        200 => {}
        401 | 403 | 407 | 429 => return Err(CameraMediaExecutionError::Rejected),
        300..=399 => return Err(CameraMediaExecutionError::Rejected),
        500..=599 => return Err(CameraMediaExecutionError::Unavailable),
        _ => return Err(CameraMediaExecutionError::Protocol),
    }
    if response.body.is_empty() {
        return Err(CameraMediaExecutionError::Protocol);
    }
    if response.body.len() > maximum {
        return Err(CameraMediaExecutionError::ResourceLimit);
    }
    if find_header(&response.headers, "Content-Encoding").is_some() {
        return Err(CameraMediaExecutionError::Protocol);
    }
    let media_type = find_header(&response.headers, "Content-Type")
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .ok_or(CameraMediaExecutionError::Protocol)?;
    let signature_matches = if media_type.eq_ignore_ascii_case("image/jpeg") {
        response.body.starts_with(&[0xff, 0xd8, 0xff])
    } else if media_type.eq_ignore_ascii_case("image/png") {
        response.body.starts_with(b"\x89PNG\r\n\x1a\n")
    } else if media_type.eq_ignore_ascii_case("image/webp") {
        response.body.len() >= 12
            && response.body.starts_with(b"RIFF")
            && response.body.get(8..12) == Some(b"WEBP")
    } else {
        false
    };
    if !signature_matches {
        return Err(CameraMediaExecutionError::Protocol);
    }
    Ok(response.body.to_vec())
}

enum SelectedAuth {
    Basic,
    Digest(DigestChallenge),
}

fn select_auth(headers: &[Header]) -> Result<SelectedAuth, CameraMediaExecutionError> {
    let challenges = headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case("WWW-Authenticate"))
        .map(|header| header.value.trim())
        .collect::<Vec<_>>();
    let digest = challenges
        .iter()
        .filter(|value| {
            value
                .split_ascii_whitespace()
                .next()
                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("Digest"))
        })
        .filter_map(|value| DigestChallenge::parse(value).ok())
        .max_by_key(|challenge| match challenge.algorithm() {
            DigestAlgorithm::Sha256 | DigestAlgorithm::Sha256Sess => 2,
            DigestAlgorithm::Md5 | DigestAlgorithm::Md5Sess => 1,
        });
    if let Some(challenge) = digest {
        return Ok(SelectedAuth::Digest(challenge));
    }
    if challenges.iter().any(|value| {
        value
            .split_ascii_whitespace()
            .next()
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("Basic"))
    }) {
        return Ok(SelectedAuth::Basic);
    }
    Err(CameraMediaExecutionError::Rejected)
}

fn authorization(
    auth: &SelectedAuth,
    credentials: &CameraMediaHttpCredentials,
    target: &str,
) -> Result<Zeroizing<String>, CameraMediaExecutionError> {
    match auth {
        SelectedAuth::Basic => {
            let raw = Zeroizing::new(format!(
                "{}:{}",
                credentials.username.as_str(),
                credentials.password.as_str()
            ));
            let encoded = Zeroizing::new(BASE64.encode(raw.as_bytes()));
            Ok(Zeroizing::new(format!("Basic {}", encoded.as_str())))
        }
        SelectedAuth::Digest(challenge) => {
            let nonce = Zeroizing::new(
                random_array::<16>().map_err(|_| CameraMediaExecutionError::Unavailable)?,
            );
            let client_nonce = Zeroizing::new(hex_bytes(nonce.as_slice()));
            challenge
                .authorization(
                    credentials.username.as_str(),
                    credentials.password.as_str(),
                    "GET",
                    target,
                    client_nonce.as_str(),
                    1,
                )
                .map_err(|_| CameraMediaExecutionError::Rejected)
        }
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn is_loopback_host(host: &str) -> bool {
    host == "localhost"
        || host == "[::1]"
        || host == "::1"
        || host == "127.0.0.1"
        || host.strip_prefix("127.").is_some_and(|suffix| {
            let octets = suffix.split('.').collect::<Vec<_>>();
            octets.len() == 3 && octets.iter().all(|octet| octet.parse::<u8>().is_ok())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_camera_media::{
        CameraMediaAccessRequest, CameraMediaClock, CameraMediaConnectionTarget,
        CameraMediaNonceError, CameraMediaNonceSource, CameraMediaPolicy,
        CameraMediaPrincipalSource, CameraMediaService,
    };
    use smart_home_core::{
        AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityGrant, CapabilityGrantId,
        CapabilityMode, Device, DeviceId, Entity, EntityKind, Health, IntegrationId, Metadata,
        PrivilegeTier, ProtocolIdentifier, ValueKind,
    };
    use smart_home_runtime::SmartHomeRuntime;
    use std::cell::{Cell, RefCell};
    use std::io::Cursor;
    use std::net::TcpListener;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};
    use std::thread;

    struct PanicConnector;

    impl TlsConnector for PanicConnector {
        fn connect(
            &self,
            _host: &str,
            _port: u16,
            _config: &TlsConfig,
        ) -> Result<Box<dyn tls_platform::TlsStream>, tls_platform::TlsError> {
            panic!("TLS connector should not be reached")
        }

        fn connect_addr(
            &self,
            _server_name: &str,
            _address: SocketAddr,
            _config: &TlsConfig,
        ) -> Result<Box<dyn tls_platform::TlsStream>, tls_platform::TlsError> {
            panic!("TLS connector should not be reached")
        }
    }

    #[derive(Default)]
    struct TlsCapture {
        connects: Vec<(String, SocketAddr, bool)>,
        request: Vec<u8>,
    }

    struct RecordingConnector {
        response: Vec<u8>,
        capture: Arc<Mutex<TlsCapture>>,
    }

    impl TlsConnector for RecordingConnector {
        fn connect(
            &self,
            _host: &str,
            _port: u16,
            _config: &TlsConfig,
        ) -> Result<Box<dyn tls_platform::TlsStream>, tls_platform::TlsError> {
            panic!("executor must use the reviewed socket path")
        }

        fn connect_addr(
            &self,
            server_name: &str,
            address: SocketAddr,
            config: &TlsConfig,
        ) -> Result<Box<dyn tls_platform::TlsStream>, tls_platform::TlsError> {
            self.capture.lock().unwrap().connects.push((
                server_name.to_string(),
                address,
                config.verify_mode == VerifyMode::Strict,
            ));
            Ok(Box::new(RecordingTlsStream {
                response: Cursor::new(self.response.clone()),
                capture: Arc::clone(&self.capture),
            }))
        }
    }

    struct RecordingTlsStream {
        response: Cursor<Vec<u8>>,
        capture: Arc<Mutex<TlsCapture>>,
    }

    impl Read for RecordingTlsStream {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            self.response.read(buffer)
        }
    }

    impl Write for RecordingTlsStream {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.capture
                .lock()
                .unwrap()
                .request
                .extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl tls_platform::TlsStream for RecordingTlsStream {
        fn peer_certificates(&self) -> Result<Vec<Vec<u8>>, tls_platform::TlsError> {
            Ok(Vec::new())
        }

        fn negotiated_alpn(&self) -> Option<String> {
            Some("http/1.1".to_string())
        }

        fn negotiated_version(&self) -> tls_platform::TlsVersion {
            tls_platform::TlsVersion::Tls13
        }

        fn close_notify(&mut self) -> Result<(), tls_platform::TlsError> {
            Ok(())
        }

        fn summary(&self) -> tls_platform::TlsConnectionSummary {
            panic!("summary is not used by the executor")
        }
    }

    #[derive(Clone)]
    struct TestClock(Rc<Cell<u64>>);

    impl CameraMediaClock for TestClock {
        fn now_ms(&self) -> u64 {
            self.0.get()
        }
    }

    #[derive(Clone)]
    struct TestPrincipal(Rc<RefCell<Option<AgentId>>>);

    impl CameraMediaPrincipalSource for TestPrincipal {
        fn current_principal(&self) -> Option<AgentId> {
            self.0.borrow().clone()
        }
    }

    struct TestNonce(u8);

    impl CameraMediaNonceSource for TestNonce {
        fn fill_nonce(&mut self, output: &mut [u8; 16]) -> Result<(), CameraMediaNonceError> {
            output.fill(self.0);
            self.0 = self.0.wrapping_add(1);
            Ok(())
        }
    }

    fn fixture_executor() -> CameraMediaHttpExecutor {
        CameraMediaHttpExecutor::new(
            Box::new(PanicConnector),
            TlsConfig::https_default(),
            CameraMediaHttpPolicy {
                allow_plaintext_loopback: true,
                ..CameraMediaHttpPolicy::default()
            },
        )
    }

    fn serve(responses: Vec<Vec<u8>>) -> (SocketAddr, Arc<Mutex<Vec<Vec<u8>>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = Vec::new();
                let mut buffer = [0u8; 2048];
                loop {
                    let count = stream.read(&mut buffer).unwrap();
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                captured.lock().unwrap().push(request);
                stream.write_all(&response).unwrap();
            }
        });
        (address, requests)
    }

    fn fixture_runtime() -> (SmartHomeRuntime, EntityId, AgentId) {
        let mut runtime = SmartHomeRuntime::default();
        let bridge_id = BridgeId::trusted("camera-bridge");
        let device_id = DeviceId::trusted("camera-device");
        let entity_id = EntityId::trusted("camera-entity");
        let principal_id = AgentId::trusted("dashboard-user");
        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted("onvif"),
            BridgeTransport::LanHttp,
        );
        bridge.health = Health::Online;
        runtime.upsert_bridge(bridge).unwrap();
        runtime
            .upsert_device(Device {
                device_id: device_id.clone(),
                bridge_id,
                manufacturer: "Fixture".to_string(),
                model: "Camera".to_string(),
                name: "Front Door".to_string(),
                serial: None,
                firmware_version: None,
                room_id: None,
                entity_ids: vec![entity_id.clone()],
                identifiers: Vec::<ProtocolIdentifier>::new(),
                health: Health::Online,
                metadata: Vec::<Metadata>::new(),
            })
            .unwrap();
        runtime
            .upsert_entity(Entity {
                entity_id: entity_id.clone(),
                device_id,
                kind: EntityKind::Camera,
                name: "Front Door".to_string(),
                capabilities: vec![Capability::new(
                    CameraMediaKind::Snapshot.capability_id(),
                    CapabilityMode::Command,
                    ValueKind::Text,
                )],
                state: None,
                metadata: Vec::new(),
            })
            .unwrap();
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_entity_capability(
                CapabilityGrantId::trusted("snapshot-grant"),
                principal_id.clone(),
                entity_id.clone(),
                CameraMediaKind::Snapshot.capability_id(),
                PrivilegeTier::HumanApproval,
                "user",
                1,
            ));
        (runtime, entity_id, principal_id)
    }

    #[test]
    fn credentials_and_executor_debug_are_redacted() {
        let credentials = CameraMediaHttpCredentials::new("operator", "secret").unwrap();
        assert!(!format!("{credentials:?}").contains("operator"));
        assert!(!format!("{credentials:?}").contains("secret"));
        let mut executor = fixture_executor();
        executor
            .register_credentials(EntityId::trusted("camera.one"), credentials)
            .unwrap();
        let debug = format!("{executor:?}");
        assert!(!debug.contains("operator"));
        assert!(!debug.contains("secret"));
    }

    #[test]
    fn https_uses_only_the_pinned_socket_and_forces_strict_tls_identity() {
        let image = vec![0xff, 0xd8, 0xff, 0xe0, 1];
        let response = [
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
                image.len()
            )
            .into_bytes(),
            image.clone(),
        ]
        .concat();
        let capture = Arc::new(Mutex::new(TlsCapture::default()));
        let connector = RecordingConnector {
            response,
            capture: Arc::clone(&capture),
        };
        let mut config = TlsConfig::https_default();
        config.verify_mode = VerifyMode::NoHostname;
        let executor = CameraMediaHttpExecutor::new(
            Box::new(connector),
            config,
            CameraMediaHttpPolicy::default(),
        );
        let entity_id = EntityId::trusted("camera.one");
        let address = "127.0.0.1:8443".parse().unwrap();
        let target = CameraMediaConnectionTarget::new("camera.local", address);
        assert_eq!(
            executor
                .fetch_snapshot_parts(
                    &entity_id,
                    "https://camera.local:8443/snapshot.jpg",
                    Some(&target),
                    64,
                )
                .unwrap(),
            image
        );
        let capture = capture.lock().unwrap();
        assert_eq!(
            capture.connects,
            vec![("camera.local".to_string(), address, true)]
        );
        let request = String::from_utf8_lossy(&capture.request);
        assert!(request.contains("Host: camera.local:8443"));
        assert!(!request.contains("Authorization:"));
    }

    #[test]
    fn basic_fallback_authorization_is_zeroizing_and_correct() {
        let headers = vec![Header {
            name: "WWW-Authenticate".to_string(),
            value: "Basic realm=\"camera\"".to_string(),
        }];
        let selected = select_auth(&headers).unwrap();
        assert!(matches!(selected, SelectedAuth::Basic));
        let credentials = CameraMediaHttpCredentials::new("operator", "secret").unwrap();
        assert_eq!(
            authorization(&selected, &credentials, "/snapshot")
                .unwrap()
                .as_str(),
            "Basic b3BlcmF0b3I6c2VjcmV0"
        );
    }

    #[test]
    fn digest_probe_prefers_sha256_and_delivers_one_bounded_snapshot() {
        let challenge = b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"camera\"\r\nWWW-Authenticate: Digest realm=\"camera\", nonce=\"server\", algorithm=SHA-256, qop=\"auth\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec();
        let image = vec![0xff, 0xd8, 0xff, 0xe0, 1, 2, 3, 4];
        let success = [
            format!("HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", image.len()).into_bytes(),
            image.clone(),
        ]
        .concat();
        let (address, requests) = serve(vec![challenge, success]);
        let uri = format!(
            "http://127.0.0.1:{}/snapshot.jpg?profile=main",
            address.port()
        );
        let target = CameraMediaConnectionTarget::new("127.0.0.1", address);
        let entity_id = EntityId::trusted("camera.one");
        let mut executor = fixture_executor();
        executor
            .register_credentials(
                entity_id.clone(),
                CameraMediaHttpCredentials::new("operator", "secret").unwrap(),
            )
            .unwrap();
        let bytes = executor
            .fetch_snapshot_parts(&entity_id, &uri, Some(&target), 64)
            .unwrap();
        assert_eq!(bytes.as_slice(), image.as_slice());
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(!String::from_utf8_lossy(&requests[0]).contains("Authorization:"));
        let authenticated = String::from_utf8_lossy(&requests[1]);
        assert!(authenticated.contains("GET /snapshot.jpg?profile=main HTTP/1.1"));
        assert!(authenticated.contains("Authorization: Digest "));
        assert!(authenticated.contains("algorithm=SHA-256"));
        assert!(!authenticated.contains("secret"));
    }

    #[test]
    fn real_loopback_redeems_one_public_broker_lease_without_exposing_endpoint() {
        let image = vec![0xff, 0xd8, 0xff, 0xe0, 7, 8, 9];
        let success = [
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                image.len()
            )
            .into_bytes(),
            image.clone(),
        ]
        .concat();
        let (address, requests) = serve(vec![success]);
        let uri = format!("http://127.0.0.1:{}/snapshot.jpg", address.port());
        let (runtime, entity_id, principal_id) = fixture_runtime();
        let mut service = CameraMediaService::new(
            CameraMediaPolicy {
                allow_plaintext_loopback: true,
                ..CameraMediaPolicy::default()
            },
            TestClock(Rc::new(Cell::new(10))),
            TestNonce(0x31),
            TestPrincipal(Rc::new(RefCell::new(Some(principal_id)))),
            fixture_executor(),
        );
        service
            .register_pinned_endpoint(
                entity_id.clone(),
                CameraMediaKind::Snapshot,
                uri.clone(),
                CameraMediaConnectionTarget::new("127.0.0.1", address),
            )
            .unwrap();
        let lease = service
            .issue_lease(
                &runtime,
                CameraMediaAccessRequest::new(
                    entity_id,
                    CameraMediaKind::Snapshot,
                    "operator preview",
                    500,
                ),
            )
            .unwrap();
        let delivery = service.deliver_lease(&runtime, &lease.lease_id).unwrap();
        assert_eq!(delivery.snapshot_bytes(), Some(image.as_slice()));
        assert_eq!(requests.lock().unwrap().len(), 1);
        let public = format!("{delivery:?}{:?}", service.snapshot());
        assert!(!public.contains(&uri));
    }

    #[test]
    fn one_refreshed_digest_challenge_is_bounded_to_three_requests() {
        let first = b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Digest realm=\"camera\", nonce=\"one\", algorithm=MD5, qop=\"auth\"\r\nContent-Length: 0\r\n\r\n".to_vec();
        let stale = b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Digest realm=\"camera\", nonce=\"two\", algorithm=SHA-256, qop=\"auth\", stale=true\r\nContent-Length: 0\r\n\r\n".to_vec();
        let image = vec![0x89, b'P', b'N', b'G', 13, 10, 26, 10, 1];
        let success = [
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\n\r\n",
                image.len()
            )
            .into_bytes(),
            image,
        ]
        .concat();
        let (address, requests) = serve(vec![first, stale, success]);
        let uri = format!("http://127.0.0.1:{}/snapshot", address.port());
        let target = CameraMediaConnectionTarget::new("127.0.0.1", address);
        let entity_id = EntityId::trusted("camera.one");
        let mut executor = fixture_executor();
        executor
            .register_credentials(
                entity_id.clone(),
                CameraMediaHttpCredentials::new("operator", "secret").unwrap(),
            )
            .unwrap();
        assert!(executor
            .fetch_snapshot_parts(&entity_id, &uri, Some(&target), 64)
            .is_ok());
        assert_eq!(requests.lock().unwrap().len(), 3);
    }

    #[test]
    fn rejects_redirects_non_images_and_oversized_bodies() {
        let redirect = b"HTTP/1.1 302 Found\r\nLocation: https://attacker.invalid/secret\r\nContent-Length: 0\r\n\r\n".to_vec();
        let html =
            b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 4\r\n\r\noops".to_vec();
        let oversized = b"HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: 9\r\n\r\n\xff\xd8\xff123456".to_vec();
        for response in [redirect, html, oversized] {
            let (address, _) = serve(vec![response]);
            let uri = format!("http://127.0.0.1:{}/snapshot", address.port());
            let target = CameraMediaConnectionTarget::new("127.0.0.1", address);
            let entity_id = EntityId::trusted("camera.one");
            assert!(fixture_executor()
                .fetch_snapshot_parts(&entity_id, &uri, Some(&target), 8)
                .is_err());
        }
    }

    #[test]
    fn rejects_unpinned_plaintext_and_stream_execution_before_io() {
        let entity_id = EntityId::trusted("camera.one");
        let target =
            CameraMediaConnectionTarget::new("camera.local", "127.0.0.1:9".parse().unwrap());
        assert_eq!(
            CameraMediaHttpExecutor::default().fetch_snapshot_parts(
                &entity_id,
                "http://camera.local/snapshot",
                Some(&target),
                64,
            ),
            Err(CameraMediaExecutionError::Rejected)
        );
        assert_eq!(
            CameraMediaExecutor::close_stream(&mut fixture_executor(), &mut ()),
            Err(CameraMediaExecutionError::Rejected)
        );
    }

    #[test]
    fn chunked_decoder_is_bounded_and_rejects_smuggling_shapes() {
        assert_eq!(
            decode_chunked(b"4\r\nRIFF\r\n0\r\n\r\n", 4).unwrap(),
            b"RIFF"
        );
        assert_eq!(
            decode_chunked(b"5\r\nlarge\r\n0\r\n\r\n", 4),
            Err(CameraMediaExecutionError::ResourceLimit)
        );
        assert!(decode_response(
            b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
            1024,
            1024,
        )
        .is_err());
    }
}
