//! Exact DNS destination binding across certificate SAN, TLS SNI, and HTTP Host.
//!
//! Each input reaches this crate only after its owning package has validated its
//! syntax. This crate adds the missing semantic equality and retains the exact
//! inputs in one non-copyable value so later composition cannot substitute a
//! different certificate, SNI, or Host authority. It can then bind that
//! complete destination to one strict canonical HTTP/1 Host field without
//! accepting a detached authority value, and bind that proof to an exact
//! bounded additional-field slice that contains no second Host field. The
//! complete field-set proof can then be joined to one exact fixed request body
//! and its canonical Content-Length field while rejecting framing ambiguity.
//! Finally, one exact validated request line classified as public by data-
//! driven provider policy can be retained with that complete proof and encoded
//! as one bounded request head in caller-owned storage. An additive audited
//! path admits exact provider-policy-bound additional fields only after a
//! provider/trace event publishes durably and before any output mutation.

#![forbid(unsafe_code)]

use coding_adventures_oauth::{OAuthTraceId, ProviderId};
use core::fmt;
use der_asn1::Asn1Limits;
use http1_fixed_request_body::Http1FixedRequestContentLengthField;
use http1_request_header_field::{
    encode_http1_request_header_field, Http1RequestHeaderField, MAX_HTTP1_REQUEST_HEADER_LINE_BYTES,
};
use http1_request_host_authority::Http1RequestHostAuthority;
use http1_request_line::{
    encode_http1_request_line, Http1RequestLine, MAX_HTTP1_REQUEST_LINE_BYTES,
};
use tls13_server_name::TlsServerName;
use tls_server_identity::ReferenceIdentity;
use x509_certificate::X509Certificate;
use x509_certificate_policy::{
    validate_certificate_server_identity, ValidatedCertificateServerIdentity,
    X509CertificateServerIdentityError,
};

/// Stable destination-binding failures that retain no identity input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpsDestinationNameBindingError {
    /// The validated TLS and HTTP DNS names do not identify the same host.
    ServerNameHostMismatch,
    /// The TLS name could not be represented as a certificate DNS reference.
    InvalidServerNameReference,
    /// The exact certificate did not satisfy SAN identity policy for the SNI.
    CertificateIdentity(X509CertificateServerIdentityError),
}

impl fmt::Display for HttpsDestinationNameBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ServerNameHostMismatch => {
                formatter.write_str("TLS server name and HTTP Host DNS name differ")
            }
            Self::InvalidServerNameReference => {
                formatter.write_str("TLS server name is not a certificate DNS reference")
            }
            Self::CertificateIdentity(_) => {
                formatter.write_str("certificate does not satisfy destination identity policy")
            }
        }
    }
}

impl std::error::Error for HttpsDestinationNameBindingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CertificateIdentity(error) => Some(error),
            Self::ServerNameHostMismatch | Self::InvalidServerNameReference => None,
        }
    }
}

/// Stable Host-field binding failures that retain no header or authority input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpsHostHeaderBindingError {
    /// The validated field name is not the HTTP Host field name.
    NotHostField,
    /// The field value is not the destination's exact canonical Host authority.
    HostAuthorityMismatch,
}

impl fmt::Display for HttpsHostHeaderBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotHostField => formatter.write_str("HTTP request field is not Host"),
            Self::HostAuthorityMismatch => {
                formatter.write_str("HTTP Host field does not match the HTTPS destination")
            }
        }
    }
}

impl std::error::Error for HttpsHostHeaderBindingError {}

/// Fixed maximum number of fields in one bound HTTPS request field set.
pub const MAX_HTTPS_REQUEST_HEADER_FIELDS: usize = 64;

/// Caller-owned total-field ceiling for one HTTPS request field set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpsRequestHeaderFieldsLimits {
    maximum_fields: u8,
}

impl HttpsRequestHeaderFieldsLimits {
    /// Validate the total-field ceiling, including the one retained Host field.
    pub fn new(maximum_fields: usize) -> Result<Self, HttpsRequestHeaderFieldsBindingError> {
        if maximum_fields == 0 || maximum_fields > MAX_HTTPS_REQUEST_HEADER_FIELDS {
            return Err(HttpsRequestHeaderFieldsBindingError::InvalidLimits);
        }
        Ok(Self {
            maximum_fields: maximum_fields as u8,
        })
    }

    /// Return the total-field ceiling, including the retained Host field.
    #[must_use]
    pub const fn maximum_fields(self) -> usize {
        self.maximum_fields as usize
    }
}

/// Stable request-field-set failures that retain no field input or observed count.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpsRequestHeaderFieldsBindingError {
    /// The total-field ceiling is zero or exceeds the fixed package maximum.
    InvalidLimits,
    /// The supplied field set exceeds the caller's total-field ceiling.
    TooManyFields,
    /// A field other than the retained canonical field is also named Host.
    AdditionalHostField,
}

impl fmt::Display for HttpsRequestHeaderFieldsBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimits => "invalid HTTPS request-header field-set limits",
            Self::TooManyFields => "HTTPS request-header field set exceeds its limit",
            Self::AdditionalHostField => "additional HTTP Host field is forbidden",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for HttpsRequestHeaderFieldsBindingError {}

/// Stable fixed request-body binding failures that retain no request input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpsFixedRequestBodyBindingError {
    /// Host, Content-Length, and additional fields exceed the retained limit.
    TooManyFields,
    /// The additional slice contains another Content-Length field.
    AdditionalContentLengthField,
    /// The additional slice contains a forbidden Transfer-Encoding field.
    TransferEncodingField,
}

