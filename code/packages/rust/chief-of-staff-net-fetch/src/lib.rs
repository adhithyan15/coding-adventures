//! # `net.fetch` — the host-mediated network operation (D18V)
//!
//! D18's leased mode ends with a step that, until this crate, nothing
//! implemented:
//!
//! ```text
//! Agent receives { vault_ref, expires_at_ms }
//! Agent passes vault_ref to an approved host operation   <-- this crate
//! Host atomically consumes the lease
//! ```
//!
//! `net.fetch` is that operation. A model asks for a URL; the daemon decides
//! whether this host may reach it, does the network work itself, and hands
//! back a bounded, masked response. When a credential is involved the model
//! only ever holds an opaque lease reference — the secret is redeemed inside
//! the daemon, written into one request header, and masked out of whatever
//! comes back.
//!
//! ## The pipeline, in the order it runs
//!
//! ```text
//!   arguments (JSON from the model)
//!        │  parse_request ............ V-R4 header allowlist, bounds, no dupes
//!        ▼
//!   FetchRequest
//!        │  NetAllowlist::authorize .. V-A1 exact net:dns + net:connect
//!        ▼
//!   Target { host, port, path }
//!        │  Resolver + is_public ..... V-R2 every address public, keep one
//!        ▼
//!   SocketAddr (checked)
//!        │  CredentialSource::redeem . V-S3/S7 only now is a lease consumed (V-S6)
//!        ▼
//!   request bytes (zeroizing) ........ V-S2 secret in exactly one header
//!        │  Transport::exchange ...... V-R1 TLS to the CHECKED address
//!        ▼
//!   raw response (zeroizing)
//!        │  mask_echoes .............. V-S4 every echo masked IN PLACE, first
//!        │  decode_response .......... V-R5..R7 bounds, header allowlist, UTF-8
//!        ▼
//!   FetchResponse → JSON for the model
//! ```
//!
//! Every step that can refuse runs **before** the lease is consumed, so a
//! refused URL or a private address never burns the caller's credential.
//!
//! ## Why the address check lives here and not in the TLS layer
//!
//! The allowlist names *hosts*. A host is just a DNS name, and DNS answers
//! change. If the check resolved the name and the connector then resolved it
//! again, an attacker controlling an allowed domain's DNS could answer
//! "203.0.113.7" to the first lookup and "127.0.0.1" to the second — and
//! `127.0.0.1` is where the daemon's own control API listens. So the name is
//! resolved exactly once, every answer is checked, and the connection is made
//! to the checked address with the host kept only for SNI and certificate
//! verification (`tls-platform`'s `connect_addr`).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::time::Duration;

use chief_of_staff_agent_manifest::Capability;
use chief_of_staff_tool_api::{
    JsonSchema, PrivilegeTier, SchemaProperty, ToolConcurrency, ToolDefinition, ToolIdempotency,
    ToolSideEffects, ToolStability, ToolStreaming,
};
use coding_adventures_json_value::{JsonNumber, JsonValue};
use coding_adventures_zeroize::Zeroizing;
use http_core::BodyKind;
use operation_primitives::OperationHttpClient;

// ── Constants (D18V V-R5 and friends) ────────────────────────────────────────

/// The D18D tool id.
pub const NET_FETCH_TOOL_ID: &str = "net.fetch";

/// Longest accepted URL, in bytes.
pub const MAX_URL_BYTES: usize = 2048;
/// Most model-supplied request headers.
pub const MAX_REQUEST_HEADERS: usize = 16;
/// Longest model-supplied request header value, in bytes.
pub const MAX_HEADER_VALUE_BYTES: usize = 1024;
/// Largest `POST` body, in bytes.
pub const MAX_REQUEST_BODY_BYTES: usize = 64 * 1024;
/// Largest response head, in bytes.
pub const MAX_RESPONSE_HEAD_BYTES: usize = 32 * 1024;
/// Largest response body returned to the model, in bytes.
pub const MAX_RESPONSE_BODY_BYTES: usize = 256 * 1024;
/// Shortest secret that may be injected (V-S2). Below this, masking the bare
/// secret from a response would redact ordinary text.
pub const MIN_SECRET_BYTES: usize = 8;
/// Longest secret that may be injected (V-S2).
pub const MAX_SECRET_BYTES: usize = 4096;

/// Total bytes read from the wire: head, body, and slack for chunk framing.
/// Reading one byte past what a valid response could need is how truncation
/// is detected rather than silently assumed.
const MAX_WIRE_BYTES: usize = MAX_RESPONSE_HEAD_BYTES + MAX_RESPONSE_BODY_BYTES + 16 * 1024;

const DEFAULT_HTTPS_PORT: u16 = 443;
const DEFAULT_USER_AGENT: &str = "chief-of-staff-net-fetch/0.1";

/// Request headers a model may set (V-R4). The daemon owns every other one.
pub const ALLOWED_REQUEST_HEADERS: &[&str] = &[
    "accept",
    "accept-language",
    "content-type",
    "user-agent",
    "if-none-match",
    "if-modified-since",
];

/// Response headers a model may see (V-R6). `set-cookie` is the reason this
/// is an allowlist and not a denylist.
pub const ALLOWED_RESPONSE_HEADERS: &[&str] = &[
    "content-type",
    "content-length",
    "location",
    "retry-after",
    "date",
    "etag",
    "last-modified",
    "cache-control",
];

// ── Errors ───────────────────────────────────────────────────────────────────

/// A bounded, secret-free failure (D18V "Output").
///
/// Variants carry no text from the request, the response, or the network:
/// a URL can carry identifiers, a response can carry anything, and an error
/// is the path most likely to be logged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchError {
    /// The URL is not in the host's signed manifest.
    Unauthorized,
    /// The arguments violate the tool's contract; names the broken rule.
    InvalidRequest(&'static str),
    /// The credential could not be redeemed or used.
    CredentialRefused,
    /// The host resolved to a non-public address (or to nothing).
    AddressRefused,
    /// A socket-level failure.
    Network,
    /// TLS failed: handshake, verification, or server name.
    Tls,
    /// The response was not well-formed HTTP/1.
    Protocol,
    /// The body was not UTF-8 (V-R7).
    BinaryBody,
    /// A connect, handshake, read, or write timed out.
    Timeout,
}

