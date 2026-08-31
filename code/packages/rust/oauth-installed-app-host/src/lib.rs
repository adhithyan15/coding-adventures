//! Audit-first loopback host for OAuth installed applications.
//!
//! This crate owns the deliberately narrow I/O authority omitted from
//! `coding_adventures_oauth`: literal-loopback binding, an injected external
//! user agent, and one bounded HTTP/1.1 callback. Provider protocol behavior
//! remains data in the pure OAuth crate.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_oauth::{AuthorizationRequest, OAuthTraceId, ProviderId};
use coding_adventures_zeroize::Zeroizing;
use std::fmt::{self, Debug, Display, Formatter};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

const MAX_CALLBACK_PATH_BYTES: usize = 1_024;
const MAX_REQUEST_HEAD_BYTES: usize = 16 * 1024;
const MAX_REQUEST_LINE_BYTES: usize = 8 * 1024;
const MAX_HEADER_LINE_BYTES: usize = 2 * 1024;
const MAX_HEADERS: usize = 64;
const MAX_CALLBACK_WAIT: Duration = Duration::from_secs(10 * 60);
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(5);
const SUCCESS_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 75\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n<!doctype html><title>Authorization complete</title>You may close this tab.";
const FAILURE_RESPONSE: &[u8] = b"HTTP/1.1 400 Bad Request\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: 27\r\nConnection: close\r\nCache-Control: no-store\r\n\r\nInvalid authorization reply";

/// Literal loopback interface selected for a transient callback listener.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopbackAddress {
    /// IPv4 loopback at `127.0.0.1`.
    Ipv4,
    /// IPv6 loopback at `::1`.
    Ipv6,
}

impl LoopbackAddress {
    fn socket_address(self, port: u16) -> SocketAddr {
        match self {
            Self::Ipv4 => SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            Self::Ipv6 => SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port),
        }
    }

    fn uri_host(self) -> &'static str {
        match self {
            Self::Ipv4 => "127.0.0.1",
            Self::Ipv6 => "[::1]",
        }
    }
}

/// External effect bracketed by durable loopback-host audit records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopbackAuditAction {
    /// Bind a literal-loopback TCP listener.
    Bind,
    /// Release an authorization URL to the external user agent.
    BrowserOpen,
    /// Accept and validate one callback request.
    CallbackReceive,
}

/// Closed, privacy-safe result of one audited host boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopbackAuditOutcome {
    /// Durable intent recorded before the external effect was attempted.
    Attempted,
    /// The effect and its bounded validation completed successfully.
    Succeeded,
    /// The effect failed with a closed class and no attacker-controlled text.
    Failed(LoopbackFailureClass),
}

/// Closed failure class safe for durable audit storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopbackFailureClass {
    /// Caller input or cross-boundary provider/trace/redirect binding failed.
    InvalidInput,
    /// Literal-loopback listener setup failed.
    Bind,
    /// The injected external user agent rejected the URL.
    Browser,
    /// No callback arrived within the caller's bounded wait.
    Timeout,
    /// The peer or HTTP callback violated the strict protocol profile.
    Protocol,
}

/// Privacy-safe durable audit row for one installed-app host boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopbackAuditEvent {
    provider: ProviderId,
    trace: OAuthTraceId,
    action: LoopbackAuditAction,
    outcome: LoopbackAuditOutcome,
}

impl LoopbackAuditEvent {
    /// Return the stable provider identity.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the caller-owned trace shared with the OAuth core.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the stable host action.
    pub const fn action(&self) -> LoopbackAuditAction {
        self.action
    }

    /// Return the closed host outcome.
    pub const fn outcome(&self) -> LoopbackAuditOutcome {
        self.outcome
    }
}

/// Durable publication required before and after every external host effect.
pub trait LoopbackAuditSink {
    /// Persist `event` durably or fail closed without diagnostic text.
    fn publish(&mut self, event: &LoopbackAuditEvent) -> Result<(), LoopbackAuditError>;
}

/// Closed durable-audit error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoopbackAuditError;

/// Closed external-user-agent failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalUserAgentError;

/// Authorized platform adapter that opens a URL in the user's browser.
pub trait ExternalUserAgent {
    /// Open the exact authorization URL or return a closed failure.
    fn open(&mut self, authorization_url: &str) -> Result<(), ExternalUserAgentError>;
}