impl fmt::Display for HttpsFixedRequestBodyBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::TooManyFields => "HTTPS fixed request field set exceeds its retained limit",
            Self::AdditionalContentLengthField => {
                "additional HTTP Content-Length field is forbidden for a fixed request body"
            }
            Self::TransferEncodingField => {
                "transfer coding is forbidden for an HTTPS fixed request body"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for HttpsFixedRequestBodyBindingError {}

/// Largest complete request head admitted by this deliberately narrow boundary.
///
/// The ceiling accounts for one maximum request line, the maximum complete
/// field set, and the final CRLF that terminates the section.
pub const MAX_HTTPS_FIXED_REQUEST_HEAD_BYTES: usize = MAX_HTTP1_REQUEST_LINE_BYTES
    + (MAX_HTTPS_REQUEST_HEADER_FIELDS * MAX_HTTP1_REQUEST_HEADER_LINE_BYTES)
    + 2;

/// Data-driven provider policy for one public, query-free request line.
///
/// Construction classifies only a provider-configured method and origin-form
/// path as public request metadata. Credential-bearing targets require a later
/// provider/trace-audited release boundary and cannot be represented here.
pub struct HttpsPublicRequestLinePolicy<'input> {
    method: &'input [u8],
    target: &'input [u8],
}

impl<'input> HttpsPublicRequestLinePolicy<'input> {
    /// Classify one provider-configured method and query-free path as public.
    pub fn new(
        method: &'input [u8],
        target: &'input [u8],
    ) -> Result<Self, HttpsPublicRequestLineBindingError> {
        if method.is_empty() || target.first() != Some(&b'/') || target.contains(&b'?') {
            return Err(HttpsPublicRequestLineBindingError::InvalidPolicy);
        }
        Ok(Self { method, target })
    }
}

impl fmt::Debug for HttpsPublicRequestLinePolicy<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpsPublicRequestLinePolicy")
            .field("method", &"[REDACTED]")
            .field("target", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Stable public request-line binding failures without policy or line bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpsPublicRequestLineBindingError {
    /// The policy method is empty or its target is not a query-free path.
    InvalidPolicy,
    /// The validated request line differs from the exact provider policy.
    PolicyMismatch,
}

impl fmt::Display for HttpsPublicRequestLineBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidPolicy => "invalid HTTPS public request-line policy",
            Self::PolicyMismatch => "HTTPS request line differs from public provider policy",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for HttpsPublicRequestLineBindingError {}

/// Caller-owned complete-head byte ceiling for one fixed HTTPS request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpsFixedRequestHeadLimits {
    maximum_head_bytes: u32,
}

impl HttpsFixedRequestHeadLimits {
    /// Validate the complete request-head ceiling.
    pub fn new(maximum_head_bytes: usize) -> Result<Self, HttpsFixedRequestHeadError> {
        if maximum_head_bytes == 0 || maximum_head_bytes > MAX_HTTPS_FIXED_REQUEST_HEAD_BYTES {
            return Err(HttpsFixedRequestHeadError::InvalidLimits);
        }
        Ok(Self {
            maximum_head_bytes: maximum_head_bytes as u32,
        })
    }

    /// Return the complete request-head byte ceiling.
    #[must_use]
    pub const fn maximum_head_bytes(self) -> usize {
        self.maximum_head_bytes as usize
    }
}

/// Stable request-head failures that retain no request input or observed size.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpsFixedRequestHeadError {
    /// The caller's complete-head ceiling is zero or exceeds the fixed maximum.
    InvalidLimits,
    /// The exact request head exceeds the caller's retained ceiling.
    HeadTooLong,
    /// Additional fields require a separate audited release composition.
    AdditionalFieldsNotAllowed,
    /// The caller-owned output cannot hold the complete exact request head.
    OutputTooSmall,
}

impl fmt::Display for HttpsFixedRequestHeadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimits => "invalid HTTPS fixed request-head limits",
            Self::HeadTooLong => "HTTPS fixed request head exceeds its byte limit",
            Self::AdditionalFieldsNotAllowed => {
                "HTTPS fixed request head contains unaudited additional fields"
            }
            Self::OutputTooSmall => "HTTPS fixed request-head output is too small",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for HttpsFixedRequestHeadError {}

/// Exact provider policy for releasing one request head with public fields.
///
/// The provider, request line, destination, and ordered exact field allowlist
/// are configuration data. Only closed, non-credential HTTP metadata names are
/// admitted. Credential-bearing fields require a higher-level custody-bound
/// provider/trace proof and cannot pass through this low-level release.
pub struct HttpsAuditedRequestHeadPolicy<'policy> {
    provider: &'policy ProviderId,
    request_line: HttpsPublicRequestLinePolicy<'policy>,
    destination: Http1RequestHostAuthority<'policy>,
    additional_fields: &'policy [Http1RequestHeaderField<'policy>],
}

impl<'policy> HttpsAuditedRequestHeadPolicy<'policy> {
    /// Validate one exact provider request-head release policy.
    ///
    /// At least one additional field is required because the unaudited public
    /// request-head boundary already covers the empty-field case. Every exact
    /// field name and value is retained as public provider policy. Only `Accept`
    /// and `Content-Type` are admitted; credential-bearing and custom fields
    /// require a separate provenance-bound composition.
    pub fn new(
        provider: &'policy ProviderId,
        method: &'policy [u8],
        target: &'policy [u8],
        destination: Http1RequestHostAuthority<'policy>,
        additional_fields: &'policy [Http1RequestHeaderField<'policy>],
    ) -> Result<Self, HttpsAuditedRequestHeadPolicyError> {
        let request_line = HttpsPublicRequestLinePolicy::new(method, target)
            .map_err(|_| HttpsAuditedRequestHeadPolicyError::InvalidRequestLine)?;
        if additional_fields.is_empty() {
            return Err(HttpsAuditedRequestHeadPolicyError::NoAdditionalFields);
        }
        if additional_fields.len() > MAX_HTTPS_REQUEST_HEADER_FIELDS - 2 {
            return Err(HttpsAuditedRequestHeadPolicyError::TooManyFields);
        }

        for (index, field) in additional_fields.iter().enumerate() {
            if !is_public_additional_field_name(field.name()) {
                return Err(HttpsAuditedRequestHeadPolicyError::UnsupportedPublicField);
            }
            if additional_fields[..index]
                .iter()
                .any(|previous| previous.name().eq_ignore_ascii_case(field.name()))
            {
                return Err(HttpsAuditedRequestHeadPolicyError::DuplicateFieldName);
            }
        }

        Ok(Self {
            provider,
            request_line,
            destination,
            additional_fields,
        })
    }

    /// Return the exact validated provider represented by this policy.
    #[must_use]
    pub const fn provider(&self) -> &ProviderId {
        self.provider
    }
}

