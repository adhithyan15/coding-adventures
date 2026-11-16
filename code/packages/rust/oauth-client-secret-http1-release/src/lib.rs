//! Audited complete HTTP/1 release for custody-built OAuth client-secret requests.
//!
//! This crate joins exact provider, trace, request-line, HTTPS destination,
//! framing, media-type, authentication-placement, and body evidence. It
//! prepares one bounded request in wipe-on-drop memory before terminal audit.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_oauth::{OAuthTraceId, ProviderId};
use coding_adventures_zeroize::Zeroizing;
use core::fmt;
use http1_request_header_field::{
    encode_http1_request_header_field, Http1RequestHeaderField, MAX_HTTP1_REQUEST_HEADER_LINE_BYTES,
};
use http1_request_line::{
    encode_http1_request_line, Http1RequestLine, MAX_HTTP1_REQUEST_LINE_BYTES,
};
use https_destination_name_binding::BoundHttpsHostHeaderField;
use oauth_client_secret_http1_binding::BoundClientSecretHttp1RequestMaterial;

/// Largest complete request admitted by this narrow release boundary.
pub const MAX_CLIENT_SECRET_HTTP1_REQUEST_BYTES: usize = MAX_HTTP1_REQUEST_LINE_BYTES
    + (4 * MAX_HTTP1_REQUEST_HEADER_LINE_BYTES)
    + 2
    + http1_fixed_request_body::MAX_HTTP1_FIXED_REQUEST_BODY_BYTES;

/// Data-driven policy for one complete client-secret request release.
pub struct ClientSecretHttp1ReleasePolicy<'policy> {
    provider: &'policy ProviderId,
    method: &'policy [u8],
    content_type: &'policy [u8],
    maximum_request_bytes: u32,
}

impl<'policy> ClientSecretHttp1ReleasePolicy<'policy> {
    /// Validate exact provider policy and its complete-request byte ceiling.
    pub fn new(
        provider: &'policy ProviderId,
        method: &'policy [u8],
        content_type: &'policy [u8],
        maximum_request_bytes: usize,
    ) -> Result<Self, ClientSecretHttp1ReleasePolicyError> {
        if method.is_empty() || content_type.is_empty() {
            return Err(ClientSecretHttp1ReleasePolicyError::InvalidMetadata);
        }
        if maximum_request_bytes == 0
            || maximum_request_bytes > MAX_CLIENT_SECRET_HTTP1_REQUEST_BYTES
        {
            return Err(ClientSecretHttp1ReleasePolicyError::InvalidLimits);
        }
        Ok(Self {
            provider,
            method,
            content_type,
            maximum_request_bytes: maximum_request_bytes as u32,
        })
    }
}

impl fmt::Debug for ClientSecretHttp1ReleasePolicy<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientSecretHttp1ReleasePolicy")
            .field("provider", &"[REDACTED]")
            .field("method", &"[REDACTED]")
            .field("content_type", &"[REDACTED]")
            .field("maximum_request_bytes", &self.maximum_request_bytes)
            .finish_non_exhaustive()
    }
}

/// Stable policy-construction failures without policy payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientSecretHttp1ReleasePolicyError {
    /// Method or content type was empty.
    InvalidMetadata,
    /// Complete-request byte ceiling was zero or exceeded the hard maximum.
    InvalidLimits,
}

impl fmt::Display for ClientSecretHttp1ReleasePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidMetadata => "invalid OAuth client-secret HTTP/1 release metadata",
            Self::InvalidLimits => "invalid OAuth client-secret HTTP/1 release limits",
        })
    }
}

impl std::error::Error for ClientSecretHttp1ReleasePolicyError {}

/// Stable action published for one complete request release.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientSecretHttp1ReleaseAuditAction {
    /// Prepare and release one exact complete client-secret request.
    ReleaseCompleteRequest,
    /// Borrow the already prepared complete wire bytes.
    ReadCompleteRequest,
}

/// Closed failure class published without request or credential payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientSecretHttp1ReleaseAuditFailure {
    /// Provider, method, endpoint, destination, or media type did not match.
    PolicyMismatch,
    /// Framing or authentication-field placement was inconsistent.
    EvidenceMismatch,
    /// A length ceiling or bounded allocation rejected the request.
    ResourceLimit,
}

/// Privacy-safe outcome for one complete request release.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientSecretHttp1ReleaseAuditOutcome {
    /// Audit precedes every credential-bearing read by this call.
    Attempted,
    /// The complete request was prepared and is ready for ownership transfer.
    Succeeded,
    /// The release failed closed and returned no request.
    Failed(ClientSecretHttp1ReleaseAuditFailure),
}