/// Closed host error that never contains OS or callback diagnostics.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LoopbackHostError {
    /// Callback path, timeout, or request binding was invalid.
    InvalidConfiguration,
    /// The literal-loopback listener could not be prepared.
    Bind,
    /// The external user agent rejected the authorization URL.
    Browser,
    /// No connection arrived before the bounded deadline.
    Timeout,
    /// The first connection or HTTP request failed strict validation.
    Protocol,
    /// Durable audit publication failed and the result was withheld.
    Audit,
}

impl LoopbackHostError {
    fn failure_class(self) -> Option<LoopbackFailureClass> {
        match self {
            Self::InvalidConfiguration => Some(LoopbackFailureClass::InvalidInput),
            Self::Bind => Some(LoopbackFailureClass::Bind),
            Self::Browser => Some(LoopbackFailureClass::Browser),
            Self::Timeout => Some(LoopbackFailureClass::Timeout),
            Self::Protocol => Some(LoopbackFailureClass::Protocol),
            Self::Audit => None,
        }
    }
}

impl Debug for LoopbackHostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidConfiguration => "InvalidConfiguration",
            Self::Bind => "Bind",
            Self::Browser => "Browser",
            Self::Timeout => "Timeout",
            Self::Protocol => "Protocol",
            Self::Audit => "Audit",
        })
    }
}

impl Display for LoopbackHostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidConfiguration => "oauth loopback host: invalid configuration",
            Self::Bind => "oauth loopback host: bind failed",
            Self::Browser => "oauth loopback host: browser open failed",
            Self::Timeout => "oauth loopback host: callback timed out",
            Self::Protocol => "oauth loopback host: invalid callback request",
            Self::Audit => "oauth loopback host: audit publication failed",
        })
    }
}

impl std::error::Error for LoopbackHostError {}

/// One transient literal-loopback listener bound to an OAuth trace.
pub struct LoopbackHost {
    listener: TcpListener,
    provider: ProviderId,
    trace: OAuthTraceId,
    address: LoopbackAddress,
    port: u16,
    callback_path: String,
    redirect_uri: String,
    expected_host: String,
}

impl LoopbackHost {
    /// Audit, then bind a literal-loopback listener.
    ///
    /// `port == 0` asks the operating system for an ephemeral port. The
    /// returned [`Self::redirect_uri`] must then be used to construct the
    /// provider configuration before authorization begins.
    pub fn bind<S: LoopbackAuditSink>(
        provider: ProviderId,
        trace: OAuthTraceId,
        address: LoopbackAddress,
        port: u16,
        callback_path: impl Into<String>,
        audit: &mut S,
    ) -> Result<Self, LoopbackHostError> {
        publish(
            audit,
            &provider,
            trace,
            LoopbackAuditAction::Bind,
            LoopbackAuditOutcome::Attempted,
        )?;
        let result = bind_inner(provider.clone(), trace, address, port, callback_path.into());
        finish(audit, &provider, trace, LoopbackAuditAction::Bind, result)
    }

    /// Return the stable provider identity.
    pub fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the exact trace shared with the OAuth core.
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the selected literal-loopback family.
    pub const fn address(&self) -> LoopbackAddress {
        self.address
    }

    /// Return the actual bound port, including an OS-selected ephemeral port.
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Borrow the exact redirect URI for [`coding_adventures_oauth::ProviderConfig`].
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// Audit, verify ceremony binding, then release the URL to the user agent.
    pub fn open_browser<U: ExternalUserAgent, S: LoopbackAuditSink>(
        &self,
        request: &AuthorizationRequest,
        user_agent: &mut U,
        audit: &mut S,
    ) -> Result<(), LoopbackHostError> {
        publish(
            audit,
            &self.provider,
            self.trace,
            LoopbackAuditAction::BrowserOpen,
            LoopbackAuditOutcome::Attempted,
        )?;
        let result = if request.provider() != &self.provider
            || request.trace() != self.trace
            || request.redirect_uri() != self.redirect_uri
        {
            Err(LoopbackHostError::InvalidConfiguration)
        } else {
            user_agent
                .open(request.url().as_str())
                .map_err(|_| LoopbackHostError::Browser)
        };
        finish(
            audit,
            &self.provider,
            self.trace,
            LoopbackAuditAction::BrowserOpen,
            result,
        )
    }