impl fmt::Debug for HttpsAuditedRequestHeadPolicy<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpsAuditedRequestHeadPolicy")
            .field("provider", self.provider)
            .field("request_line", &"[REDACTED]")
            .field("destination", &"[REDACTED]")
            .field("additional_fields", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Stable provider-policy failures without request, destination, or field data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpsAuditedRequestHeadPolicyError {
    /// Method or target is not valid public query-free provider policy.
    InvalidRequestLine,
    /// The audited path would be redundant with the empty-field public path.
    NoAdditionalFields,
    /// Host and Content-Length leave no room for the configured field count.
    TooManyFields,
    /// A field is not closed public request metadata admitted by this boundary.
    UnsupportedPublicField,
    /// Two configured names are equal under HTTP field-name semantics.
    DuplicateFieldName,
}

impl fmt::Display for HttpsAuditedRequestHeadPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidRequestLine => "invalid audited HTTPS request-line policy",
            Self::NoAdditionalFields => {
                "audited HTTPS request-head policy has no additional fields"
            }
            Self::TooManyFields => "audited HTTPS request-head policy has too many fields",
            Self::UnsupportedPublicField => {
                "audited HTTPS request-head policy contains a non-public field"
            }
            Self::DuplicateFieldName => "audited HTTPS request-head policy has a duplicate field",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for HttpsAuditedRequestHeadPolicyError {}

/// Stable privacy-safe action written before one request-head byte release.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpsRequestHeadAuditAction {
    /// Release one provider-policy-bound fixed request head with extra fields.
    AdditionalFieldsRelease,
}

/// Closed failure class retained by a privacy-safe request-head audit event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpsRequestHeadAuditFailure {
    /// Request, destination, or field names did not match provider policy.
    PolicyMismatch,
    /// A caller-owned length or output limit rejected the request.
    ResourceLimit,
}

/// Privacy-safe outcome written before any request-head output is mutated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpsRequestHeadAuditOutcome {
    /// Exact policy binding and every complete-output preflight succeeded.
    Succeeded,
    /// The release failed closed before any output mutation.
    Failed(HttpsRequestHeadAuditFailure),
}

/// Provider- and trace-bound event containing no request or credential bytes.
pub struct HttpsRequestHeadAuditEvent<'provider> {
    provider: &'provider ProviderId,
    trace: OAuthTraceId,
    action: HttpsRequestHeadAuditAction,
    outcome: HttpsRequestHeadAuditOutcome,
}

impl HttpsRequestHeadAuditEvent<'_> {
    /// Return the exact provider policy whose request release was attempted.
    #[must_use]
    pub const fn provider(&self) -> &ProviderId {
        self.provider
    }

    /// Return the caller-owned trace correlating this exact release attempt.
    #[must_use]
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Return the stable request-head release action.
    #[must_use]
    pub const fn action(&self) -> HttpsRequestHeadAuditAction {
        self.action
    }

    /// Return the closed outcome without request, field, or credential data.
    #[must_use]
    pub const fn outcome(&self) -> HttpsRequestHeadAuditOutcome {
        self.outcome
    }
}

impl fmt::Debug for HttpsRequestHeadAuditEvent<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpsRequestHeadAuditEvent")
            .field("provider", self.provider)
            .field("trace", &self.trace)
            .field("action", &self.action)
            .field("outcome", &self.outcome)
            .finish()
    }
}

/// Durable publication required before one additional-field head is released.
pub trait HttpsRequestHeadAuditSink {
    /// Persist the complete privacy-safe event durably or fail closed.
    fn publish(
        &mut self,
        event: &HttpsRequestHeadAuditEvent<'_>,
    ) -> Result<(), HttpsRequestHeadAuditError>;
}

/// Closed audit-publication failure without sink diagnostics or request data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpsRequestHeadAuditError;

/// Stable audited request-head failures without request or credential payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpsAuditedRequestHeadError {
    /// The retained request line differs from exact provider policy.
    RequestLinePolicyMismatch,
    /// The retained Host authority differs from exact provider policy.
    DestinationPolicyMismatch,
    /// Additional field count, order, or names differ from provider policy.
    AdditionalFieldsPolicyMismatch,
    /// The exact encoded head exceeds the retained complete-head ceiling.
    HeadTooLong,
    /// Caller-owned output cannot hold the complete exact request head.
    OutputTooSmall,
    /// Durable provider/trace audit publication failed before output mutation.
    Audit,
}

impl HttpsAuditedRequestHeadError {
    const fn failure_class(self) -> Option<HttpsRequestHeadAuditFailure> {
        match self {
            Self::RequestLinePolicyMismatch
            | Self::DestinationPolicyMismatch
            | Self::AdditionalFieldsPolicyMismatch => {
                Some(HttpsRequestHeadAuditFailure::PolicyMismatch)
            }
            Self::HeadTooLong | Self::OutputTooSmall => {
                Some(HttpsRequestHeadAuditFailure::ResourceLimit)
            }
            Self::Audit => None,
        }
    }
}

