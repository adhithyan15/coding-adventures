//! Exact DNS destination binding across certificate SAN, TLS SNI, and HTTP Host.
//!
//! Each input reaches this crate only after its owning package has validated its
//! syntax. This crate adds the missing semantic equality and retains the exact
//! inputs in one non-copyable value so later composition cannot substitute a
//! different certificate, SNI, or Host authority. It can then bind that
//! complete destination to one strict canonical HTTP/1 Host field without
//! accepting a detached authority value.

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