    /// Audit, wait for one connection, close the listener, and validate it.
    ///
    /// The listener is consumed by the first accepted connection even when
    /// that peer sends a malformed request. The callback remains opaque here;
    /// pass [`LoopbackCallback::as_uri`] to
    /// [`coding_adventures_oauth::complete_authorization`] for exact state,
    /// redirect, issuer, and code validation.
    pub fn receive_callback<S: LoopbackAuditSink>(
        self,
        timeout: Duration,
        audit: &mut S,
    ) -> Result<LoopbackCallback, LoopbackHostError> {
        let provider = self.provider.clone();
        let trace = self.trace;
        publish(
            audit,
            &provider,
            trace,
            LoopbackAuditAction::CallbackReceive,
            LoopbackAuditOutcome::Attempted,
        )?;
        let (result, mut stream) = receive_inner(self, timeout);
        let result = finish(
            audit,
            &provider,
            trace,
            LoopbackAuditAction::CallbackReceive,
            result,
        );
        if let Some(stream) = stream.as_mut() {
            if stream
                .set_write_timeout(Some(Duration::from_secs(1)))
                .is_ok()
            {
                let _ = stream.write_all(if result.is_ok() {
                    SUCCESS_RESPONSE
                } else {
                    FAILURE_RESPONSE
                });
                let _ = stream.flush();
            }
        }
        result
    }
}

impl Debug for LoopbackHost {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LoopbackHost")
            .field("provider", &self.provider)
            .field("trace", &self.trace)
            .field("address", &self.address)
            .field("port", &self.port)
            .field("callback_path", &"<redacted>")
            .field("redirect_uri", &"<redacted>")
            .finish()
    }
}

/// Validated callback URI held in wipe-on-drop storage.
pub struct LoopbackCallback(Zeroizing<String>);

impl LoopbackCallback {
    /// Borrow the callback only for immediate OAuth-core validation.
    pub fn as_uri(&self) -> &str {
        self.0.as_str()
    }
}

impl Debug for LoopbackCallback {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("LoopbackCallback(<redacted>)")
    }
}

fn bind_inner(
    provider: ProviderId,
    trace: OAuthTraceId,
    address: LoopbackAddress,
    port: u16,
    callback_path: String,
) -> Result<LoopbackHost, LoopbackHostError> {
    validate_callback_path(&callback_path)?;
    let listener =
        TcpListener::bind(address.socket_address(port)).map_err(|_| LoopbackHostError::Bind)?;
    listener
        .set_nonblocking(true)
        .map_err(|_| LoopbackHostError::Bind)?;
    let port = listener
        .local_addr()
        .map_err(|_| LoopbackHostError::Bind)?
        .port();
    if port == 0 {
        return Err(LoopbackHostError::Bind);
    }
    let expected_host = format!("{}:{port}", address.uri_host());
    let redirect_uri = format!("http://{expected_host}{callback_path}");
    Ok(LoopbackHost {
        listener,
        provider,
        trace,
        address,
        port,
        callback_path,
        redirect_uri,
        expected_host,
    })
}

fn receive_inner(
    host: LoopbackHost,
    timeout: Duration,
) -> (
    Result<LoopbackCallback, LoopbackHostError>,
    Option<TcpStream>,
) {
    if timeout.is_zero() || timeout > MAX_CALLBACK_WAIT {
        return (Err(LoopbackHostError::InvalidConfiguration), None);
    }
    let LoopbackHost {
        listener,
        callback_path,
        redirect_uri,
        expected_host,
        ..
    } = host;
    let deadline = match Instant::now().checked_add(timeout) {
        Some(deadline) => deadline,
        None => return (Err(LoopbackHostError::InvalidConfiguration), None),
    };
    let (mut stream, peer) = loop {
        match listener.accept() {
            Ok(pair) => break pair,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                let now = Instant::now();
                if now >= deadline {
                    return (Err(LoopbackHostError::Timeout), None);
                }
                thread::sleep(ACCEPT_POLL_INTERVAL.min(deadline.saturating_duration_since(now)));
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return (Err(LoopbackHostError::Protocol), None),
        }
    };
    drop(listener);

    if !peer.ip().is_loopback() {
        return (Err(LoopbackHostError::Protocol), Some(stream));
    }
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return (Err(LoopbackHostError::Timeout), Some(stream));
    }
    if stream.set_read_timeout(Some(remaining)).is_err() {
        return (Err(LoopbackHostError::Protocol), Some(stream));
    }
    let result =
        read_and_validate_callback(&mut stream, &callback_path, &redirect_uri, &expected_host);
    (result, Some(stream))
}