impl fmt::Display for HttpsAuditedRequestHeadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::RequestLinePolicyMismatch => {
                "HTTPS request line differs from audited provider policy"
            }
            Self::DestinationPolicyMismatch => {
                "HTTPS destination differs from audited provider policy"
            }
            Self::AdditionalFieldsPolicyMismatch => {
                "HTTPS additional fields differ from audited provider policy"
            }
            Self::HeadTooLong => "audited HTTPS request head exceeds its byte limit",
            Self::OutputTooSmall => "audited HTTPS request-head output is too small",
            Self::Audit => "audited HTTPS request-head publication failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for HttpsAuditedRequestHeadError {}

/// One exactly emitted request head released only after durable provider audit.
///
/// This wrapper is non-copyable and redacts its complete emitted bytes. This
/// low-level path excludes credential-bearing fields and grants no body-write,
/// TLS, HTTP I/O, or transport authority.
pub struct EncodedHttpsAuditedRequestHead<'output, 'provider> {
    provider: &'provider ProviderId,
    trace: OAuthTraceId,
    encoded: &'output mut [u8],
}

impl EncodedHttpsAuditedRequestHead<'_, '_> {
    /// Return the exact provider bound to the successful release audit.
    #[must_use]
    pub const fn provider(&self) -> &ProviderId {
        self.provider
    }

    /// Return the exact trace bound to the successful release audit.
    #[must_use]
    pub const fn trace(&self) -> OAuthTraceId {
        self.trace
    }

    /// Borrow the exact complete emitted request-head bytes after audit.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.encoded
    }
}

impl fmt::Debug for EncodedHttpsAuditedRequestHead<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EncodedHttpsAuditedRequestHead")
            .field("provider", self.provider)
            .field("trace", &self.trace)
            .field("encoded", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// Non-copyable proof that one exact certificate, SNI, and Host share a DNS name.
///
/// This value proves only destination-name consistency and complete-certificate
/// SAN matching. It grants no certificate path, signature, role, validity,
/// revocation, trust, TLS, HTTP, OAuth, I/O, credential, audit, trace, or
/// external-effect authority.
pub struct BoundHttpsDnsDestination<'certificate, 'encoded, 'server_name, 'host> {
    certificate: &'certificate X509Certificate<'encoded>,
    server_name: TlsServerName<'server_name>,
    host_authority: Http1RequestHostAuthority<'host>,
    certificate_identity: ValidatedCertificateServerIdentity,
}

/// One exact HTTPS DNS destination bound to one canonical HTTP/1 Host field.
///
/// This non-copyable value retains one field only. It does not prove that a
/// later request head contains no additional Host field and grants no request,
/// TLS, HTTP, OAuth, I/O, credential, audit, trace, or external-effect
/// authority.
pub struct BoundHttpsHostHeaderField<'certificate, 'encoded, 'server_name, 'host, 'field> {
    destination: BoundHttpsDnsDestination<'certificate, 'encoded, 'server_name, 'host>,
    host_field: Http1RequestHeaderField<'field>,
}

/// One HTTPS destination bound to the complete retained request field set.
///
/// The field set contains exactly one Host field: the canonical field inside
/// `host_binding`. The exact additional validated-field slice and the caller's
/// field-count policy are retained without authorizing any additional field or
/// granting request, TLS, HTTP, OAuth, I/O, credential, audit, trace, or
/// external-effect authority.
pub struct BoundHttpsRequestHeaderFields<
    'certificate,
    'encoded,
    'server_name,
    'host,
    'host_field,
    'fields,
    'additional_field,
> {
    host_binding:
        BoundHttpsHostHeaderField<'certificate, 'encoded, 'server_name, 'host, 'host_field>,
    additional_fields: &'fields [Http1RequestHeaderField<'additional_field>],
    limits: HttpsRequestHeaderFieldsLimits,
}

/// One exact HTTPS field-set proof bound to one exact fixed request body.
///
/// This non-copyable proof defines the logical field order as the retained Host
/// field, the retained canonical Content-Length field, and then the retained
/// additional fields in caller order. It proves no field authorization,
/// request-line binding, encoding, TLS, HTTP, OAuth, I/O, credential, audit,
/// trace, or external-effect authority.
pub struct BoundHttpsFixedRequestBody<
    'certificate,
    'encoded,
    'server_name,
    'host,
    'host_field,
    'fields,
    'additional_field,
    'body,
    'content_length_field,
> {
    header_fields: BoundHttpsRequestHeaderFields<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
    >,
    fixed_body: Http1FixedRequestContentLengthField<'body, 'content_length_field>,
}

/// One exact validated request line classified by public provider policy.
///
/// The proof is non-copyable. It retains only the exact query-free request
/// metadata explicitly classified as public by the consumed provider policy.
pub struct BoundHttpsPublicRequestLine<'request_line> {
    request_line: Http1RequestLine<'request_line>,
}

impl<'request_line> BoundHttpsPublicRequestLine<'request_line> {
    /// Return the exact retained validated request line.
    #[must_use]
    pub const fn request_line(&self) -> &Http1RequestLine<'request_line> {
        &self.request_line
    }
}

