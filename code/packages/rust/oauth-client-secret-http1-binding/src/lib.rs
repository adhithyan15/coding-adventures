//! Audited construction of exact OAuth client-secret HTTP/1 request evidence.
//!
//! This crate constructs strict fixed-body, canonical Content-Length, and
//! optional Authorization evidence directly from one custody-built request.
//! Credential-bearing bytes are not inspected before the attempted audit.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use coding_adventures_oauth::{OAuthTraceId, ProviderId};
use coding_adventures_oauth_client_secret_custody::ClientSecretAuthenticatedRequest;
use core::fmt;
use http1_fixed_request_body::{
    bind_http1_fixed_request_content_length_field, encode_http1_fixed_request_content_length,
    validate_http1_fixed_request_body, Http1FixedRequestBody, Http1FixedRequestBodyLimits,
    Http1FixedRequestContentLengthField, MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES,
};
use http1_request_header_field::{
    validate_http1_request_header_field, Http1RequestHeaderField, Http1RequestHeaderFieldLimits,
};

/// Stable action emitted for one request-material construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientSecretHttp1AuditAction {
    /// Construct exact HTTP/1 evidence from custody-built request material.
    BindRequestMaterial,
}

/// Closed outcome emitted without credential or request payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientSecretHttp1AuditOutcome {
    /// The construction is about to inspect credential-bearing material.
    Attempted,
    /// Credential-dependent validation succeeded and deterministic release is authorized.
    Succeeded,
    /// A bounded body, field, or Content-Length check rejected the request.
    Rejected,
}

/// Privacy-safe audit event for one request-material construction.
#[derive(Clone, Eq, PartialEq)]
pub struct ClientSecretHttp1AuditEvent {
    provider: ProviderId,
    trace: OAuthTraceId,
    action: ClientSecretHttp1AuditAction,
    outcome: ClientSecretHttp1AuditOutcome,
}

impl fmt::Debug for ClientSecretHttp1AuditEvent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientSecretHttp1AuditEvent")
            .field("provider", &"[REDACTED]")
            .field("trace", &"[REDACTED]")
            .field("action", &self.action)
            .field("outcome", &self.outcome)
            .finish()
    }
}

impl ClientSecretHttp1AuditEvent {
    /// Return the validated provider inherited from custody.
    #[must_use]
    pub const fn provider(&self) -> &ProviderId {
        &self.provider
    }

    /// Return the exact custody and transport correlation identity.
    #[must_use]
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the stable request-material action.
    #[must_use]
    pub const fn action(&self) -> ClientSecretHttp1AuditAction {
        self.action
    }

    /// Return the closed event outcome.
    #[must_use]
    pub const fn outcome(&self) -> ClientSecretHttp1AuditOutcome {
        self.outcome
    }
}

/// Opaque durable-audit failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientSecretHttp1AuditError;

/// Injected durable sink for request-material construction events.
pub trait ClientSecretHttp1AuditSink {
    /// Publish one privacy-safe event durably before returning success.
    fn publish(
        &mut self,
        event: &ClientSecretHttp1AuditEvent,
    ) -> Result<(), ClientSecretHttp1AuditError>;
}

/// Stable payload-free request-material binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientSecretHttp1BindingError {
    /// The custody-built form body violates the selected fixed-body limits.
    BodyRejected,
    /// Canonical Content-Length construction or field validation failed.
    ContentLengthRejected,
    /// The custody-built Authorization value violates selected field limits.
    AuthorizationRejected,
    /// A required audit publication failed.
    Audit,
}

impl fmt::Display for ClientSecretHttp1BindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::BodyRejected => "OAuth client-secret HTTP/1 body was rejected",
            Self::ContentLengthRejected => "OAuth client-secret HTTP/1 Content-Length was rejected",
            Self::AuthorizationRejected => {
                "OAuth client-secret HTTP/1 Authorization field was rejected"
            }
            Self::Audit => "OAuth client-secret HTTP/1 identity audit failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ClientSecretHttp1BindingError {}

/// Non-copyable exact binding between custody material and strict HTTP/1 evidence.
pub struct BoundClientSecretHttp1RequestMaterial<'request, 'length> {
    request: &'request ClientSecretAuthenticatedRequest,
    body: Http1FixedRequestContentLengthField<'request, 'length>,
    authorization: Option<Http1RequestHeaderField<'request>>,
}

impl<'request, 'length> BoundClientSecretHttp1RequestMaterial<'request, 'length> {
    /// Return the exact custody-built request retained by this proof.
    #[must_use]
    pub const fn request(&self) -> &'request ClientSecretAuthenticatedRequest {
        self.request
    }

    /// Return the exact fixed-body and canonical Content-Length proof.
    #[must_use]
    pub const fn body(&self) -> &Http1FixedRequestContentLengthField<'request, 'length> {
        &self.body
    }

    /// Return the exact Authorization field created for Basic authentication.
    #[must_use]
    pub const fn authorization_field(&self) -> Option<&Http1RequestHeaderField<'request>> {
        self.authorization.as_ref()
    }
}