fn read_and_validate_callback(
    stream: &mut TcpStream,
    expected_path: &str,
    redirect_uri: &str,
    expected_host: &str,
) -> Result<LoopbackCallback, LoopbackHostError> {
    let mut head = Zeroizing::new(Vec::with_capacity(2 * 1024));
    let mut scratch = Zeroizing::new([0_u8; 1024]);
    loop {
        let read = match stream.read(&mut *scratch) {
            Ok(0) => return Err(LoopbackHostError::Protocol),
            Ok(read) => read,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Err(LoopbackHostError::Timeout)
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(LoopbackHostError::Protocol),
        };
        if head.len() + read > MAX_REQUEST_HEAD_BYTES {
            return Err(LoopbackHostError::Protocol);
        }
        head.extend_from_slice(&scratch[..read]);
        if let Some(end) = find_head_end(&head) {
            if end != head.len() {
                return Err(LoopbackHostError::Protocol);
            }
            return parse_callback_head(
                &head[..end - 4],
                expected_path,
                redirect_uri,
                expected_host,
            );
        }
    }
}

fn parse_callback_head(
    head: &[u8],
    expected_path: &str,
    redirect_uri: &str,
    expected_host: &str,
) -> Result<LoopbackCallback, LoopbackHostError> {
    let text = std::str::from_utf8(head).map_err(|_| LoopbackHostError::Protocol)?;
    if !text.is_ascii() {
        return Err(LoopbackHostError::Protocol);
    }
    let lines = text.split("\r\n").collect::<Vec<_>>();
    let (request_line, header_lines) = lines.split_first().ok_or(LoopbackHostError::Protocol)?;
    if request_line.is_empty()
        || request_line.len() > MAX_REQUEST_LINE_BYTES
        || request_line.contains('\r')
        || request_line.contains('\n')
        || header_lines.len() > MAX_HEADERS
    {
        return Err(LoopbackHostError::Protocol);
    }
    let mut parts = request_line.split(' ');
    let method = parts.next().ok_or(LoopbackHostError::Protocol)?;
    let target = parts.next().ok_or(LoopbackHostError::Protocol)?;
    let version = parts.next().ok_or(LoopbackHostError::Protocol)?;
    if method != "GET"
        || version != "HTTP/1.1"
        || parts.next().is_some()
        || target.is_empty()
        || !target.bytes().all(|byte| matches!(byte, 0x21..=0x7e))
        || target.contains('#')
    {
        return Err(LoopbackHostError::Protocol);
    }

    let mut names = Vec::with_capacity(header_lines.len());
    let mut host = None;
    for line in header_lines {
        if line.is_empty()
            || line.len() > MAX_HEADER_LINE_BYTES
            || line.contains('\r')
            || line.contains('\n')
        {
            return Err(LoopbackHostError::Protocol);
        }
        let (name, raw_value) = line.split_once(':').ok_or(LoopbackHostError::Protocol)?;
        if !valid_header_name(name)
            || names
                .iter()
                .any(|existing: &&str| existing.eq_ignore_ascii_case(name))
        {
            return Err(LoopbackHostError::Protocol);
        }
        names.push(name);
        let value = raw_value.trim_matches([' ', '\t']);
        if value
            .bytes()
            .any(|byte| !matches!(byte, b'\t' | 0x20..=0x7e))
        {
            return Err(LoopbackHostError::Protocol);
        }
        if name.eq_ignore_ascii_case("host") {
            host = Some(value);
        } else if name.eq_ignore_ascii_case("transfer-encoding")
            || (name.eq_ignore_ascii_case("content-length") && value != "0")
        {
            return Err(LoopbackHostError::Protocol);
        }
    }
    if host != Some(expected_host) {
        return Err(LoopbackHostError::Protocol);
    }
    let (path, query) = target.split_once('?').ok_or(LoopbackHostError::Protocol)?;
    if path != expected_path || query.is_empty() {
        return Err(LoopbackHostError::Protocol);
    }
    let mut callback = Zeroizing::new(String::with_capacity(redirect_uri.len() + 1 + query.len()));
    callback.push_str(redirect_uri);
    callback.push('?');
    callback.push_str(query);
    Ok(LoopbackCallback(callback))
}