impl fmt::Debug for BoundHttpsPublicRequestLine<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundHttpsPublicRequestLine")
            .field("request_line", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

/// One exact validated request line bound to one exact fixed HTTPS request.
///
/// This non-copyable proof retains the public-policy-bound request line,
/// destination, Host field, canonical Content-Length field, empty additional
/// field slice, body, and exact encoded head length. It grants no TLS, HTTP,
/// OAuth, I/O, credential, audit, trace, or external-effect authority.
pub struct BoundHttpsFixedRequestHead<
    'request_line,
    'certificate,
    'encoded,
    'server_name,
    'host,
    'host_field,
    'fields,
    'additional_field,
    'body,
    'content_length_field,
> {
    request_line: BoundHttpsPublicRequestLine<'request_line>,
    fixed_request_body: BoundHttpsFixedRequestBody<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
        'body,
        'content_length_field,
    >,
    encoded_head_length: u32,
}

impl<
        'request_line,
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
        'body,
        'content_length_field,
    >
    BoundHttpsFixedRequestHead<
        'request_line,
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
        'body,
        'content_length_field,
    >
{
    /// Return the exact retained validated request line.
    #[must_use]
    pub const fn request_line(&self) -> &BoundHttpsPublicRequestLine<'request_line> {
        &self.request_line
    }

    /// Return the exact retained fixed-body destination proof.
    #[must_use]
    pub const fn fixed_request_body(
        &self,
    ) -> &BoundHttpsFixedRequestBody<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
        'body,
        'content_length_field,
    > {
        &self.fixed_request_body
    }

    /// Return the exact complete-head byte length, including its final CRLF.
    #[must_use]
    pub const fn encoded_head_length(&self) -> usize {
        self.encoded_head_length as usize
    }
}

impl fmt::Debug for BoundHttpsFixedRequestHead<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundHttpsFixedRequestHead")
            .field("request_line", &"[REDACTED]")
            .field("fixed_request_body", &"[REDACTED]")
            .field("encoded_head_length", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
        'body,
        'content_length_field,
    >
    BoundHttpsFixedRequestBody<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
        'body,
        'content_length_field,
    >
{
    /// Return the exact retained Host-plus-additional field-set proof.
    #[must_use]
    pub const fn header_fields(
        &self,
    ) -> &BoundHttpsRequestHeaderFields<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
    > {
        &self.header_fields
    }

    /// Return the exact retained body and canonical Content-Length proof.
    #[must_use]
    pub const fn fixed_body(
        &self,
    ) -> &Http1FixedRequestContentLengthField<'body, 'content_length_field> {
        &self.fixed_body
    }

    /// Return Host plus Content-Length plus every retained additional field.
    #[must_use]
    pub const fn total_field_count(&self) -> usize {
        self.header_fields.additional_fields.len() + 2
    }
}

impl fmt::Debug for BoundHttpsFixedRequestBody<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundHttpsFixedRequestBody")
            .field("header_fields", &"[REDACTED]")
            .field("fixed_body", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl<'certificate, 'encoded, 'server_name, 'host, 'host_field, 'fields, 'additional_field>
    BoundHttpsRequestHeaderFields<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
    >
{
    /// Return the complete destination and canonical Host-field binding.
    #[must_use]
    pub const fn host_binding(
        &self,
    ) -> &BoundHttpsHostHeaderField<'certificate, 'encoded, 'server_name, 'host, 'host_field> {
        &self.host_binding
    }

    /// Return the exact additional validated fields in caller order.
    #[must_use]
    pub const fn additional_fields(&self) -> &'fields [Http1RequestHeaderField<'additional_field>] {
        self.additional_fields
    }

    /// Return the exact caller field-count policy retained by this proof.
    #[must_use]
    pub const fn limits(&self) -> HttpsRequestHeaderFieldsLimits {
        self.limits
    }
}

impl fmt::Debug for BoundHttpsRequestHeaderFields<'_, '_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundHttpsRequestHeaderFields")
            .field("host_binding", &"[REDACTED]")
            .field("additional_fields", &"[REDACTED]")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl<'certificate, 'encoded, 'server_name, 'host, 'field>
    BoundHttpsHostHeaderField<'certificate, 'encoded, 'server_name, 'host, 'field>
{
    /// Return the complete destination retained by this Host-field proof.
    #[must_use]
    pub const fn destination(
        &self,
    ) -> &BoundHttpsDnsDestination<'certificate, 'encoded, 'server_name, 'host> {
        &self.destination
    }

    /// Return the exact validated Host field retained by this proof.
    #[must_use]
    pub const fn host_field(&self) -> &Http1RequestHeaderField<'field> {
        &self.host_field
    }
}

impl fmt::Debug for BoundHttpsHostHeaderField<'_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundHttpsHostHeaderField")
            .field("destination", &"[REDACTED]")
            .field("host_field", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl<'certificate, 'encoded, 'server_name, 'host>
    BoundHttpsDnsDestination<'certificate, 'encoded, 'server_name, 'host>
{
    /// Return the exact decoded certificate whose SAN was matched.
    #[must_use]
    pub const fn certificate(&self) -> &'certificate X509Certificate<'encoded> {
        self.certificate
    }

    /// Return the exact validated SNI DNS name used as the certificate reference.
    #[must_use]
    pub const fn server_name(&self) -> TlsServerName<'server_name> {
        self.server_name
    }

    /// Return the exact validated HTTP Host authority, including its optional port.
    #[must_use]
    pub const fn host_authority(&self) -> &Http1RequestHostAuthority<'host> {
        &self.host_authority
    }

    /// Return the SAN identity evidence produced for the retained certificate.
    #[must_use]
    pub const fn certificate_identity(&self) -> ValidatedCertificateServerIdentity {
        self.certificate_identity
    }
}

impl fmt::Debug for BoundHttpsDnsDestination<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BoundHttpsDnsDestination")
            .field("certificate", &"[REDACTED]")
            .field("server_name", &"[REDACTED]")
            .field("host_authority", &"[REDACTED]")
            .field("certificate_identity", &self.certificate_identity)
            .finish_non_exhaustive()
    }
}

/// Bind one exact certificate, TLS SNI, and HTTP Host to the same DNS identity.
///
/// SNI and Host are compared under ASCII case-insensitive DNS semantics. The
/// SNI then becomes the sole certificate reference, and SAN matching is rerun
/// against the exact retained certificate. The Host port is retained but is not
/// part of DNS identity comparison.
pub fn bind_https_dns_destination<'certificate, 'encoded, 'server_name, 'host>(
    certificate: &'certificate X509Certificate<'encoded>,
    server_name: TlsServerName<'server_name>,
    host_authority: Http1RequestHostAuthority<'host>,
    san_limits: Asn1Limits,
) -> Result<
    BoundHttpsDnsDestination<'certificate, 'encoded, 'server_name, 'host>,
    HttpsDestinationNameBindingError,