impl FetchError {
    /// The stable error kind reported to the model.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Unauthorized => "unauthorized",
            Self::InvalidRequest(_) => "invalid_request",
            Self::CredentialRefused => "credential_refused",
            Self::AddressRefused => "address_refused",
            Self::Network => "network",
            Self::Tls => "tls",
            Self::Protocol => "protocol",
            Self::BinaryBody => "binary_body",
            Self::Timeout => "timeout",
        }
    }
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(why) => write!(f, "net.fetch invalid_request: {why}"),
            other => write!(f, "net.fetch {}", other.kind()),
        }
    }
}

impl std::error::Error for FetchError {}

// ── The request ──────────────────────────────────────────────────────────────

/// `GET` or `POST`, the only methods D18V admits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// `GET`.
    Get,
    /// `POST`, with an optional body.
    Post,
}

impl Method {
    fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

/// The one header a credential may be written into (V-S2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialHeader {
    /// `authorization`.
    Authorization,
    /// `x-api-key`.
    XApiKey,
    /// `x-auth-token`.
    XAuthToken,
}

impl CredentialHeader {
    fn as_str(self) -> &'static str {
        match self {
            Self::Authorization => "authorization",
            Self::XApiKey => "x-api-key",
            Self::XAuthToken => "x-auth-token",
        }
    }
}

/// The optional scheme word placed before the secret (V-S2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialScheme {
    /// `Bearer <secret>`.
    Bearer,
    /// `Basic <secret>`.
    Basic,
    /// `Token <secret>`.
    Token,
}

impl CredentialScheme {
    fn as_str(self) -> &'static str {
        match self {
            Self::Bearer => "Bearer",
            Self::Basic => "Basic",
            Self::Token => "Token",
        }
    }
}

/// Where a leased secret goes, and which lease it is.
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialSpec {
    /// The opaque receipt from `vault.request_lease`. A bearer reference, so
    /// it is redacted from `Debug`.
    pub vault_ref: String,
    /// The header that receives the secret.
    pub header: CredentialHeader,
    /// An optional scheme word.
    pub scheme: Option<CredentialScheme>,
}

impl fmt::Debug for CredentialSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CredentialSpec")
            .field("vault_ref", &"<redacted>")
            .field("header", &self.header)
            .field("scheme", &self.scheme)
            .finish()
    }
}

/// A validated `net.fetch` call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchRequest {
    /// The absolute `https://` URL.
    pub url: String,
    /// The method.
    pub method: Method,
    /// Model-supplied headers, lowercased, all from [`ALLOWED_REQUEST_HEADERS`].
    pub headers: Vec<(String, String)>,
    /// The `POST` body.
    pub body: Option<String>,
    /// The leased credential, if any.
    pub credential: Option<CredentialSpec>,
}

/// Validate the model's arguments into a [`FetchRequest`].
///
/// Duplicate keys are refused at every level. `JsonValue::Object` keeps them,
/// and a duplicate is how two components end up reading two different values
/// from "the same" field.
pub fn parse_request(arguments: &JsonValue) -> Result<FetchRequest, FetchError> {
    let top = object(arguments, "arguments must be an object")?;
    reject_unknown(
        top,
        &["url", "method", "headers", "body", "credential"],
        "unknown argument",
    )?;

    let url = required_string(top, "url")?;
    if url.len() > MAX_URL_BYTES {
        return Err(FetchError::InvalidRequest("url is longer than 2048 bytes"));
    }
    let method = match required_string(top, "method")? {
        "GET" => Method::Get,
        "POST" => Method::Post,
        _ => return Err(FetchError::InvalidRequest("method must be GET or POST")),
    };

    let mut headers = Vec::new();
    if let Some(value) = optional(top, "headers")? {
        let entries = object(value, "headers must be an object")?;
        if entries.len() > MAX_REQUEST_HEADERS {
            return Err(FetchError::InvalidRequest("more than 16 headers"));
        }
        for (name, value) in entries {
            let name = name.to_ascii_lowercase();
            if !ALLOWED_REQUEST_HEADERS.contains(&name.as_str()) {
                return Err(FetchError::InvalidRequest("header is not allowed"));
            }
            if headers.iter().any(|(existing, _)| existing == &name) {
                return Err(FetchError::InvalidRequest("duplicate header"));
            }
            let JsonValue::String(value) = value else {
                return Err(FetchError::InvalidRequest("header values must be strings"));
            };
            validate_header_value(value, MAX_HEADER_VALUE_BYTES)?;
            headers.push((name, value.clone()));
        }
    }

    let body = match optional(top, "body")? {
        None => None,
        Some(JsonValue::String(body)) => {
            if method != Method::Post {
                return Err(FetchError::InvalidRequest("only POST may carry a body"));
            }
            if body.len() > MAX_REQUEST_BODY_BYTES {
                return Err(FetchError::InvalidRequest("body is larger than 64 KiB"));
            }
            Some(body.clone())
        }
        Some(_) => return Err(FetchError::InvalidRequest("body must be a string")),
    };

    let credential = match optional(top, "credential")? {
        None => None,
        Some(value) => Some(parse_credential(value)?),
    };

    Ok(FetchRequest {
        url: url.to_string(),
        method,
        headers,
        body,
        credential,
    })
}