impl fmt::Debug for BoundClientSecretHttp1RequestMaterial<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundClientSecretHttp1RequestMaterial")
            .field("provider", &"[REDACTED]")
            .field("trace", &"[REDACTED]")
            .field("request", &"[REDACTED]")
            .field("body", &"[REDACTED]")
            .field("has_authorization", &self.authorization.is_some())
            .finish()
    }
}

/// Audit and construct strict HTTP/1 evidence from one custody-built request.
///
/// The attempted event is durable before credential-bearing bytes are
/// inspected. Validation and audit failures leave the seven-byte caller buffer
/// unchanged. A succeeded event is durable before deterministic output binding
/// releases the proof, and no credential-bearing input is read after it.
pub fn bind_client_secret_http1_request_material<
    'request,
    'length,
    A: ClientSecretHttp1AuditSink,
>(
    request: &'request ClientSecretAuthenticatedRequest,
    body_limits: Http1FixedRequestBodyLimits,
    field_limits: Http1RequestHeaderFieldLimits,
    content_length_output: &'length mut [u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES],
    audit: &mut A,
) -> Result<BoundClientSecretHttp1RequestMaterial<'request, 'length>, ClientSecretHttp1BindingError>
{
    publish(audit, request, ClientSecretHttp1AuditOutcome::Attempted)?;
    let prepared = match prepare_request_material(request, body_limits, field_limits) {
        Ok(prepared) => prepared,
        Err(error) => {
            publish(audit, request, ClientSecretHttp1AuditOutcome::Rejected)?;
            return Err(error);
        }
    };

    publish(audit, request, ClientSecretHttp1AuditOutcome::Succeeded)?;

    Ok(release_request_material(prepared, content_length_output))
}

struct PreparedClientSecretHttp1RequestMaterial<'request> {
    request: &'request ClientSecretAuthenticatedRequest,
    body: Http1FixedRequestBody<'request>,
    authorization: Option<Http1RequestHeaderField<'request>>,
    content_length_value: [u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES],
    content_length_value_length: u8,
    field_limits: Http1RequestHeaderFieldLimits,
}

fn prepare_request_material<'request>(
    request: &'request ClientSecretAuthenticatedRequest,
    body_limits: Http1FixedRequestBodyLimits,
    field_limits: Http1RequestHeaderFieldLimits,
) -> Result<PreparedClientSecretHttp1RequestMaterial<'request>, ClientSecretHttp1BindingError> {
    let body = validate_http1_fixed_request_body(request.form_body().as_bytes(), body_limits)
        .map_err(|_| ClientSecretHttp1BindingError::BodyRejected)?;
    let encoded_length = body.content_length_value_length();
    let mut content_length_value = [0u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES];
    let length_value = encode_http1_fixed_request_content_length(
        &body,
        &mut content_length_value[..encoded_length],
    )
    .map_err(|_| ClientSecretHttp1BindingError::ContentLengthRejected)?;
    validate_http1_request_header_field(b"Content-Length", length_value, field_limits)
        .map_err(|_| ClientSecretHttp1BindingError::ContentLengthRejected)?;

    let authorization = request
        .authorization_header()
        .map(|value| {
            validate_http1_request_header_field(b"Authorization", value.as_bytes(), field_limits)
                .map_err(|_| ClientSecretHttp1BindingError::AuthorizationRejected)
        })
        .transpose()?;

    Ok(PreparedClientSecretHttp1RequestMaterial {
        request,
        body,
        authorization,
        content_length_value,
        content_length_value_length: encoded_length as u8,
        field_limits,
    })
}

fn release_request_material<'request, 'length>(
    prepared: PreparedClientSecretHttp1RequestMaterial<'request>,
    content_length_output: &'length mut [u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES],
) -> BoundClientSecretHttp1RequestMaterial<'request, 'length> {
    let encoded_length = prepared.content_length_value_length as usize;
    content_length_output[..encoded_length]
        .copy_from_slice(&prepared.content_length_value[..encoded_length]);
    let content_length = validate_http1_request_header_field(
        b"Content-Length",
        &content_length_output[..encoded_length],
        prepared.field_limits,
    )
    .expect("prepared Content-Length field remains valid in exact caller storage");
    let body = bind_http1_fixed_request_content_length_field(prepared.body, content_length)
        .expect("prepared canonical Content-Length remains bound to the exact body");

    BoundClientSecretHttp1RequestMaterial {
        request: prepared.request,
        body,
        authorization: prepared.authorization,
    }
}

fn publish<A: ClientSecretHttp1AuditSink>(
    audit: &mut A,
    request: &ClientSecretAuthenticatedRequest,
    outcome: ClientSecretHttp1AuditOutcome,
) -> Result<(), ClientSecretHttp1BindingError> {
    audit
        .publish(&ClientSecretHttp1AuditEvent {
            provider: request.provider().clone(),
            trace: request.trace(),
            action: ClientSecretHttp1AuditAction::BindRequestMaterial,
            outcome,
        })
        .map_err(|_| ClientSecretHttp1BindingError::Audit)
}