> {
    if !server_name
        .as_str()
        .as_bytes()
        .eq_ignore_ascii_case(host_authority.host())
    {
        return Err(HttpsDestinationNameBindingError::ServerNameHostMismatch);
    }

    let reference = ReferenceIdentity::parse(server_name.as_str())
        .map_err(|_| HttpsDestinationNameBindingError::InvalidServerNameReference)?;
    let certificate_identity =
        validate_certificate_server_identity(certificate, &reference, san_limits)
            .map_err(HttpsDestinationNameBindingError::CertificateIdentity)?;

    Ok(BoundHttpsDnsDestination {
        certificate,
        server_name,
        host_authority,
        certificate_identity,
    })
}

/// Bind one complete HTTPS destination to one canonical HTTP/1 Host field.
///
/// Field-name comparison follows HTTP's ASCII case-insensitive semantics. The
/// value must equal the exact encoding owned by the retained Host authority:
/// lowercase DNS bytes followed, when present, by a colon and the port's
/// shortest decimal representation. The function accepts no detached hostname
/// or port claim.
pub fn bind_https_host_header_field<'certificate, 'encoded, 'server_name, 'host, 'field>(
    destination: BoundHttpsDnsDestination<'certificate, 'encoded, 'server_name, 'host>,
    host_field: Http1RequestHeaderField<'field>,
) -> Result<
    BoundHttpsHostHeaderField<'certificate, 'encoded, 'server_name, 'host, 'field>,
    HttpsHostHeaderBindingError,
> {
    if !host_field.name().eq_ignore_ascii_case(b"Host") {
        return Err(HttpsHostHeaderBindingError::NotHostField);
    }
    if !matches_canonical_host_authority(destination.host_authority(), host_field.value()) {
        return Err(HttpsHostHeaderBindingError::HostAuthorityMismatch);
    }

    Ok(BoundHttpsHostHeaderField {
        destination,
        host_field,
    })
}

/// Bind one canonical HTTPS Host field to the complete additional field slice.
///
/// The caller's total-field ceiling includes the canonical Host field. Every
/// additional field name is compared using HTTP's ASCII case-insensitive
/// semantics, and any second Host field fails closed. Other duplicate names,
/// field authorization, request-line and body-framing policy, encoding, and
/// transport remain outside this boundary.
pub fn bind_https_request_header_fields<
    'certificate,
    'encoded,
    'server_name,
    'host,
    'host_field,
    'fields,
    'additional_field,
>(
    host_binding: BoundHttpsHostHeaderField<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
    >,
    additional_fields: &'fields [Http1RequestHeaderField<'additional_field>],
    limits: HttpsRequestHeaderFieldsLimits,
) -> Result<
    BoundHttpsRequestHeaderFields<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
    >,
    HttpsRequestHeaderFieldsBindingError,
> {
    let maximum_additional_fields = limits.maximum_fields() - 1;
    if additional_fields.len() > maximum_additional_fields {
        return Err(HttpsRequestHeaderFieldsBindingError::TooManyFields);
    }
    if additional_fields
        .iter()
        .any(|field| field.name().eq_ignore_ascii_case(b"Host"))
    {
        return Err(HttpsRequestHeaderFieldsBindingError::AdditionalHostField);
    }

    Ok(BoundHttpsRequestHeaderFields {
        host_binding,
        additional_fields,
        limits,
    })
}

/// Bind one exact HTTPS field set to one exact fixed request body.
///
/// The canonical Content-Length field carried by `fixed_body` counts inside
/// the existing total-field ceiling. Every additional field is checked under
/// HTTP ASCII case-insensitive field-name semantics. A second Content-Length
/// field or any Transfer-Encoding field fails closed.
pub fn bind_https_fixed_request_body<
    'certificate,
    'encoded,
    'server_name,
    'host,
    'host_field,
    'fields,
    'additional_field,
    'body,
    'content_length_field,
>(
    header_fields: BoundHttpsRequestHeaderFields<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
    >,
    fixed_body: Http1FixedRequestContentLengthField<'body, 'content_length_field>,
) -> Result<
    BoundHttpsFixedRequestBody<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
        'body,
        'content_length_field,
    >,
    HttpsFixedRequestBodyBindingError,
> {
    let total_fields = header_fields
        .additional_fields()
        .len()
        .checked_add(2)
        .ok_or(HttpsFixedRequestBodyBindingError::TooManyFields)?;
    if total_fields > header_fields.limits().maximum_fields() {
        return Err(HttpsFixedRequestBodyBindingError::TooManyFields);
    }
    if header_fields
        .additional_fields()
        .iter()
        .any(|field| field.name().eq_ignore_ascii_case(b"Content-Length"))
    {
        return Err(HttpsFixedRequestBodyBindingError::AdditionalContentLengthField);
    }
    if header_fields
        .additional_fields()
        .iter()
        .any(|field| field.name().eq_ignore_ascii_case(b"Transfer-Encoding"))
    {
        return Err(HttpsFixedRequestBodyBindingError::TransferEncodingField);
    }

    Ok(BoundHttpsFixedRequestBody {
        header_fields,
        fixed_body,
    })
}