fn parse_credential(value: &JsonValue) -> Result<CredentialSpec, FetchError> {
    let fields = object(value, "credential must be an object")?;
    reject_unknown(
        fields,
        &["vault_ref", "header", "scheme"],
        "unknown credential field",
    )?;
    let vault_ref = required_string(fields, "vault_ref")?;
    if vault_ref.is_empty() || vault_ref.len() > 512 {
        return Err(FetchError::InvalidRequest(
            "vault_ref length is out of range",
        ));
    }
    let header = match required_string(fields, "header")? {
        "authorization" => CredentialHeader::Authorization,
        "x-api-key" => CredentialHeader::XApiKey,
        "x-auth-token" => CredentialHeader::XAuthToken,
        _ => {
            return Err(FetchError::InvalidRequest(
                "credential header must be authorization, x-api-key, or x-auth-token",
            ))
        }
    };
    let scheme = match optional(fields, "scheme")? {
        None => None,
        Some(JsonValue::String(scheme)) => Some(match scheme.as_str() {
            "Bearer" => CredentialScheme::Bearer,
            "Basic" => CredentialScheme::Basic,
            "Token" => CredentialScheme::Token,
            _ => {
                return Err(FetchError::InvalidRequest(
                    "scheme must be Bearer, Basic, or Token",
                ))
            }
        }),
        Some(_) => return Err(FetchError::InvalidRequest("scheme must be a string")),
    };
    Ok(CredentialSpec {
        vault_ref: vault_ref.to_string(),
        header,
        scheme,
    })
}

/// CR, LF and NUL are how one header becomes two (request smuggling); other
/// control characters have no business in a header either.
fn validate_header_value(value: &str, limit: usize) -> Result<(), FetchError> {
    if value.len() > limit {
        return Err(FetchError::InvalidRequest("header value is too long"));
    }
    if value.bytes().any(|b| b.is_ascii_control() && b != b'\t') {
        return Err(FetchError::InvalidRequest(
            "header value contains a control character",
        ));
    }
    Ok(())
}

type Fields = [(String, JsonValue)];

fn object<'a>(value: &'a JsonValue, why: &'static str) -> Result<&'a Fields, FetchError> {
    let JsonValue::Object(fields) = value else {
        return Err(FetchError::InvalidRequest(why));
    };
    // Bound before the quadratic duplicate scan below.
    if fields.len() > 32 {
        return Err(FetchError::InvalidRequest("too many fields"));
    }
    for (index, (key, _)) in fields.iter().enumerate() {
        if fields[..index].iter().any(|(earlier, _)| earlier == key) {
            return Err(FetchError::InvalidRequest("duplicate key"));
        }
    }
    Ok(fields)
}

fn reject_unknown(fields: &Fields, known: &[&str], why: &'static str) -> Result<(), FetchError> {
    if fields.iter().any(|(key, _)| !known.contains(&key.as_str())) {
        return Err(FetchError::InvalidRequest(why));
    }
    Ok(())
}

fn optional<'a>(fields: &'a Fields, name: &str) -> Result<Option<&'a JsonValue>, FetchError> {
    Ok(fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .filter(|value| !matches!(value, JsonValue::Null)))
}

fn required_string<'a>(fields: &'a Fields, name: &'static str) -> Result<&'a str, FetchError> {
    match optional(fields, name)? {
        Some(JsonValue::String(value)) => Ok(value),
        Some(_) => Err(FetchError::InvalidRequest("expected a string")),
        None => Err(FetchError::InvalidRequest("a required argument is missing")),
    }
}

// ── Authorization (V-A1) ─────────────────────────────────────────────────────

/// Where an authorized request goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// The lowercased DNS host, also the TLS server name.
    pub host: String,
    /// The TCP port.
    pub port: u16,
    /// The origin-form request target (`/path?query`).
    pub path_and_query: String,
}

/// The `net` slice of one host's signed manifest.
///
/// Built only from capabilities that came out of a verified package (V-A2 is
/// the daemon's job); this type trusts what it is given and enforces it
/// exactly, through `operation-primitives`' allowlist rather than a second
/// implementation of the same matching rules.
#[derive(Clone, Debug)]
pub struct NetAllowlist {
    client: OperationHttpClient,
}

impl NetAllowlist {
    /// Build the allowlist, or `None` when the manifest grants no network at
    /// all (V-A3: such a host is not offered the tool).
    ///
    /// A malformed `net` target in a signed manifest is an error, not a
    /// silently skipped entry.
    pub fn from_capabilities(capabilities: &[Capability]) -> Result<Option<Self>, FetchError> {
        let mut dns = Vec::new();
        let mut connect = Vec::new();
        for capability in capabilities.iter().filter(|c| c.category == "net") {
            match capability.action.as_str() {
                "dns" => dns.push(capability.target.as_str()),
                "connect" => connect.push(capability.target.as_str()),
                _ => {}
            }
        }
        if connect.is_empty() {
            return Ok(None);
        }
        OperationHttpClient::from_compiled_allowlist(&dns, &connect)
            .map(|client| Some(Self { client }))
            .map_err(|_| {
                FetchError::InvalidRequest("the signed manifest has a malformed net target")
            })
    }

    /// Check a URL against the manifest (V-A1), and refuse IP-literal hosts.
    pub fn authorize(&self, url: &str) -> Result<Target, FetchError> {
        let preflight = self
            .client
            .preflight_get(url)
            .map_err(|_| FetchError::Unauthorized)?;
        // The allowlist accepts any exact name, including one that happens to
        // be an IP literal. D18V requires a DNS name: an address skips the
        // resolution step the public-address check hangs off.
        if preflight.host().parse::<IpAddr>().is_ok() {
            return Err(FetchError::Unauthorized);
        }
        let path_and_query = if preflight.path_and_query().is_empty() {
            "/".to_string()
        } else {
            preflight.path_and_query().to_string()
        };
        Ok(Target {
            host: preflight.host().to_string(),
            port: preflight.port(),
            path_and_query,
        })
    }
}

// ── Addresses (V-R2) ─────────────────────────────────────────────────────────