fn find_head_end(input: &[u8]) -> Option<usize> {
    input
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn validate_callback_path(path: &str) -> Result<(), LoopbackHostError> {
    if path.is_empty()
        || path.len() > MAX_CALLBACK_PATH_BYTES
        || !path.starts_with('/')
        || path.starts_with("//")
        || !path.is_ascii()
    {
        return Err(LoopbackHostError::InvalidConfiguration);
    }
    let bytes = path.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return Err(LoopbackHostError::InvalidConfiguration);
            }
            index += 3;
            continue;
        }
        let allowed = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'/' | b'-'
                    | b'.'
                    | b'_'
                    | b'~'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
                    | b':'
                    | b'@'
            );
        if !allowed {
            return Err(LoopbackHostError::InvalidConfiguration);
        }
        index += 1;
    }
    Ok(())
}

fn valid_header_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

fn publish<S: LoopbackAuditSink>(
    audit: &mut S,
    provider: &ProviderId,
    trace: OAuthTraceId,
    action: LoopbackAuditAction,
    outcome: LoopbackAuditOutcome,
) -> Result<(), LoopbackHostError> {
    audit
        .publish(&LoopbackAuditEvent {
            provider: provider.clone(),
            trace,
            action,
            outcome,
        })
        .map_err(|_| LoopbackHostError::Audit)
}