/// Bind one validated request line to exact public provider policy.
///
/// The policy is consumed and must contain a query-free origin-form path. A
/// mismatch fails without retaining or disclosing either input.
pub fn bind_https_public_request_line<'request_line>(
    request_line: Http1RequestLine<'request_line>,
    policy: HttpsPublicRequestLinePolicy<'_>,
) -> Result<BoundHttpsPublicRequestLine<'request_line>, HttpsPublicRequestLineBindingError> {
    if request_line.method() != policy.method || request_line.target() != policy.target {
        return Err(HttpsPublicRequestLineBindingError::PolicyMismatch);
    }
    Ok(BoundHttpsPublicRequestLine { request_line })
}

/// Bind one exact public-policy request line to one exact fixed HTTPS request.
///
/// The complete encoded head is preflighted under the caller's bounded byte
/// ceiling. Its exact wire order is request line, Host, Content-Length, and one
/// final CRLF. Additional fields fail closed because potentially credential-
/// bearing fields require a separate provider/trace-audited release proof. The
/// body is retained by the result but remains outside the encoded head.
pub fn bind_https_fixed_request_head<
    'request_line,
    'certificate,
    'encoded,
    'server_name,
    'host,
    'host_field,
    'fields,
    'additional_field,
    'body,
    'content_length_field,
>(
    request_line: BoundHttpsPublicRequestLine<'request_line>,
    fixed_request_body: BoundHttpsFixedRequestBody<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
        'body,
        'content_length_field,
    >,
    limits: HttpsFixedRequestHeadLimits,
) -> Result<
    BoundHttpsFixedRequestHead<
        'request_line,
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
        'body,
        'content_length_field,
    >,
    HttpsFixedRequestHeadError,
> {
    let header_fields = fixed_request_body.header_fields();
    if !header_fields.additional_fields().is_empty() {
        return Err(HttpsFixedRequestHeadError::AdditionalFieldsNotAllowed);
    }
    let encoded_head_length = request_line
        .request_line()
        .encoded_length()
        .checked_add(header_fields.host_binding().host_field().encoded_length())
        .and_then(|length| {
            length.checked_add(fixed_request_body.fixed_body().field().encoded_length())
        })
        .and_then(|length| length.checked_add(2))
        .ok_or(HttpsFixedRequestHeadError::HeadTooLong)?;
    if encoded_head_length > limits.maximum_head_bytes() {
        return Err(HttpsFixedRequestHeadError::HeadTooLong);
    }

    Ok(BoundHttpsFixedRequestHead {
        request_line,
        fixed_request_body,
        encoded_head_length: encoded_head_length as u32,
    })
}

/// Encode one exact bound fixed HTTPS request head into caller-owned storage.
///
/// The output remains unchanged when its capacity is insufficient. On success,
/// the returned slice covers only the complete head; the retained body is not
/// written or otherwise exposed.
pub fn encode_https_fixed_request_head<'output>(
    request: &BoundHttpsFixedRequestHead<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    output: &'output mut [u8],
) -> Result<&'output mut [u8], HttpsFixedRequestHeadError> {
    let encoded_head_length = request.encoded_head_length();
    if output.len() < encoded_head_length {
        return Err(HttpsFixedRequestHeadError::OutputTooSmall);
    }

    let output = &mut output[..encoded_head_length];
    let mut offset = 0usize;
    let encoded =
        encode_http1_request_line(request.request_line().request_line(), &mut output[offset..])
            .map_err(|_| HttpsFixedRequestHeadError::OutputTooSmall)?;
    offset += encoded.len();
    let fixed_request_body = request.fixed_request_body();
    let header_fields = fixed_request_body.header_fields();
    let encoded = encode_http1_request_header_field(
        header_fields.host_binding().host_field(),
        &mut output[offset..],
    )
    .map_err(|_| HttpsFixedRequestHeadError::OutputTooSmall)?;
    offset += encoded.len();
    let encoded = encode_http1_request_header_field(
        fixed_request_body.fixed_body().field(),
        &mut output[offset..],
    )
    .map_err(|_| HttpsFixedRequestHeadError::OutputTooSmall)?;
    offset += encoded.len();
    output[offset..offset + 2].copy_from_slice(b"\r\n");
    offset += 2;
    debug_assert_eq!(offset, encoded_head_length);
    Ok(output)
}

/// Audit and encode one exact provider-policy-bound request head.
///
/// The method, query-free target, canonical Host authority, and complete
/// additional-field sequence must match the consumed provider policy by exact
/// public name, value, and order.
/// Complete length and output-capacity checks happen before the audit effect.
/// The privacy-safe provider/trace event must then publish durably before any
/// provider-approved public field bytes are copied to caller storage.
/// Every call consumes its inputs and performs its own audit, preventing one
/// successful publication from authorizing another output release.
#[allow(clippy::too_many_arguments)]
pub fn encode_https_audited_fixed_request_head<
    'output,
    'policy,
    'request_line,
    'certificate,
    'encoded,
    'server_name,
    'host,
    'host_field,
    'fields,
    'additional_field,
    'body,
    'content_length_field,
    A: HttpsRequestHeadAuditSink,