/// Is this an address `net.fetch` may connect to?
///
/// The refusals, and why each one matters to a daemon that also listens
/// locally:
///
/// | range | why refused |
/// |---|---|
/// | loopback `127/8`, `::1` | the daemon's own control API |
/// | private `10/8` `172.16/12` `192.168/16`, ULA `fc00::/7` | the owner's LAN |
/// | link-local `169.254/16`, `fe80::/10` | cloud metadata endpoints |
/// | CGNAT `100.64/10` | carrier-internal |
/// | unspecified, broadcast, multicast | not a server |
/// | documentation, benchmarking, reserved | not a server |
/// | IPv4-mapped/compatible IPv6 | the v4 rules, smuggled through v6 |
pub fn is_public(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_public_v4(v4);
            }
            is_public_v6(v6)
        }
    }
}

fn is_public_v4(v4: Ipv4Addr) -> bool {
    let [a, b, c, _] = v4.octets();
    !(v4.is_loopback()
        || v4.is_private()
        || v4.is_link_local()
        || v4.is_unspecified()
        || v4.is_broadcast()
        || v4.is_multicast()
        || v4.is_documentation()
        || a == 0                                   // "this network"
        || (a == 100 && (64..=127).contains(&b))    // CGNAT
        || (a == 192 && b == 0 && c == 0)           // IETF protocol assignments
        || (a == 198 && (b == 18 || b == 19))       // benchmarking
        || a >= 240) // reserved
}

/// IPv6 is an allowlist, not a denylist: only global unicast `2000::/3`,
/// minus the ranges inside it that embed or translate an IPv4 address or are
/// not servers. A denylist missed NAT64 `64:ff9b::/96` the first time round,
/// which on a DNS64 network reaches RFC 1918 hosts — so everything outside
/// `2000::/3` (NAT64, IPv4-compatible and -translated forms, discard `100::/64`,
/// ULA, link-local, multicast, loopback) is refused by construction.
///
/// | refused inside `2000::/3` | why |
/// |---|---|
/// | `2001::/32` Teredo | tunnels to an embedded IPv4 address |
/// | `2001:2::/48` | benchmarking |
/// | `2001:10::/28`, `2001:20::/28` | ORCHID identifiers, not addresses |
/// | `2001:db8::/32` | documentation |
/// | `2002::/16` 6to4 | tunnels to an embedded IPv4 address |
fn is_public_v6(v6: Ipv6Addr) -> bool {
    let s = v6.segments();
    let global_unicast = (s[0] & 0xe000) == 0x2000;
    global_unicast
        && !(s[0] == 0x2001 && s[1] == 0x0000)            // Teredo
        && !(s[0] == 0x2001 && s[1] == 0x0002 && s[2] == 0) // benchmarking
        && !(s[0] == 0x2001 && (s[1] & 0xfff0) == 0x0010)  // ORCHID
        && !(s[0] == 0x2001 && (s[1] & 0xfff0) == 0x0020)  // ORCHIDv2
        && !(s[0] == 0x2001 && s[1] == 0x0db8)            // documentation
        && s[0] != 0x2002 // 6to4
}

/// Resolves a host name. A trait so tests never touch DNS.
pub trait Resolver: Send + Sync {
    /// Every address `host` resolves to on `port`.
    fn resolve(&self, host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>>;
}

/// The operating system's resolver.
pub struct SystemResolver;

impl Resolver for SystemResolver {
    fn resolve(&self, host: &str, port: u16) -> std::io::Result<Vec<SocketAddr>> {
        Ok((host, port).to_socket_addrs()?.collect())
    }
}

/// Resolve and require that **every** answer is public, then pick the first.
///
/// "Every", not "any": a name that answers with one public and one private
/// address would otherwise depend on which one a later step happened to pick.
fn checked_address(resolver: &dyn Resolver, target: &Target) -> Result<SocketAddr, FetchError> {
    let addresses = resolver
        .resolve(&target.host, target.port)
        .map_err(|_| FetchError::Network)?;
    let first = *addresses.first().ok_or(FetchError::AddressRefused)?;
    if addresses.iter().all(|address| is_public(address.ip())) {
        Ok(first)
    } else {
        Err(FetchError::AddressRefused)
    }
}

// ── Credentials (V-S1..S3) ───────────────────────────────────────────────────

/// Redeems a lease into secret bytes.
///
/// Implemented by the daemon over the vault runtime, which knows two things
/// this crate never sees, and must refuse **without consuming the lease**
/// unless both hold:
///
/// - **V-S3** the lease was issued to the calling host;
/// - **V-S7** the secret is allowed to be sent to `destination` (`host:port`).
///
/// V-S7 exists because a manifest can name several hosts: without it, a key
/// minted for one API could be written into a request to another the same
/// agent may reach, where it could be logged or reflected in a form the
/// response mask does not recognize.
pub trait CredentialSource {
    /// Atomically consume `vault_ref` for a request to `destination`.
    fn redeem(&self, vault_ref: &str, destination: &str) -> Result<Zeroizing<Vec<u8>>, FetchError>;
}

/// The header value a secret becomes: `scheme + " " + secret`, or the secret.
fn credential_value(
    spec: &CredentialSpec,
    secret: &[u8],
) -> Result<Zeroizing<Vec<u8>>, FetchError> {
    if secret.len() < MIN_SECRET_BYTES || secret.len() > MAX_SECRET_BYTES {
        return Err(FetchError::CredentialRefused);
    }
    // Printable ASCII only: anything else is either a header-smuggling
    // attempt (CR/LF) or not something an HTTP header can carry.
    if !secret.iter().all(|b| (0x21..=0x7e).contains(b)) {
        return Err(FetchError::CredentialRefused);
    }
    let prefix = spec.scheme.map(CredentialScheme::as_str);
    let length = prefix.map_or(0, |p| p.len() + 1) + secret.len();
    let mut value = Zeroizing::new(Vec::with_capacity(length));
    if let Some(prefix) = prefix {
        value.extend_from_slice(prefix.as_bytes());
        value.push(b' ');
    }
    value.extend_from_slice(secret);
    Ok(value)
}

// ── The wire (V-R4, V-S2, V-S5) ──────────────────────────────────────────────

/// Serialize the HTTP/1.1 request. Allocated once at its exact final size,
/// because when a credential is present this buffer holds the secret.
fn encode_request(
    request: &FetchRequest,
    target: &Target,
    credential: Option<(CredentialHeader, &[u8])>,
) -> Zeroizing<Vec<u8>> {
    let host_header = if target.port == DEFAULT_HTTPS_PORT {
        target.host.clone()
    } else {
        format!("{}:{}", target.host, target.port)
    };
    let body = request.body.as_deref().unwrap_or("");
    let has_user_agent = request.headers.iter().any(|(name, _)| name == "user-agent");

    let mut lines: Vec<(&str, &[u8])> = vec![("host", host_header.as_bytes())];
    for (name, value) in &request.headers {
        lines.push((name, value.as_bytes()));
    }
    if !has_user_agent {
        lines.push(("user-agent", DEFAULT_USER_AGENT.as_bytes()));
    }
    lines.push(("accept-encoding", b"identity"));
    lines.push(("connection", b"close"));
    let content_length = body.len().to_string();
    if request.method == Method::Post {
        lines.push(("content-length", content_length.as_bytes()));
    }
    if let Some((header, value)) = credential {
        lines.push((header.as_str(), value));
    }

    let start = format!(
        "{} {} HTTP/1.1\r\n",
        request.method.as_str(),
        target.path_and_query
    );
    let size = start.len()
        + lines
            .iter()
            .map(|(name, value)| name.len() + 2 + value.len() + 2)
            .sum::<usize>()
        + 2
        + body.len();
    let mut out = Zeroizing::new(Vec::with_capacity(size));
    out.extend_from_slice(start.as_bytes());
    for (name, value) in lines {
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(value);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(body.as_bytes());
    debug_assert_eq!(out.len(), size);
    out
}

/// Bounds and timeouts the transport must apply (V-R5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Connect timeout.
    pub connect_timeout: Duration,
    /// TLS handshake timeout.
    pub handshake_timeout: Duration,
    /// Per-read and per-write timeout.
    pub io_timeout: Duration,
    /// Most bytes to read from the wire.
    pub max_wire_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            handshake_timeout: Duration::from_secs(10),
            io_timeout: Duration::from_secs(15),
            max_wire_bytes: MAX_WIRE_BYTES,
        }
    }
}