fn finish<T, S: LoopbackAuditSink>(
    audit: &mut S,
    provider: &ProviderId,
    trace: OAuthTraceId,
    action: LoopbackAuditAction,
    result: Result<T, LoopbackHostError>,
) -> Result<T, LoopbackHostError> {
    let outcome = match result.as_ref() {
        Ok(_) => LoopbackAuditOutcome::Succeeded,
        Err(error) => match error.failure_class() {
            Some(class) => LoopbackAuditOutcome::Failed(class),
            None => return Err(LoopbackHostError::Audit),
        },
    };
    publish(audit, provider, trace, action, outcome)?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_oauth::{
        begin_authorization, EntropySource, OAuthAuditError, OAuthAuditEvent, OAuthAuditSink,
        OAuthError, ProviderConfig,
    };
    use std::net::Shutdown;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct RecordingHostAudit {
        events: Vec<LoopbackAuditEvent>,
        calls: usize,
        fail_on_call: Option<usize>,
        timeline: Option<Arc<Mutex<Vec<&'static str>>>>,
    }

    impl LoopbackAuditSink for RecordingHostAudit {
        fn publish(&mut self, event: &LoopbackAuditEvent) -> Result<(), LoopbackAuditError> {
            self.calls += 1;
            if self.fail_on_call == Some(self.calls) {
                return Err(LoopbackAuditError);
            }
            if let Some(timeline) = &self.timeline {
                timeline.lock().unwrap().push(match event.outcome() {
                    LoopbackAuditOutcome::Attempted => "audit-attempted",
                    LoopbackAuditOutcome::Succeeded => "audit-succeeded",
                    LoopbackAuditOutcome::Failed(_) => "audit-failed",
                });
            }
            self.events.push(event.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct CoreAudit;

    impl OAuthAuditSink for CoreAudit {
        fn publish(&mut self, _event: &OAuthAuditEvent) -> Result<(), OAuthAuditError> {
            Ok(())
        }
    }

    struct FixedEntropy(u8);

    impl EntropySource for FixedEntropy {
        fn fill(&mut self, destination: &mut [u8]) -> Result<(), OAuthError> {
            for (index, byte) in destination.iter_mut().enumerate() {
                *byte = self.0.wrapping_add(index as u8);
            }
            Ok(())
        }
    }

    struct RecordingUserAgent {
        urls: Vec<String>,
        fail: bool,
        timeline: Option<Arc<Mutex<Vec<&'static str>>>>,
    }

    impl ExternalUserAgent for RecordingUserAgent {
        fn open(&mut self, authorization_url: &str) -> Result<(), ExternalUserAgentError> {
            if let Some(timeline) = &self.timeline {
                timeline.lock().unwrap().push("browser-open");
            }
            self.urls.push(authorization_url.to_owned());
            if self.fail {
                Err(ExternalUserAgentError)
            } else {
                Ok(())
            }
        }
    }

    fn provider() -> ProviderId {
        ProviderId::new("fixture").unwrap()
    }

    fn trace() -> OAuthTraceId {
        OAuthTraceId::new([0x2a; 16])
    }

    fn bind(audit: &mut RecordingHostAudit) -> LoopbackHost {
        LoopbackHost::bind(
            provider(),
            trace(),
            LoopbackAddress::Ipv4,
            0,
            "/oauth/callback",
            audit,
        )
        .unwrap()
    }

    fn authorization_request(host: &LoopbackHost) -> AuthorizationRequest {
        let config = ProviderConfig::new(
            provider(),
            "https://authorize.example/oauth2/auth",
            "https://token.example/oauth2/token",
            "public-client",
            host.redirect_uri(),
        )
        .unwrap()
        .with_distinct_redirect_uri();
        begin_authorization(&config, &["vault.read"], trace(), &mut FixedEntropy(3))
            .publish_then_release(&mut CoreAudit)
            .unwrap()
    }

    fn send_request(port: u16, request: String) -> thread::JoinHandle<Vec<u8>> {
        thread::spawn(move || {
            let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
            stream.write_all(request.as_bytes()).unwrap();
            stream.shutdown(Shutdown::Write).unwrap();
            let mut response = Vec::new();
            stream.read_to_end(&mut response).unwrap();
            response
        })
    }

    #[test]
    fn bind_is_audited_before_and_after_effect() {
        let mut audit = RecordingHostAudit::default();
        let host = bind(&mut audit);
        assert_eq!(host.provider(), &provider());
        assert_eq!(host.trace(), trace());
        assert_eq!(host.address(), LoopbackAddress::Ipv4);
        assert_ne!(host.port(), 0);
        assert_eq!(
            host.redirect_uri(),
            format!("http://127.0.0.1:{}/oauth/callback", host.port())
        );
        assert_eq!(audit.events.len(), 2);
        assert_eq!(audit.events[0].action(), LoopbackAuditAction::Bind);
        assert_eq!(audit.events[0].outcome(), LoopbackAuditOutcome::Attempted);
        assert_eq!(audit.events[1].outcome(), LoopbackAuditOutcome::Succeeded);
    }

    #[test]
    fn bind_validation_and_audit_failures_fail_closed() {
        let mut invalid_audit = RecordingHostAudit::default();
        let invalid = LoopbackHost::bind(
            provider(),
            trace(),
            LoopbackAddress::Ipv4,
            0,
            "//authority",
            &mut invalid_audit,
        );
        assert!(matches!(
            invalid,
            Err(LoopbackHostError::InvalidConfiguration)
        ));
        assert_eq!(
            invalid_audit.events[1].outcome(),
            LoopbackAuditOutcome::Failed(LoopbackFailureClass::InvalidInput)
        );

        let mut pre_audit_failure = RecordingHostAudit {
            fail_on_call: Some(1),
            ..RecordingHostAudit::default()
        };
        assert!(matches!(
            LoopbackHost::bind(
                provider(),
                trace(),
                LoopbackAddress::Ipv4,
                0,
                "/callback",
                &mut pre_audit_failure,
            ),
            Err(LoopbackHostError::Audit)
        ));

        let mut post_audit_failure = RecordingHostAudit {
            fail_on_call: Some(2),
            ..RecordingHostAudit::default()
        };
        let mut probe = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        assert!(matches!(
            LoopbackHost::bind(
                provider(),
                trace(),
                LoopbackAddress::Ipv4,
                port,
                "/callback",
                &mut post_audit_failure,
            ),
            Err(LoopbackHostError::Audit)
        ));
        probe = TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap();
        drop(probe);
    }

    #[test]
    fn browser_open_is_request_bound_and_bracketed_by_audit() {
        let timeline = Arc::new(Mutex::new(Vec::new()));
        let mut bind_audit = RecordingHostAudit::default();
        let host = bind(&mut bind_audit);
        let request = authorization_request(&host);
        let mut audit = RecordingHostAudit {
            timeline: Some(Arc::clone(&timeline)),
            ..RecordingHostAudit::default()
        };
        let mut user_agent = RecordingUserAgent {
            urls: Vec::new(),
            fail: false,
            timeline: Some(Arc::clone(&timeline)),
        };
        host.open_browser(&request, &mut user_agent, &mut audit)
            .unwrap();
        assert_eq!(user_agent.urls, [request.url().as_str()]);
        assert_eq!(
            *timeline.lock().unwrap(),
            ["audit-attempted", "browser-open", "audit-succeeded"]
        );
    }

    #[test]
    fn browser_binding_mismatch_and_adapter_failure_are_closed() {
        let mut bind_audit = RecordingHostAudit::default();
        let host = bind(&mut bind_audit);
        let other_config = ProviderConfig::new(
            ProviderId::new("other").unwrap(),
            "https://authorize.example/auth",
            "https://token.example/token",
            "client",
            host.redirect_uri(),
        )
        .unwrap()
        .with_distinct_redirect_uri();
        let other = begin_authorization(
            &other_config,
            &["vault.read"],
            trace(),
            &mut FixedEntropy(9),
        )
        .publish_then_release(&mut CoreAudit)
        .unwrap();
        let mut user_agent = RecordingUserAgent {
            urls: Vec::new(),
            fail: false,
            timeline: None,
        };
        let mut audit = RecordingHostAudit::default();
        assert_eq!(
            host.open_browser(&other, &mut user_agent, &mut audit),
            Err(LoopbackHostError::InvalidConfiguration)
        );
        assert!(user_agent.urls.is_empty());
        assert_eq!(
            audit.events[1].outcome(),
            LoopbackAuditOutcome::Failed(LoopbackFailureClass::InvalidInput)
        );

        user_agent.fail = true;
        assert_eq!(
            host.open_browser(&authorization_request(&host), &mut user_agent, &mut audit),
            Err(LoopbackHostError::Browser)
        );
        assert_eq!(
            audit.events[3].outcome(),
            LoopbackAuditOutcome::Failed(LoopbackFailureClass::Browser)
        );
    }

    #[test]
    fn valid_callback_is_redacted_audited_and_releases_listener() {
        let mut bind_audit = RecordingHostAudit::default();
        let host = bind(&mut bind_audit);
        let port = host.port();
        let query = "code=top-secret-code&state=top-secret-state";
        let client = send_request(
            port,
            format!(
                "GET /oauth/callback?{query} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: 0\r\n\r\n"
            ),
        );
        let mut audit = RecordingHostAudit::default();
        let callback = host
            .receive_callback(Duration::from_secs(2), &mut audit)
            .unwrap();
        let response = client.join().unwrap();
        assert_eq!(
            callback.as_uri(),
            format!("http://127.0.0.1:{port}/oauth/callback?{query}")
        );
        assert_eq!(format!("{callback:?}"), "LoopbackCallback(<redacted>)");
        assert!(!format!("{callback:?}").contains("top-secret"));
        assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert!(response.ends_with(
            b"<!doctype html><title>Authorization complete</title>You may close this tab."
        ));
        assert_eq!(audit.events.len(), 2);
        assert_eq!(audit.events[0].outcome(), LoopbackAuditOutcome::Attempted);
        assert_eq!(audit.events[1].outcome(), LoopbackAuditOutcome::Succeeded);
        drop(TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap());
    }

    #[test]
    fn malformed_first_connection_consumes_listener_and_is_audited() {
        let mut bind_audit = RecordingHostAudit::default();
        let host = bind(&mut bind_audit);
        let port = host.port();
        let client = send_request(
            port,
            format!("POST /oauth/callback?code=x HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"),
        );
        let mut audit = RecordingHostAudit::default();
        assert!(matches!(
            host.receive_callback(Duration::from_secs(2), &mut audit),
            Err(LoopbackHostError::Protocol)
        ));
        let response = client.join().unwrap();
        assert!(response.starts_with(b"HTTP/1.1 400 Bad Request\r\n"));
        assert_eq!(
            audit.events[1].outcome(),
            LoopbackAuditOutcome::Failed(LoopbackFailureClass::Protocol)
        );
        drop(TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap());
    }

    #[test]
    fn timeout_is_bounded_and_audited() {
        let mut bind_audit = RecordingHostAudit::default();
        let host = bind(&mut bind_audit);
        let mut audit = RecordingHostAudit::default();
        let started = Instant::now();
        assert!(matches!(
            host.receive_callback(Duration::from_millis(20), &mut audit),
            Err(LoopbackHostError::Timeout)
        ));
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(
            audit.events[1].outcome(),
            LoopbackAuditOutcome::Failed(LoopbackFailureClass::Timeout)
        );
    }

    #[test]
    fn callback_result_audit_failure_withholds_callback_and_browser_success() {
        let mut bind_audit = RecordingHostAudit::default();
        let host = bind(&mut bind_audit);
        let port = host.port();
        let client = send_request(
            port,
            format!("GET /oauth/callback?code=secret HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n"),
        );
        let mut audit = RecordingHostAudit {
            fail_on_call: Some(2),
            ..RecordingHostAudit::default()
        };
        assert!(matches!(
            host.receive_callback(Duration::from_secs(2), &mut audit),
            Err(LoopbackHostError::Audit)
        ));
        assert!(client
            .join()
            .unwrap()
            .starts_with(b"HTTP/1.1 400 Bad Request\r\n"));
        drop(TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap());
    }

    #[test]
    fn strict_http_profile_rejects_ambiguous_requests() {
        let port = 49152;
        let expected_host = format!("127.0.0.1:{port}");
        let redirect = format!("http://{expected_host}/callback");
        let valid = format!("GET /callback?code=x HTTP/1.1\r\nHost: {expected_host}");
        assert_eq!(
            parse_callback_head(valid.as_bytes(), "/callback", &redirect, &expected_host)
                .unwrap()
                .as_uri(),
            format!("{redirect}?code=x")
        );
        let invalid = [
            format!("POST /callback?code=x HTTP/1.1\r\nHost: {expected_host}"),
            format!("GET /callback?code=x HTTP/1.0\r\nHost: {expected_host}"),
            "GET /callback?code=x HTTP/1.1\r\nHost: attacker.example".to_owned(),
            format!(
                "GET /callback?code=x HTTP/1.1\r\nHost: {expected_host}\r\nhost: {expected_host}"
            ),
            format!(
                "GET /callback?code=x HTTP/1.1\r\nHost: {expected_host}\r\nTransfer-Encoding: chunked"
            ),
            format!(
                "GET /callback?code=x HTTP/1.1\r\nHost: {expected_host}\r\nContent-Length: 1"
            ),
            format!("GET /wrong?code=x HTTP/1.1\r\nHost: {expected_host}"),
            format!("GET /callback HTTP/1.1\r\nHost: {expected_host}"),
            format!("GET /callback?code=x#fragment HTTP/1.1\r\nHost: {expected_host}"),
            format!("GET  /callback?code=x HTTP/1.1\r\nHost: {expected_host}"),
        ];
        for request in invalid {
            assert!(
                matches!(
                    parse_callback_head(request.as_bytes(), "/callback", &redirect, &expected_host),
                    Err(LoopbackHostError::Protocol)
                ),
                "accepted: {request:?}"
            );
        }
    }

    #[test]
    fn callback_path_validation_is_bounded_and_unambiguous() {
        for valid in ["/", "/oauth/callback", "/oauth/%63allback", "/a:b@c"] {
            assert_eq!(validate_callback_path(valid), Ok(()), "rejected: {valid}");
        }
        for invalid in [
            "",
            "callback",
            "//host/path",
            "/callback?query",
            "/callback#fragment",
            "/bad%2",
            "/bad%xx",
            "/bad path",
        ] {
            assert_eq!(
                validate_callback_path(invalid),
                Err(LoopbackHostError::InvalidConfiguration),
                "accepted: {invalid}"
            );
        }
        assert_eq!(
            validate_callback_path(&format!("/{}", "a".repeat(MAX_CALLBACK_PATH_BYTES))),
            Err(LoopbackHostError::InvalidConfiguration)
        );
    }

    #[test]
    fn audit_and_debug_surfaces_never_contain_callback_or_browser_secrets() {
        let mut audit = RecordingHostAudit::default();
        let host = bind(&mut audit);
        let request = authorization_request(&host);
        let debug = format!("{host:?} {request:?} {:?}", audit.events);
        assert!(!debug.contains("code_challenge="));
        assert!(!debug.contains("state="));
        assert!(!debug.contains("/oauth/callback"));
        assert!(debug.contains("<redacted>"));
    }
}