>(
    policy: HttpsAuditedRequestHeadPolicy<'policy>,
    request_line: BoundHttpsPublicRequestLine<'request_line>,
    fixed_request_body: BoundHttpsFixedRequestBody<
        'certificate,
        'encoded,
        'server_name,
        'host,
        'host_field,
        'fields,
        'additional_field,
        'body,
        'content_length_field,
    >,
    limits: HttpsFixedRequestHeadLimits,
    trace: OAuthTraceId,
    audit: &mut A,
    output: &'output mut [u8],
) -> Result<EncodedHttpsAuditedRequestHead<'output, 'policy>, HttpsAuditedRequestHeadError> {
    let prepared = prepare_audited_request_head(
        &policy,
        &request_line,
        &fixed_request_body,
        limits,
        output.len(),
    );
    let outcome = match prepared {
        Ok(_) => HttpsRequestHeadAuditOutcome::Succeeded,
        Err(error) => match error.failure_class() {
            Some(failure) => HttpsRequestHeadAuditOutcome::Failed(failure),
            None => return Err(error),
        },
    };
    audit
        .publish(&HttpsRequestHeadAuditEvent {
            provider: policy.provider,
            trace,
            action: HttpsRequestHeadAuditAction::AdditionalFieldsRelease,
            outcome,
        })
        .map_err(|_| HttpsAuditedRequestHeadError::Audit)?;
    let encoded_head_length = prepared?;

    let output = &mut output[..encoded_head_length];
    let mut offset = 0usize;
    offset += encode_preflighted_request_line(request_line.request_line(), &mut output[offset..]);

    let header_fields = fixed_request_body.header_fields();
    offset += encode_preflighted_header_field(
        header_fields.host_binding().host_field(),
        &mut output[offset..],
    );
    offset += encode_preflighted_header_field(
        fixed_request_body.fixed_body().field(),
        &mut output[offset..],
    );
    for field in header_fields.additional_fields() {
        offset += encode_preflighted_header_field(field, &mut output[offset..]);
    }
    output[offset..offset + 2].copy_from_slice(b"\r\n");
    offset += 2;
    debug_assert_eq!(offset, encoded_head_length);

    Ok(EncodedHttpsAuditedRequestHead {
        provider: policy.provider,
        trace,
        encoded: output,
    })
}

fn prepare_audited_request_head(
    policy: &HttpsAuditedRequestHeadPolicy<'_>,
    request_line: &BoundHttpsPublicRequestLine<'_>,
    fixed_request_body: &BoundHttpsFixedRequestBody<'_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: HttpsFixedRequestHeadLimits,
    output_length: usize,
) -> Result<usize, HttpsAuditedRequestHeadError> {
    let retained_request_line = request_line.request_line();
    if retained_request_line.method() != policy.request_line.method
        || retained_request_line.target() != policy.request_line.target
    {
        return Err(HttpsAuditedRequestHeadError::RequestLinePolicyMismatch);
    }

    let header_fields = fixed_request_body.header_fields();
    let retained_destination = header_fields.host_binding().destination().host_authority();
    if !retained_destination
        .host()
        .eq_ignore_ascii_case(policy.destination.host())
        || retained_destination.port() != policy.destination.port()
    {
        return Err(HttpsAuditedRequestHeadError::DestinationPolicyMismatch);
    }

    let additional_fields = header_fields.additional_fields();
    if additional_fields.len() != policy.additional_fields.len()
        || additional_fields
            .iter()
            .zip(policy.additional_fields.iter())
            .any(|(field, expected)| {
                !field.name().eq_ignore_ascii_case(expected.name())
                    || field.value() != expected.value()
            })
    {
        return Err(HttpsAuditedRequestHeadError::AdditionalFieldsPolicyMismatch);
    }

    let mut encoded_head_length = retained_request_line
        .encoded_length()
        .checked_add(header_fields.host_binding().host_field().encoded_length())
        .and_then(|length| {
            length.checked_add(fixed_request_body.fixed_body().field().encoded_length())
        })
        .ok_or(HttpsAuditedRequestHeadError::HeadTooLong)?;
    for field in additional_fields {
        encoded_head_length = encoded_head_length
            .checked_add(field.encoded_length())
            .ok_or(HttpsAuditedRequestHeadError::HeadTooLong)?;
    }
    encoded_head_length = encoded_head_length
        .checked_add(2)
        .ok_or(HttpsAuditedRequestHeadError::HeadTooLong)?;
    if encoded_head_length > limits.maximum_head_bytes() {
        return Err(HttpsAuditedRequestHeadError::HeadTooLong);
    }
    if output_length < encoded_head_length {
        return Err(HttpsAuditedRequestHeadError::OutputTooSmall);
    }
    Ok(encoded_head_length)
}

fn encode_preflighted_request_line(
    request_line: &Http1RequestLine<'_>,
    output: &mut [u8],
) -> usize {
    let encoded_length = request_line.encoded_length();
    let result = encode_http1_request_line(request_line, &mut output[..encoded_length]);
    debug_assert!(result.is_ok());
    encoded_length
}

fn encode_preflighted_header_field(
    field: &Http1RequestHeaderField<'_>,
    output: &mut [u8],
) -> usize {
    let encoded_length = field.encoded_length();
    let result = encode_http1_request_header_field(field, &mut output[..encoded_length]);
    debug_assert!(result.is_ok());
    encoded_length
}

fn is_public_additional_field_name(name: &[u8]) -> bool {
    name.eq_ignore_ascii_case(b"Accept") || name.eq_ignore_ascii_case(b"Content-Type")
}

fn matches_canonical_host_authority(
    authority: &Http1RequestHostAuthority<'_>,
    value: &[u8],
) -> bool {
    if value.len() != authority.encoded_length() || value.len() < authority.host().len() {
        return false;
    }

    let (encoded_host, encoded_port) = value.split_at(authority.host().len());
    if !encoded_host
        .iter()
        .copied()
        .zip(authority.host().iter().copied())
        .all(|(encoded, source)| encoded == source.to_ascii_lowercase())
    {
        return false;
    }

    match authority.port() {
        None => encoded_port.is_empty(),
        Some(port) => matches_canonical_port(encoded_port, port),
    }
}

fn matches_canonical_port(encoded: &[u8], port: u16) -> bool {
    let Some(digits) = encoded.strip_prefix(b":") else {
        return false;
    };
    if digits.len() != decimal_length(port) {
        return false;
    }

    let mut remaining = port;
    for digit in digits.iter().rev().copied() {
        if digit != b'0' + (remaining % 10) as u8 {
            return false;
        }
        remaining /= 10;
    }
    remaining == 0
}

fn decimal_length(mut value: u16) -> usize {
    let mut length = 1usize;
    while value >= 10 {
        value /= 10;
        length += 1;
    }
    length
}