/// Moves bytes. A trait so the policy above it is tested without sockets.
pub trait Transport: Send + Sync {
    /// Send `request` over TLS to `address`, using `server_name` for SNI and
    /// certificate verification, and read the response up to
    /// `limits.max_wire_bytes`.
    fn exchange(
        &self,
        server_name: &str,
        address: SocketAddr,
        request: &[u8],
        limits: &Limits,
    ) -> Result<Zeroizing<Vec<u8>>, FetchError>;
}

/// The production transport: `tls-platform` over the checked address.
pub struct TlsTransport;

impl Transport for TlsTransport {
    fn exchange(
        &self,
        server_name: &str,
        address: SocketAddr,
        request: &[u8],
        limits: &Limits,
    ) -> Result<Zeroizing<Vec<u8>>, FetchError> {
        let mut config = tls_platform::TlsConfig::https_default();
        config.connect_timeout = limits.connect_timeout;
        config.handshake_timeout = limits.handshake_timeout;
        config.read_timeout = Some(limits.io_timeout);
        config.write_timeout = Some(limits.io_timeout);
        let connector = tls_platform::default_connector();
        let mut stream = connector
            .connect_addr(server_name, address, &config)
            .map_err(map_tls_error)?;
        stream.write_all(request).map_err(map_io_error)?;
        stream.flush().map_err(map_io_error)?;
        read_limited(&mut stream, limits.max_wire_bytes)
    }
}

/// Read until EOF or `limit` bytes, into one allocation made up front. A
/// response can echo the secret, so it gets the same care as the request.
pub fn read_limited(
    reader: &mut impl Read,
    limit: usize,
) -> Result<Zeroizing<Vec<u8>>, FetchError> {
    let mut buffer = Zeroizing::new(vec![0_u8; limit]);
    let mut filled = 0;
    while filled < limit {
        match reader.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(read) => filled += read,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            // A TLS peer that closes without close_notify after a complete
            // response is common; what we have is decoded and judged below.
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(map_io_error(error)),
        }
    }
    buffer.truncate(filled);
    Ok(buffer)
}

fn map_io_error(error: std::io::Error) -> FetchError {
    match error.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => FetchError::Timeout,
        _ => FetchError::Network,
    }
}

fn map_tls_error(error: tls_platform::TlsError) -> FetchError {
    use tls_platform::TlsError;
    match error {
        TlsError::TcpConnect { source, .. } => map_io_error(source),
        TlsError::DnsResolutionFailed { .. } => FetchError::Network,
        _ => FetchError::Tls,
    }
}

// ── The response (V-R5..R7, V-S4) ────────────────────────────────────────────

/// What the model gets back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchResponse {
    /// HTTP status.
    pub status: u16,
    /// Allowlisted response headers, lowercased names.
    pub headers: Vec<(String, String)>,
    /// The UTF-8 body, with every echo of a secret masked.
    pub body: String,
    /// Whether the body was cut at [`MAX_RESPONSE_BODY_BYTES`].
    pub truncated: bool,
}

impl FetchResponse {
    /// The tool output object.
    pub fn to_json(&self) -> JsonValue {
        JsonValue::Object(vec![
            (
                "status".to_string(),
                JsonValue::Number(JsonNumber::Integer(i64::from(self.status))),
            ),
            (
                "headers".to_string(),
                JsonValue::Object(
                    self.headers
                        .iter()
                        .map(|(name, value)| (name.clone(), JsonValue::String(value.clone())))
                        .collect(),
                ),
            ),
            ("body".to_string(), JsonValue::String(self.body.clone())),
            ("truncated".to_string(), JsonValue::Bool(self.truncated)),
        ])
    }
}