/// Provider- and trace-bound audit event containing no request payload.
pub struct ClientSecretHttp1ReleaseAuditEvent<'provider> {
    provider: &'provider ProviderId,
    trace: OAuthTraceId,
    action: ClientSecretHttp1ReleaseAuditAction,
    outcome: ClientSecretHttp1ReleaseAuditOutcome,
}

impl ClientSecretHttp1ReleaseAuditEvent<'_> {
    /// Return the exact request provider whose release was attempted.
    #[must_use]
    pub const fn provider(&self) -> &ProviderId {
        self.provider
    }

    /// Return the exact custody trace for this release.
    #[must_use]
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the stable complete-request action.
    #[must_use]
    pub const fn action(&self) -> ClientSecretHttp1ReleaseAuditAction {
        self.action
    }

    /// Return the closed outcome without request payloads.
    #[must_use]
    pub const fn outcome(&self) -> ClientSecretHttp1ReleaseAuditOutcome {
        self.outcome
    }
}

impl fmt::Debug for ClientSecretHttp1ReleaseAuditEvent<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientSecretHttp1ReleaseAuditEvent")
            .field("provider", &"[REDACTED]")
            .field("trace", &"[REDACTED]")
            .field("action", &self.action)
            .field("outcome", &self.outcome)
            .finish()
    }
}

/// Opaque durable-audit failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientSecretHttp1ReleaseAuditError;

/// Injected durable sink for complete-request release events.
pub trait ClientSecretHttp1ReleaseAuditSink {
    /// Publish one privacy-safe event durably or fail closed.
    fn publish(
        &mut self,
        event: &ClientSecretHttp1ReleaseAuditEvent<'_>,
    ) -> Result<(), ClientSecretHttp1ReleaseAuditError>;
}

/// Stable payload-free complete-request release failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientSecretHttp1ReleaseError {
    /// Provider or configured request metadata did not match exact evidence.
    PolicyMismatch,
    /// Custody endpoint did not equal the exact HTTPS destination and target.
    EndpointMismatch,
    /// Content-Type, framing, or authentication placement was inconsistent.
    EvidenceMismatch,
    /// Complete request exceeded its byte ceiling.
    RequestTooLong,
    /// Bounded wipe-on-drop request allocation failed.
    Allocation,
    /// Durable audit publication failed.
    Audit,
}

impl ClientSecretHttp1ReleaseError {
    const fn failure_class(self) -> Option<ClientSecretHttp1ReleaseAuditFailure> {
        match self {
            Self::PolicyMismatch | Self::EndpointMismatch => {
                Some(ClientSecretHttp1ReleaseAuditFailure::PolicyMismatch)
            }
            Self::EvidenceMismatch => Some(ClientSecretHttp1ReleaseAuditFailure::EvidenceMismatch),
            Self::RequestTooLong | Self::Allocation => {
                Some(ClientSecretHttp1ReleaseAuditFailure::ResourceLimit)
            }
            Self::Audit => None,
        }
    }
}

impl fmt::Display for ClientSecretHttp1ReleaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::PolicyMismatch => "OAuth client-secret HTTP/1 release policy mismatch",
            Self::EndpointMismatch => "OAuth client-secret HTTPS endpoint mismatch",
            Self::EvidenceMismatch => "OAuth client-secret HTTP/1 evidence mismatch",
            Self::RequestTooLong => "OAuth client-secret HTTP/1 request exceeds its limit",
            Self::Allocation => "OAuth client-secret HTTP/1 request allocation failed",
            Self::Audit => "OAuth client-secret HTTP/1 release audit failed",
        })
    }
}

impl std::error::Error for ClientSecretHttp1ReleaseError {}

/// Non-copyable complete request retaining every exact input proof.
pub struct ReleasedClientSecretHttp1Request<
    'policy,
    'request_line,
    'destination,
    'certificate,
    'encoded,
    'server_name,
    'host,
    'host_field,
    'request,
    'length,
    'content_type,
> {
    provider: &'policy ProviderId,
    trace: OAuthTraceId,
    request_line: Http1RequestLine<'request_line>,
    destination: &'destination BoundHttpsHostHeaderField<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
    >,
    _material: BoundClientSecretHttp1RequestMaterial<'request, 'length>,
    content_type: Http1RequestHeaderField<'content_type>,
    encoded: Zeroizing<Vec<u8>>,
}

