//! Exact DNS destination binding across certificate SAN, TLS SNI, and HTTP Host.
//!
//! Each input reaches this crate only after its owning package has validated its
//! syntax. This crate adds the missing semantic equality and retains the exact
//! inputs in one non-copyable value so later composition cannot substitute a
//! different certificate, SNI, or Host authority. It can then bind that
//! complete destination to one strict canonical HTTP/1 Host field without
//! accepting a detached authority value, and bind that proof to an exact
//! bounded additional-field slice that contains no second Host field.

#![forbid(unsafe_code)]

use core::fmt;
use der_asn1::Asn1Limits;
use http1_request_header_field::Http1RequestHeaderField;
use http1_request_host_authority::Http1RequestHostAuthority;
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