/// Decode a raw response into a [`FetchResponse`]. When a credential was
/// used, `raw` has already been through [`mask_echoes`].
///
/// `hit_wire_limit` says the transport stopped at its byte cap, so a body
/// that ends early is *truncated*, not malformed.
fn decode_response(raw: &[u8], hit_wire_limit: bool) -> Result<FetchResponse, FetchError> {
    let head_end = find(raw, b"\r\n\r\n").ok_or(FetchError::Protocol)? + 4;
    if head_end > MAX_RESPONSE_HEAD_BYTES {
        return Err(FetchError::Protocol);
    }
    let parsed = http1::parse_response_head(&raw[..head_end]).map_err(|_| FetchError::Protocol)?;
    let body_bytes = &raw[parsed.body_offset.min(raw.len())..];

    let (mut body, mut truncated) = match parsed.body_kind {
        BodyKind::None => (Vec::new(), false),
        BodyKind::ContentLength(length) => {
            if body_bytes.len() >= length {
                (body_bytes[..length].to_vec(), false)
            } else if hit_wire_limit {
                (body_bytes.to_vec(), true)
            } else {
                return Err(FetchError::Protocol);
            }
        }
        BodyKind::UntilEof => (body_bytes.to_vec(), hit_wire_limit),
        BodyKind::Chunked => decode_chunked(body_bytes, hit_wire_limit)?,
    };
    if body.len() > MAX_RESPONSE_BODY_BYTES {
        body.truncate(MAX_RESPONSE_BODY_BYTES);
        truncated = true;
    }
    let body = utf8_body(body, truncated)?;

    let headers = parsed
        .head
        .headers
        .iter()
        .filter_map(|header| {
            let name = header.name.to_ascii_lowercase();
            ALLOWED_RESPONSE_HEADERS
                .contains(&name.as_str())
                .then(|| (name, header.value.clone()))
        })
        .collect();
    Ok(FetchResponse {
        status: parsed.head.status,
        headers,
        body,
        truncated,
    })
}

/// Longest accepted chunk-size line (hex digits plus any extensions).
const MAX_CHUNK_LINE_BYTES: usize = 128;

/// Chunked transfer decoding. Returns `(body, truncated)`.
///
/// The chunk size is the server's word, and the server is not trusted: a
/// size near `usize::MAX` makes `size + 2` wrap, and a naive slice then
/// panics the daemon. So every size is checked against what could possibly
/// fit — the bytes actually present and the remaining body budget — with
/// checked arithmetic, before any slice is taken.
fn decode_chunked(mut input: &[u8], hit_wire_limit: bool) -> Result<(Vec<u8>, bool), FetchError> {
    let mut body = Vec::new();
    loop {
        let Some(line_end) = find(input, b"\r\n") else {
            if input.len() > MAX_CHUNK_LINE_BYTES {
                return Err(FetchError::Protocol);
            }
            return incomplete(body, hit_wire_limit);
        };
        if line_end > MAX_CHUNK_LINE_BYTES {
            return Err(FetchError::Protocol);
        }
        let size_text =
            std::str::from_utf8(&input[..line_end]).map_err(|_| FetchError::Protocol)?;
        let size_text = size_text.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_text, 16).map_err(|_| FetchError::Protocol)?;
        input = &input[line_end + 2..];
        if size == 0 {
            return Ok((body, false));
        }
        let remaining_budget = MAX_RESPONSE_BODY_BYTES.saturating_sub(body.len());
        let framed = size.checked_add(2).ok_or(FetchError::Protocol)?;
        if input.len() < framed {
            // Not all here: either the stream was cut at the wire limit, or
            // the server lied about the size.
            body.extend_from_slice(&input[..size.min(input.len()).min(remaining_budget)]);
            return incomplete(body, hit_wire_limit);
        }
        if &input[size..framed] != b"\r\n" {
            return Err(FetchError::Protocol);
        }
        if size > remaining_budget {
            body.extend_from_slice(&input[..remaining_budget]);
            return Ok((body, true));
        }
        body.extend_from_slice(&input[..size]);
        input = &input[framed..];
    }
}

fn incomplete(body: Vec<u8>, hit_wire_limit: bool) -> Result<(Vec<u8>, bool), FetchError> {
    if hit_wire_limit {
        Ok((body, true))
    } else {
        Err(FetchError::Protocol)
    }
}