impl ReleasedClientSecretHttp1Request<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    /// Return the exact policy provider bound to the successful release.
    #[must_use]
    pub const fn provider(&self) -> &ProviderId {
        self.provider
    }

    /// Return the exact custody trace bound to the successful release.
    #[must_use]
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the exact validated request line retained by this release.
    #[must_use]
    pub const fn request_line(&self) -> &Http1RequestLine<'_> {
        &self.request_line
    }

    /// Return the exact HTTPS destination and canonical Host proof.
    #[must_use]
    pub const fn destination(&self) -> &BoundHttpsHostHeaderField<'_, '_, '_, '_, '_> {
        self.destination
    }

    /// Return the exact validated Content-Type field retained by this release.
    #[must_use]
    pub const fn content_type(&self) -> &Http1RequestHeaderField<'_> {
        &self.content_type
    }

    /// Audit and borrow the exact prepared complete request bytes.
    ///
    /// Every borrow introduced by this package requires a fresh durable event
    /// carrying the retained provider and trace. Audit failure withholds the
    /// slice and leaves the prepared wipe-on-drop request owned by this proof.
    pub fn audited_bytes<A: ClientSecretHttp1ReleaseAuditSink>(
        &self,
        audit: &mut A,
    ) -> Result<&[u8], ClientSecretHttp1ReleaseError> {
        publish(
            audit,
            self.provider,
            self.trace,
            ClientSecretHttp1ReleaseAuditAction::ReadCompleteRequest,
            ClientSecretHttp1ReleaseAuditOutcome::Succeeded,
        )?;
        Ok(&self.encoded)
    }

    /// Return the complete request byte length.
    #[must_use]
    pub fn len(&self) -> usize {
        self.encoded.len()
    }

    /// Return whether the complete request is empty, which is always false.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.encoded.is_empty()
    }
}

