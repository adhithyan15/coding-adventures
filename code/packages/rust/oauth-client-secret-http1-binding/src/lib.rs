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
    validate_http1_fixed_request_body, Http1FixedRequestBodyLimits,
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
    /// Every strict construction succeeded and a proof may be released.
    Succeeded,
    /// A bounded body, field, or Content-Length check rejected the request.
    Rejected,
}

/// Privacy-safe audit event for one request-material construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientSecretHttp1AuditEvent {
    provider: ProviderId,
    trace: OAuthTraceId,
    action: ClientSecretHttp1AuditAction,
    outcome: ClientSecretHttp1AuditOutcome,
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
            .field("provider", self.request.provider())
            .field("trace", &self.request.trace())
            .field("request", &"[REDACTED]")
            .field("body", &"[REDACTED]")
            .field("has_authorization", &self.authorization.is_some())
            .finish()
    }
}

/// Audit and construct strict HTTP/1 evidence from one custody-built request.
///
/// The attempted event is durable before credential-bearing bytes are
/// inspected. The seven-byte caller buffer is restored on every rejection or
/// audit failure. A succeeded event is durable before the proof is released.
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
    let original_output = *content_length_output;

    if let Err(error) =
        prepare_request_material(request, body_limits, field_limits, content_length_output)
    {
        *content_length_output = original_output;
        publish(audit, request, ClientSecretHttp1AuditOutcome::Rejected)?;
        return Err(error);
    }

    if publish(audit, request, ClientSecretHttp1AuditOutcome::Succeeded).is_err() {
        *content_length_output = original_output;
        return Err(ClientSecretHttp1BindingError::Audit);
    }

    construct_request_material(request, body_limits, field_limits, content_length_output)
}

fn prepare_request_material(
    request: &ClientSecretAuthenticatedRequest,
    body_limits: Http1FixedRequestBodyLimits,
    field_limits: Http1RequestHeaderFieldLimits,
    content_length_output: &mut [u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES],
) -> Result<(), ClientSecretHttp1BindingError> {
    let _prepared =
        construct_request_material(request, body_limits, field_limits, content_length_output)?;
    Ok(())
}

fn construct_request_material<'request, 'length>(
    request: &'request ClientSecretAuthenticatedRequest,
    body_limits: Http1FixedRequestBodyLimits,
    field_limits: Http1RequestHeaderFieldLimits,
    content_length_output: &'length mut [u8; MAX_HTTP1_FIXED_REQUEST_CONTENT_LENGTH_BYTES],
) -> Result<BoundClientSecretHttp1RequestMaterial<'request, 'length>, ClientSecretHttp1BindingError>
{
    let body = validate_http1_fixed_request_body(request.form_body().as_bytes(), body_limits)
        .map_err(|_| ClientSecretHttp1BindingError::BodyRejected)?;
    let encoded_length = body.content_length_value_length();
    let length_value = encode_http1_fixed_request_content_length(
        &body,
        &mut content_length_output[..encoded_length],
    )
    .map_err(|_| ClientSecretHttp1BindingError::ContentLengthRejected)?;
    let content_length =
        validate_http1_request_header_field(b"Content-Length", length_value, field_limits)
            .map_err(|_| ClientSecretHttp1BindingError::ContentLengthRejected)?;
    let body = bind_http1_fixed_request_content_length_field(body, content_length)
        .map_err(|_| ClientSecretHttp1BindingError::ContentLengthRejected)?;

    let authorization = request
        .authorization_header()
        .map(|value| {
            validate_http1_request_header_field(b"Authorization", value.as_bytes(), field_limits)
                .map_err(|_| ClientSecretHttp1BindingError::AuthorizationRejected)
        })
        .transpose()?;

    Ok(BoundClientSecretHttp1RequestMaterial {
        request,
        body,
        authorization,
    })
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