/// V-R7: text only. A truncated body may end mid-character; that tail is
/// dropped rather than reported as binary.
fn utf8_body(body: Vec<u8>, truncated: bool) -> Result<String, FetchError> {
    match String::from_utf8(body) {
        Ok(text) => Ok(text),
        Err(error) => {
            let utf8 = error.utf8_error();
            if truncated && utf8.error_len().is_none() {
                let valid = utf8.valid_up_to();
                let mut bytes = error.into_bytes();
                bytes.truncate(valid);
                Ok(String::from_utf8(bytes).expect("valid_up_to marks a UTF-8 prefix"))
            } else {
                Err(FetchError::BinaryBody)
            }
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// V-S4: overwrite every echo of the secret in the raw response, in place.
///
/// This runs on the zeroizing wire buffer **before** anything parses or copies
/// it, which is what makes three properties hold at once:
///
/// - **No unmasked copy ever exists.** `http1` copies header values (including
///   ones off the allowlist, like an echoed `authorization`) into ordinary
///   `String`s; masking first means those copies are already clean.
/// - **Framing survives.** Each byte becomes `*`, so lengths do not change and
///   `Content-Length` and chunk sizes still describe the body.
/// - **A cut cannot split an echo out of the mask.** The whole buffer is
///   masked before the body is truncated to its bound. When the stream itself
///   was cut at the wire limit, a trailing *prefix* of a needle is masked too,
///   so an echo straddling the cut does not leak all but its last byte.
///
/// Each needle is masked in its exact form and in the two encodings an API is
/// most likely to reflect it in: JSON string escaping (`\"`, `\\`, `\/`)
/// and percent-encoding. Longest first, so the header value is masked as one
/// run rather than leaving its scheme word behind.
fn mask_echoes(raw: &mut [u8], needles: &[&[u8]], stream_was_cut: bool) {
    let mut variants: Vec<Vec<u8>> = Vec::new();
    for needle in needles {
        for variant in [
            needle.to_vec(),
            json_escaped(needle),
            percent_encoded(needle),
        ] {
            if variant.len() >= MIN_SECRET_BYTES && !variants.contains(&variant) {
                variants.push(variant);
            }
        }
    }
    variants.sort_by_key(|variant| std::cmp::Reverse(variant.len()));
    for variant in &variants {
        let mut start = 0;
        while let Some(found) = find(&raw[start..], variant) {
            let at = start + found;
            raw[at..at + variant.len()].fill(b'*');
            start = at + variant.len();
        }
    }
    if stream_was_cut {
        for variant in &variants {
            let longest = variant.len().saturating_sub(1).min(raw.len());
            if let Some(k) = (1..=longest).rev().find(|k| raw.ends_with(&variant[..*k])) {
                let end = raw.len();
                raw[end - k..].fill(b'*');
            }
        }
    }
}

fn json_escaped(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        if matches!(byte, b'"' | b'\\' | b'/') {
            out.push(b'\\');
        }
        out.push(byte);
    }
    out
}

fn percent_encoded(bytes: &[u8]) -> Vec<u8> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = Vec::with_capacity(bytes.len() * 3);
    for &byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte);
        } else {
            out.extend_from_slice(&[
                b'%',
                HEX[usize::from(byte >> 4)],
                HEX[usize::from(byte & 15)],
            ]);
        }
    }
    out
}

// ── The operation ────────────────────────────────────────────────────────────

/// `net.fetch`, parameterized by how names resolve and how bytes move.
pub struct NetFetch<R: Resolver, T: Transport> {
    resolver: R,
    transport: T,
    limits: Limits,
}

impl NetFetch<SystemResolver, TlsTransport> {
    /// The production composition: system DNS and `tls-platform`.
    pub fn production() -> Self {
        Self::new(SystemResolver, TlsTransport)
    }
}

impl<R: Resolver, T: Transport> NetFetch<R, T> {
    /// Compose a fetcher with the default [`Limits`].
    pub fn new(resolver: R, transport: T) -> Self {
        Self {
            resolver,
            transport,
            limits: Limits::default(),
        }
    }

    /// Run one fetch for a host whose signed manifest produced `allowlist`.
    ///
    /// `credentials` is consulted only when the request carries a
    /// credential, and only after every refusal that does not need the
    /// network has had its chance (V-S6).
    pub fn execute(
        &self,
        allowlist: &NetAllowlist,
        request: &FetchRequest,
        credentials: Option<&dyn CredentialSource>,
    ) -> Result<FetchResponse, FetchError> {
        let target = allowlist.authorize(&request.url)?;
        let address = checked_address(&self.resolver, &target)?;

        let redeemed = match &request.credential {
            None => None,
            Some(spec) => {
                let source = credentials.ok_or(FetchError::CredentialRefused)?;
                let destination = format!("{}:{}", target.host, target.port);
                let secret = source.redeem(&spec.vault_ref, &destination)?;
                let value = credential_value(spec, &secret)?;
                Some((spec.header, secret, value))
            }
        };

        let wire = encode_request(
            request,
            &target,
            redeemed
                .as_ref()
                .map(|(header, _, value)| (*header, value.as_slice())),
        );
        let mut raw = self
            .transport
            .exchange(&target.host, address, &wire, &self.limits)?;
        drop(wire);
        let stream_was_cut = raw.len() >= self.limits.max_wire_bytes;
        if let Some((_, secret, value)) = &redeemed {
            mask_echoes(
                &mut raw,
                &[value.as_slice(), secret.as_slice()],
                stream_was_cut,
            );
        }
        decode_response(&raw, stream_was_cut)
    }
}

// ── The D18D definition ──────────────────────────────────────────────────────