impl fmt::Debug for ReleasedClientSecretHttp1Request<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReleasedClientSecretHttp1Request")
            .field("provider", &"[REDACTED]")
            .field("trace", &"[REDACTED]")
            .field("request_line", &"[REDACTED]")
            .field("destination", &"[REDACTED]")
            .field("material", &"[REDACTED]")
            .field("content_type", &"[REDACTED]")
            .field("encoded", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Audit, prepare, and release one exact complete client-secret request.
///
/// The attempted event is durable before any credential-bearing body or
/// Authorization bytes are read. The terminal event follows complete
/// preparation. Success then moves the prepared wipe-on-drop allocation into
/// the returned proof without another credential read or copy.
#[allow(clippy::too_many_arguments)]
pub fn release_client_secret_http1_request<
    'policy,
    'request_line,
    'destination,
    'certificate,
    'encoded,
    'server_name,
    'host,
    'host_field,
    'request,
    'length,
    'content_type,
    A: ClientSecretHttp1ReleaseAuditSink,
>(
    policy: ClientSecretHttp1ReleasePolicy<'policy>,
    request_line: Http1RequestLine<'request_line>,
    destination: &'destination BoundHttpsHostHeaderField<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
    >,
    material: BoundClientSecretHttp1RequestMaterial<'request, 'length>,
    content_type: Http1RequestHeaderField<'content_type>,
    audit: &mut A,
) -> Result<
    ReleasedClientSecretHttp1Request<
        'policy,
        'request_line,
        'destination,
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'request,
        'length,
        'content_type,
    >,
    ClientSecretHttp1ReleaseError,
> {
    let request = material.request();
    let provider = request.provider();
    let trace = request.trace();
    publish(
        audit,
        provider,
        trace,
        ClientSecretHttp1ReleaseAuditAction::ReleaseCompleteRequest,
        ClientSecretHttp1ReleaseAuditOutcome::Attempted,
    )?;

    let prepared = prepare_request(
        &policy,
        &request_line,
        destination,
        &material,
        &content_type,
    );
    let outcome = match &prepared {
        Ok(_) => ClientSecretHttp1ReleaseAuditOutcome::Succeeded,
        Err(error) => ClientSecretHttp1ReleaseAuditOutcome::Failed(
            error
                .failure_class()
                .expect("preparation never returns the audit-only failure"),
        ),
    };
    publish(
        audit,
        provider,
        trace,
        ClientSecretHttp1ReleaseAuditAction::ReleaseCompleteRequest,
        outcome,
    )?;
    let encoded = prepared?;

    Ok(ReleasedClientSecretHttp1Request {
        provider: policy.provider,
        trace,
        request_line,
        destination,
        _material: material,
        content_type,
        encoded,
    })
}

fn prepare_request(
    policy: &ClientSecretHttp1ReleasePolicy<'_>,
    request_line: &Http1RequestLine<'_>,
    destination: &BoundHttpsHostHeaderField<'_, '_, '_, '_, '_>,
    material: &BoundClientSecretHttp1RequestMaterial<'_, '_>,
    content_type: &Http1RequestHeaderField<'_>,
) -> Result<Zeroizing<Vec<u8>>, ClientSecretHttp1ReleaseError> {
    let request = material.request();
    if request.provider() != policy.provider || request_line.method() != policy.method {
        return Err(ClientSecretHttp1ReleaseError::PolicyMismatch);
    }
    if !endpoint_matches(
        request.endpoint().as_bytes(),
        destination.host_field().value(),
        request_line.target(),
    ) {
        return Err(ClientSecretHttp1ReleaseError::EndpointMismatch);
    }
    if !content_type.name().eq_ignore_ascii_case(b"Content-Type")
        || content_type.value() != policy.content_type
        || content_type.value() != request.content_type().as_bytes()
    {
        return Err(ClientSecretHttp1ReleaseError::EvidenceMismatch);
    }

    let body = material.body().body().body();
    let content_length = material.body().field();
    let authorization = material.authorization_field();
    let encoded_length = request_line
        .encoded_length()
        .checked_add(destination.host_field().encoded_length())
        .and_then(|length| length.checked_add(content_length.encoded_length()))
        .and_then(|length| length.checked_add(content_type.encoded_length()))
        .and_then(|length| {
            authorization.map_or(Some(length), |field| {
                length.checked_add(field.encoded_length())
            })
        })
        .and_then(|length| length.checked_add(2))
        .and_then(|length| length.checked_add(body.len()))
        .ok_or(ClientSecretHttp1ReleaseError::RequestTooLong)?;
    if encoded_length > policy.maximum_request_bytes as usize {
        return Err(ClientSecretHttp1ReleaseError::RequestTooLong);
    }

    let mut storage = Vec::new();
    storage
        .try_reserve_exact(encoded_length)
        .map_err(|_| ClientSecretHttp1ReleaseError::Allocation)?;
    storage.resize(encoded_length, 0);
    let mut storage = Zeroizing::new(storage);
    let mut offset = 0usize;
    offset += encode_line(request_line, &mut storage[offset..])?;
    offset += encode_field(destination.host_field(), &mut storage[offset..])?;
    offset += encode_field(content_length, &mut storage[offset..])?;
    offset += encode_field(content_type, &mut storage[offset..])?;
    if let Some(field) = authorization {
        offset += encode_field(field, &mut storage[offset..])?;
    }
    storage[offset..offset + 2].copy_from_slice(b"\r\n");
    offset += 2;
    storage[offset..].copy_from_slice(body);
    debug_assert_eq!(storage.len(), encoded_length);
    Ok(storage)
}

fn encode_line(
    request_line: &Http1RequestLine<'_>,
    output: &mut [u8],
) -> Result<usize, ClientSecretHttp1ReleaseError> {
    encode_http1_request_line(request_line, output)
        .map(|encoded| encoded.len())
        .map_err(|_| ClientSecretHttp1ReleaseError::EvidenceMismatch)
}

fn encode_field(
    field: &Http1RequestHeaderField<'_>,
    output: &mut [u8],
) -> Result<usize, ClientSecretHttp1ReleaseError> {
    encode_http1_request_header_field(field, output)
        .map(|encoded| encoded.len())
        .map_err(|_| ClientSecretHttp1ReleaseError::EvidenceMismatch)
}

fn endpoint_matches(endpoint: &[u8], authority: &[u8], target: &[u8]) -> bool {
    endpoint.len() == b"https://".len() + authority.len() + target.len()
        && endpoint.starts_with(b"https://")
        && endpoint[b"https://".len()..].starts_with(authority)
        && endpoint.ends_with(target)
}

fn publish<A: ClientSecretHttp1ReleaseAuditSink>(
    audit: &mut A,
    provider: &ProviderId,
    trace: OAuthTraceId,
    action: ClientSecretHttp1ReleaseAuditAction,
    outcome: ClientSecretHttp1ReleaseAuditOutcome,
) -> Result<(), ClientSecretHttp1ReleaseError> {
    audit
        .publish(&ClientSecretHttp1ReleaseAuditEvent {
            provider,
            trace,
            action,
            outcome,
        })
        .map_err(|_| ClientSecretHttp1ReleaseError::Audit)
}