/// The `net.fetch` tool definition.
///
/// The `headers` schemas enumerate their allowlists as the only properties,
/// with unknown fields refused, so the schema a model sees *is* the rule.
pub fn tool_definition() -> ToolDefinition {
    let string_headers = |names: &[&str]| JsonSchema::Object {
        properties: names
            .iter()
            .map(|name| SchemaProperty::new(*name, JsonSchema::String))
            .collect(),
        required: vec![],
        allow_unknown_fields: false,
    };
    let enum_of = |values: &[&str]| JsonSchema::Enum {
        values: values
            .iter()
            .map(|value| JsonValue::String((*value).to_string()))
            .collect(),
    };
    ToolDefinition {
        tool_id: NET_FETCH_TOOL_ID.to_string(),
        display_name: "Fetch a URL".to_string(),
        description: "Fetch an HTTPS URL that this agent's signed manifest declares. To \
                      authenticate, first call vault.request_lease and pass its vault_ref as \
                      credential.vault_ref; the secret never reaches you."
            .to_string(),
        input_schema: JsonSchema::Object {
            properties: vec![
                SchemaProperty::new("url", JsonSchema::String),
                SchemaProperty::new("method", enum_of(&["GET", "POST"])),
                SchemaProperty::new("headers", string_headers(ALLOWED_REQUEST_HEADERS)),
                SchemaProperty::new("body", JsonSchema::String),
                SchemaProperty::new(
                    "credential",
                    JsonSchema::Object {
                        properties: vec![
                            SchemaProperty::new("vault_ref", JsonSchema::String),
                            SchemaProperty::new(
                                "header",
                                enum_of(&["authorization", "x-api-key", "x-auth-token"]),
                            ),
                            SchemaProperty::new("scheme", enum_of(&["Bearer", "Basic", "Token"])),
                        ],
                        required: vec!["vault_ref".to_string(), "header".to_string()],
                        allow_unknown_fields: false,
                    },
                ),
            ],
            required: vec!["url".to_string(), "method".to_string()],
            allow_unknown_fields: false,
        },
        output_schema: Some(JsonSchema::Object {
            properties: vec![
                SchemaProperty::new("status", JsonSchema::Integer),
                SchemaProperty::new("headers", string_headers(ALLOWED_RESPONSE_HEADERS)),
                SchemaProperty::new("body", JsonSchema::String),
                SchemaProperty::new("truncated", JsonSchema::Boolean),
            ],
            required: vec![
                "status".to_string(),
                "headers".to_string(),
                "body".to_string(),
                "truncated".to_string(),
            ],
            allow_unknown_fields: false,
        }),
        side_effects: ToolSideEffects::External,
        idempotency: ToolIdempotency::Never,
        concurrency: ToolConcurrency::Safe,
        streaming: ToolStreaming::None,
        // Declared, not enforced on the daemon model-tool path today (D18V
        // "Privilege").
        required_tier: PrivilegeTier::Tier1,
        required_capabilities: vec!["net:connect".to_string()],
        preferred_lock_scope: None,
        timeout_seconds: Some(45),
        tags: vec!["net".to_string(), "vault".to_string()],
        stability: ToolStability::Experimental,
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn chunked_decoding_handles_extensions_and_detects_truncation() {
        let complete = b"4;ext=1\r\nwiki\r\n5\r\npedia\r\n0\r\n\r\n";
        assert_eq!(
            decode_chunked(complete, false).unwrap(),
            (b"wikipedia".to_vec(), false)
        );
        let cut = b"4\r\nwiki\r\n5\r\npe";
        assert_eq!(decode_chunked(cut, false), Err(FetchError::Protocol));
        assert_eq!(
            decode_chunked(cut, true).unwrap(),
            (b"wikipe".to_vec(), true)
        );
        assert_eq!(decode_chunked(b"zz\r\n", false), Err(FetchError::Protocol));
        assert_eq!(
            decode_chunked(b"2\r\nabXX", false),
            Err(FetchError::Protocol)
        );
        assert_eq!(decode_chunked(b"2", true).unwrap(), (Vec::new(), true));
    }

    #[test]
    fn mask_echoes_masks_a_prefix_left_at_a_cut_and_nothing_else() {
        let needles: [&[u8]; 1] = [b"tok-1234567890"];
        let mut cut = b"data data tok-12345".to_vec();
        mask_echoes(&mut cut, &needles, true);
        assert_eq!(&cut, b"data data *********");
        // Without a cut, a partial is ordinary text and is left alone.
        let mut whole = b"data data tok-12345".to_vec();
        mask_echoes(&mut whole, &needles, false);
        assert_eq!(&whole, b"data data tok-12345");
        // Exact echoes are masked wherever they are, lengths unchanged.
        let mut echo = b"<tok-1234567890><tok-1234567890>".to_vec();
        mask_echoes(&mut echo, &needles, false);
        assert_eq!(&echo, b"<**************><**************>");
    }

    #[test]
    fn encodings_cover_json_and_percent_forms() {
        assert_eq!(json_escaped(br#"a/b"c\d"#), br#"a\/b\"c\\d"#.to_vec());
        assert_eq!(percent_encoded(b"a/b c~"), b"a%2Fb%20c~".to_vec());
    }

    #[test]
    fn utf8_body_keeps_a_truncated_prefix_but_refuses_binary() {
        let mut cut = "héllo".as_bytes().to_vec();
        cut.truncate(2); // splits the é
        assert_eq!(utf8_body(cut.clone(), true).unwrap(), "h");
        assert_eq!(utf8_body(cut, false), Err(FetchError::BinaryBody));
        assert_eq!(
            utf8_body(vec![0xff, b'a'], true),
            Err(FetchError::BinaryBody)
        );
    }

    #[test]
    fn credential_values_are_bounded_and_printable() {
        let spec = CredentialSpec {
            vault_ref: "r".into(),
            header: CredentialHeader::Authorization,
            scheme: Some(CredentialScheme::Bearer),
        };
        assert_eq!(
            &*credential_value(&spec, b"abcdefgh").unwrap(),
            b"Bearer abcdefgh"
        );
        for bad in [
            &b"short"[..],
            b"has space1",
            b"line\r\nbrk",
            &[b'a'; 4097][..],
        ] {
            assert_eq!(
                credential_value(&spec, bad).err(),
                Some(FetchError::CredentialRefused)
            );
        }
        let bare = CredentialSpec {
            scheme: None,
            ..spec
        };
        assert_eq!(&*credential_value(&bare, b"abcdefgh").unwrap(), b"abcdefgh");
    }

    #[test]
    fn io_and_tls_errors_map_to_bounded_kinds() {
        use std::io::{Error, ErrorKind};
        assert_eq!(
            map_io_error(Error::from(ErrorKind::TimedOut)),
            FetchError::Timeout
        );
        assert_eq!(
            map_io_error(Error::from(ErrorKind::WouldBlock)),
            FetchError::Timeout
        );
        assert_eq!(
            map_io_error(Error::from(ErrorKind::ConnectionReset)),
            FetchError::Network
        );
        assert_eq!(
            map_tls_error(tls_platform::TlsError::TcpConnect {
                host: "h".into(),
                port: 443,
                source: Error::from(ErrorKind::TimedOut),
            }),
            FetchError::Timeout
        );
        assert_eq!(
            map_tls_error(tls_platform::TlsError::DnsResolutionFailed {
                host: "h".into(),
                message: "m".into(),
            }),
            FetchError::Network
        );
        assert_eq!(
            map_tls_error(tls_platform::TlsError::InvalidPort { port: 0 }),
            FetchError::Tls
        );
    }
}
