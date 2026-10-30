//! # Effect-free OCSP AIA locator policy
//!
//! This crate validates and uniquely selects one borrowed `id-ad-ocsp`
//! locator. It grants no DNS, network, HTTP, TLS, credential, trust, path, or
//! revocation authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::{Asn1Decoder, Asn1Limits};
use std::net::IpAddr;
use x509_authority_information_access::{
    decode_authority_information_access, AuthorityInformationAccess,
    AuthorityInformationAccessErrorKind, AUTHORITY_INFORMATION_ACCESS_OID, ID_AD_OCSP_OID,
    MAX_ACCESS_DESCRIPTIONS,
};
use x509_certificate_policy::ValidatedCertificateRolePolicy;

/// Maximum caller-supplied scheme rules.
pub const MAX_ALLOWED_SCHEMES: usize = 8;
/// Maximum explicit ports in one scheme rule.
pub const MAX_EXPLICIT_PORTS_PER_SCHEME: usize = 16;
/// Hard ceiling for one URI accepted by this boundary.
pub const MAX_OCSP_AIA_URI_BYTES: usize = 4096;
/// DNS presentation-form ceiling used by this boundary.
pub const MAX_OCSP_AIA_HOST_BYTES: usize = 253;
/// Hard ceiling for the retained path and query portion.
pub const MAX_OCSP_AIA_PATH_AND_QUERY_BYTES: usize = 2048;

/// Stable, payload-free malformed-policy categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspAiaLocatorPolicyErrorKind {
    NoAllowedSchemes,
    TooManyAllowedSchemes,
    InvalidScheme,
    DuplicateScheme,
    InvalidDefaultPort,
    TooManyExplicitPorts,
    InvalidExplicitPort,
    DuplicateExplicitPort,
    InvalidUriLimit,
    InvalidHostLimit,
    InvalidPathAndQueryLimit,
}

/// Stable, payload-free locator failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspAiaLocatorErrorKind {
    InvalidPolicy(X509OcspAiaLocatorPolicyErrorKind),
    NoOcspLocator,
    AmbiguousOcspLocator,
    UriTooLong,
    MissingScheme,
    DisallowedScheme,
    MissingAuthority,
    UserinfoNotAllowed,
    InvalidHost,
    IpLiteralHost,
    HostTooLong,
    InvalidPort,
    DisallowedPort,
    FragmentNotAllowed,
    InvalidPathAndQuery,
    PathAndQueryTooLong,
}

/// One redacted OCSP AIA locator failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspAiaLocatorError {
    kind: X509OcspAiaLocatorErrorKind,
}

impl X509OcspAiaLocatorError {
    const fn new(kind: X509OcspAiaLocatorErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free category.
    pub const fn kind(self) -> X509OcspAiaLocatorErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspAiaLocatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "X.509 OCSP AIA locator error {:?}", self.kind)
    }
}

impl std::error::Error for X509OcspAiaLocatorError {}

/// Stable certificate-bound locator failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspCertificateAiaLocatorErrorKind {
    MissingExtensions,
    MissingAuthorityInformationAccess,
    AuthorityInformationAccess(AuthorityInformationAccessErrorKind),
    Locator(X509OcspAiaLocatorErrorKind),
}

/// One redacted failure while deriving an OCSP locator from a certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspCertificateAiaLocatorError {
    kind: X509OcspCertificateAiaLocatorErrorKind,
}

impl X509OcspCertificateAiaLocatorError {
    const fn new(kind: X509OcspCertificateAiaLocatorErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspCertificateAiaLocatorErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspCertificateAiaLocatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP certificate AIA locator error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspCertificateAiaLocatorError {}

/// One caller-supplied scheme and exact port rule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspAiaSchemePolicy<'policy> {
    scheme: &'policy str,
    default_port: u16,
    explicit_ports: &'policy [u16],
}

impl<'policy> X509OcspAiaSchemePolicy<'policy> {
    /// Declare one lowercase URI scheme, its implicit port, and exact allowed
    /// explicit ports. Construction is inert; complete validation occurs when
    /// the containing policy is applied.
    pub const fn new(
        scheme: &'policy str,
        default_port: u16,
        explicit_ports: &'policy [u16],
    ) -> Self {
        Self {
            scheme,
            default_port,
            explicit_ports,
        }
    }

    pub const fn scheme(self) -> &'policy str {
        self.scheme
    }

    pub const fn default_port(self) -> u16 {
        self.default_port
    }

    pub const fn explicit_ports(self) -> &'policy [u16] {
        self.explicit_ports
    }
}

/// Caller-owned, data-driven limits and scheme/port rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspAiaLocatorPolicy<'policy> {
    allowed_schemes: &'policy [X509OcspAiaSchemePolicy<'policy>],
    maximum_uri_bytes: usize,
    maximum_host_bytes: usize,
    maximum_path_and_query_bytes: usize,
}

impl<'policy> X509OcspAiaLocatorPolicy<'policy> {
    /// Construct inert policy data. It is validated in full before any AIA
    /// description is inspected.
    pub const fn new(
        allowed_schemes: &'policy [X509OcspAiaSchemePolicy<'policy>],
        maximum_uri_bytes: usize,
        maximum_host_bytes: usize,
        maximum_path_and_query_bytes: usize,
    ) -> Self {
        Self {
            allowed_schemes,
            maximum_uri_bytes,
            maximum_host_bytes,
            maximum_path_and_query_bytes,
        }
    }

    pub const fn allowed_schemes(self) -> &'policy [X509OcspAiaSchemePolicy<'policy>] {
        self.allowed_schemes
    }

    pub const fn maximum_uri_bytes(self) -> usize {
        self.maximum_uri_bytes
    }

    pub const fn maximum_host_bytes(self) -> usize {
        self.maximum_host_bytes
    }

    pub const fn maximum_path_and_query_bytes(self) -> usize {
        self.maximum_path_and_query_bytes
    }
}

/// Non-copyable evidence selecting one exact AIA description under policy.
pub struct SelectedX509OcspAiaLocator<'aia, 'encoded, 'policy> {
    authority_information_access: &'aia AuthorityInformationAccess<'encoded>,
    description_index: usize,
    scheme_policy: &'policy X509OcspAiaSchemePolicy<'policy>,
    host_start: usize,
    host_end: usize,
    explicit_port: Option<u16>,
    path_start: usize,
    query_start: Option<usize>,
}

impl<'aia, 'encoded, 'policy> SelectedX509OcspAiaLocator<'aia, 'encoded, 'policy> {
    pub const fn authority_information_access(&self) -> &'aia AuthorityInformationAccess<'encoded> {
        self.authority_information_access
    }

    pub const fn description_index(&self) -> usize {
        self.description_index
    }

    pub const fn scheme_policy(&self) -> &'policy X509OcspAiaSchemePolicy<'policy> {
        self.scheme_policy
    }

    pub fn uri(&self) -> &'encoded str {
        self.description().uri()
    }

    pub fn host(&self) -> &'encoded str {
        &self.uri()[self.host_start..self.host_end]
    }

    pub const fn explicit_port(&self) -> Option<u16> {
        self.explicit_port
    }

    pub const fn effective_port(&self) -> u16 {
        match self.explicit_port {
            Some(port) => port,
            None => self.scheme_policy.default_port,
        }
    }

    pub fn path(&self) -> &'encoded str {
        let uri = self.uri();
        let end = self.query_start.unwrap_or(uri.len());
        &uri[self.path_start..end]
    }

    pub fn query(&self) -> Option<&'encoded str> {
        self.query_start.map(|start| &self.uri()[start + 1..])
    }

    pub fn path_and_query(&self) -> &'encoded str {
        &self.uri()[self.path_start..]
    }

    fn description(&self) -> x509_authority_information_access::AccessDescription<'encoded> {
        self.authority_information_access
            .get(self.description_index)
            .expect("selected description index remains bound to the retained AIA value")
    }
}

impl fmt::Debug for SelectedX509OcspAiaLocator<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SelectedX509OcspAiaLocator")
    }
}

/// Non-copyable OCSP locator evidence bound to one exact certificate.
///
/// Every retained string slice originates in the encoded bytes of the exact
/// certificate retained by `role_policy`. This is discovery evidence only; it
/// grants no DNS, destination, HTTP, TLS, trust, or revocation authority.
pub struct SelectedX509CertificateOcspAiaLocator<'certificate, 'encoded, 'policy> {
    role_policy: ValidatedCertificateRolePolicy<'certificate, 'encoded>,
    description_index: usize,
    scheme_policy: &'policy X509OcspAiaSchemePolicy<'policy>,
    uri: &'encoded str,
    host: &'encoded str,
    explicit_port: Option<u16>,
    path: &'encoded str,
    query: Option<&'encoded str>,
    path_and_query: &'encoded str,
}

impl<'certificate, 'encoded, 'policy>
    SelectedX509CertificateOcspAiaLocator<'certificate, 'encoded, 'policy>
{
    /// Return the consumed critical-extension and role-policy evidence.
    pub const fn role_policy(&self) -> &ValidatedCertificateRolePolicy<'certificate, 'encoded> {
        &self.role_policy
    }

    /// Return the exact AIA description index selected from the certificate.
    pub const fn description_index(&self) -> usize {
        self.description_index
    }

    /// Return the exact caller-owned scheme rule that accepted the locator.
    pub const fn scheme_policy(&self) -> &'policy X509OcspAiaSchemePolicy<'policy> {
        self.scheme_policy
    }

    pub const fn uri(&self) -> &'encoded str {
        self.uri
    }

    pub const fn host(&self) -> &'encoded str {
        self.host
    }

    pub const fn explicit_port(&self) -> Option<u16> {
        self.explicit_port
    }

    pub const fn effective_port(&self) -> u16 {
        match self.explicit_port {
            Some(port) => port,
            None => self.scheme_policy.default_port,
        }
    }

    pub const fn path(&self) -> &'encoded str {
        self.path
    }

    pub const fn query(&self) -> Option<&'encoded str> {
        self.query
    }

    pub const fn path_and_query(&self) -> &'encoded str {
        self.path_and_query
    }
}

impl fmt::Debug for SelectedX509CertificateOcspAiaLocator<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SelectedX509CertificateOcspAiaLocator")
    }
}

struct ParsedLocator {
    scheme_index: usize,
    host_start: usize,
    host_end: usize,
    explicit_port: Option<u16>,
    path_start: usize,
    query_start: Option<usize>,
}

/// Validate every exact `id-ad-ocsp` locator and select exactly one.
///
/// The complete policy is checked before the AIA value is inspected. Exact
/// OCSP descriptions are processed in wire order; an invalid or disallowed
/// locator fails the whole selection rather than being skipped. Other access
/// methods are ignored. Success grants only borrowed selection evidence.
pub fn select_unique_x509_ocsp_aia_locator<'aia, 'encoded, 'policy>(
    authority_information_access: &'aia AuthorityInformationAccess<'encoded>,
    policy: &'policy X509OcspAiaLocatorPolicy<'policy>,
) -> Result<SelectedX509OcspAiaLocator<'aia, 'encoded, 'policy>, X509OcspAiaLocatorError> {
    validate_policy(policy).map_err(|kind| {
        X509OcspAiaLocatorError::new(X509OcspAiaLocatorErrorKind::InvalidPolicy(kind))
    })?;

    let mut selected = None;
    let mut count = 0usize;
    for index in 0..authority_information_access
        .len()
        .min(MAX_ACCESS_DESCRIPTIONS)
    {
        let description = authority_information_access
            .get(index)
            .expect("indices below the decoded AIA length are populated");
        if !description.access_method().equals(ID_AD_OCSP_OID) {
            continue;
        }
        let parsed = parse_locator(description.uri(), policy)?;
        count += 1;
        if selected.is_none() {
            selected = Some((index, parsed));
        }
    }

    if count == 0 {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::NoOcspLocator,
        ));
    }
    if count != 1 {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::AmbiguousOcspLocator,
        ));
    }

    let (description_index, parsed) = selected.expect("one parsed locator was counted");
    Ok(SelectedX509OcspAiaLocator {
        authority_information_access,
        description_index,
        scheme_policy: &policy.allowed_schemes[parsed.scheme_index],
        host_start: parsed.host_start,
        host_end: parsed.host_end,
        explicit_port: parsed.explicit_port,
        path_start: parsed.path_start,
        query_start: parsed.query_start,
    })
}

/// Derive and select one OCSP AIA locator from one exact certificate.
///
/// Caller policy is validated before certificate data is inspected. The AIA
/// extension is then derived only through the consumed certificate-wide role
/// policy, structurally decoded under `asn1_limits`, and passed through the
/// same fail-closed unique-locator policy as the lower-level entry point.
pub fn select_unique_x509_certificate_ocsp_aia_locator<'certificate, 'encoded, 'policy>(
    role_policy: ValidatedCertificateRolePolicy<'certificate, 'encoded>,
    asn1_limits: Asn1Limits,
    policy: &'policy X509OcspAiaLocatorPolicy<'policy>,
) -> Result<
    SelectedX509CertificateOcspAiaLocator<'certificate, 'encoded, 'policy>,
    X509OcspCertificateAiaLocatorError,
> {
    validate_policy(policy).map_err(|kind| {
        X509OcspCertificateAiaLocatorError::new(X509OcspCertificateAiaLocatorErrorKind::Locator(
            X509OcspAiaLocatorErrorKind::InvalidPolicy(kind),
        ))
    })?;

    let extensions = role_policy
        .certificate()
        .tbs_certificate()
        .extensions()
        .ok_or_else(|| {
            X509OcspCertificateAiaLocatorError::new(
                X509OcspCertificateAiaLocatorErrorKind::MissingExtensions,
            )
        })?;
    let extension = extensions
        .iter()
        .find(|extension| {
            extension
                .extension_id()
                .equals(AUTHORITY_INFORMATION_ACCESS_OID)
        })
        .ok_or_else(|| {
            X509OcspCertificateAiaLocatorError::new(
                X509OcspCertificateAiaLocatorErrorKind::MissingAuthorityInformationAccess,
            )
        })?;

    let mut decoder = Asn1Decoder::new(asn1_limits);
    let authority_information_access = decode_authority_information_access(&mut decoder, extension)
        .map_err(|error| {
            X509OcspCertificateAiaLocatorError::new(
                X509OcspCertificateAiaLocatorErrorKind::AuthorityInformationAccess(error.kind()),
            )
        })?;
    let selected = select_unique_x509_ocsp_aia_locator(&authority_information_access, policy)
        .map_err(|error| {
            X509OcspCertificateAiaLocatorError::new(
                X509OcspCertificateAiaLocatorErrorKind::Locator(error.kind()),
            )
        })?;

    Ok(SelectedX509CertificateOcspAiaLocator {
        role_policy,
        description_index: selected.description_index(),
        scheme_policy: selected.scheme_policy(),
        uri: selected.uri(),
        host: selected.host(),
        explicit_port: selected.explicit_port(),
        path: selected.path(),
        query: selected.query(),
        path_and_query: selected.path_and_query(),
    })
}

fn validate_policy(
    policy: &X509OcspAiaLocatorPolicy<'_>,
) -> Result<(), X509OcspAiaLocatorPolicyErrorKind> {
    if policy.allowed_schemes.is_empty() {
        return Err(X509OcspAiaLocatorPolicyErrorKind::NoAllowedSchemes);
    }
    if policy.allowed_schemes.len() > MAX_ALLOWED_SCHEMES {
        return Err(X509OcspAiaLocatorPolicyErrorKind::TooManyAllowedSchemes);
    }
    if !(1..=MAX_OCSP_AIA_URI_BYTES).contains(&policy.maximum_uri_bytes) {
        return Err(X509OcspAiaLocatorPolicyErrorKind::InvalidUriLimit);
    }
    if !(1..=MAX_OCSP_AIA_HOST_BYTES).contains(&policy.maximum_host_bytes) {
        return Err(X509OcspAiaLocatorPolicyErrorKind::InvalidHostLimit);
    }
    if policy.maximum_path_and_query_bytes > MAX_OCSP_AIA_PATH_AND_QUERY_BYTES {
        return Err(X509OcspAiaLocatorPolicyErrorKind::InvalidPathAndQueryLimit);
    }

    for (index, scheme_policy) in policy.allowed_schemes.iter().enumerate() {
        if !valid_policy_scheme(scheme_policy.scheme) {
            return Err(X509OcspAiaLocatorPolicyErrorKind::InvalidScheme);
        }
        if policy.allowed_schemes[..index]
            .iter()
            .any(|earlier| earlier.scheme == scheme_policy.scheme)
        {
            return Err(X509OcspAiaLocatorPolicyErrorKind::DuplicateScheme);
        }
        if scheme_policy.default_port == 0 {
            return Err(X509OcspAiaLocatorPolicyErrorKind::InvalidDefaultPort);
        }
        if scheme_policy.explicit_ports.len() > MAX_EXPLICIT_PORTS_PER_SCHEME {
            return Err(X509OcspAiaLocatorPolicyErrorKind::TooManyExplicitPorts);
        }
        for (port_index, port) in scheme_policy.explicit_ports.iter().copied().enumerate() {
            if port == 0 {
                return Err(X509OcspAiaLocatorPolicyErrorKind::InvalidExplicitPort);
            }
            if scheme_policy.explicit_ports[..port_index].contains(&port) {
                return Err(X509OcspAiaLocatorPolicyErrorKind::DuplicateExplicitPort);
            }
        }
    }
    Ok(())
}

fn valid_policy_scheme(scheme: &str) -> bool {
    let mut bytes = scheme.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.')
        })
}

fn parse_locator(
    uri: &str,
    policy: &X509OcspAiaLocatorPolicy<'_>,
) -> Result<ParsedLocator, X509OcspAiaLocatorError> {
    if uri.len() > policy.maximum_uri_bytes {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::UriTooLong,
        ));
    }
    if uri.as_bytes().contains(&b'#') {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::FragmentNotAllowed,
        ));
    }

    let scheme_end = uri
        .find(':')
        .ok_or_else(|| X509OcspAiaLocatorError::new(X509OcspAiaLocatorErrorKind::MissingScheme))?;
    let scheme = &uri[..scheme_end];
    if !valid_uri_scheme(scheme) {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::MissingScheme,
        ));
    }
    let scheme_index = policy
        .allowed_schemes
        .iter()
        .position(|allowed| scheme.eq_ignore_ascii_case(allowed.scheme))
        .ok_or_else(|| {
            X509OcspAiaLocatorError::new(X509OcspAiaLocatorErrorKind::DisallowedScheme)
        })?;

    let authority_start = scheme_end + 3;
    if uri.as_bytes().get(scheme_end + 1..authority_start) != Some(b"//") {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::MissingAuthority,
        ));
    }
    let authority_end = uri[authority_start..]
        .find(['/', '?'])
        .map_or(uri.len(), |relative| authority_start + relative);
    let authority = &uri[authority_start..authority_end];
    if authority.is_empty() {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::MissingAuthority,
        ));
    }
    if authority.as_bytes().contains(&b'@') {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::UserinfoNotAllowed,
        ));
    }

    let (host, explicit_port) = parse_authority(authority)?;
    if host.len() > policy.maximum_host_bytes {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::HostTooLong,
        ));
    }
    validate_dns_host(host)?;
    let scheme_policy = &policy.allowed_schemes[scheme_index];
    if explicit_port.is_some_and(|port| !scheme_policy.explicit_ports.contains(&port)) {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::DisallowedPort,
        ));
    }

    let path_and_query = &uri[authority_end..];
    if path_and_query.len() > policy.maximum_path_and_query_bytes {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::PathAndQueryTooLong,
        ));
    }
    let query_start = path_and_query
        .find('?')
        .map(|relative| authority_end + relative);
    let path_end = query_start.unwrap_or(uri.len());
    if !valid_component(&uri[authority_end..path_end], false)
        || query_start.is_some_and(|start| !valid_component(&uri[start + 1..], true))
    {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::InvalidPathAndQuery,
        ));
    }

    Ok(ParsedLocator {
        scheme_index,
        host_start: authority_start,
        host_end: authority_start + host.len(),
        explicit_port,
        path_start: authority_end,
        query_start,
    })
}

fn valid_uri_scheme(scheme: &str) -> bool {
    let mut bytes = scheme.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

fn parse_authority(authority: &str) -> Result<(&str, Option<u16>), X509OcspAiaLocatorError> {
    let Some((host, port_text)) = authority.rsplit_once(':') else {
        return Ok((authority, None));
    };
    if host.as_bytes().contains(&b':') || host.is_empty() || port_text.is_empty() {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::InvalidPort,
        ));
    }
    if !port_text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::InvalidPort,
        ));
    }
    let port = port_text
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| X509OcspAiaLocatorError::new(X509OcspAiaLocatorErrorKind::InvalidPort))?;
    Ok((host, Some(port)))
}

fn validate_dns_host(host: &str) -> Result<(), X509OcspAiaLocatorError> {
    // `IpAddr::from_str` deliberately accepts only canonical dotted IPv4 and
    // IPv6. System resolvers commonly accept the older inet_aton spellings as
    // well: shortened dotted forms (`127.1`), one integer (`2130706433`),
    // octal components, and `0x`-prefixed hexadecimal components. Reject the
    // whole numeric grammar here so later DNS composition cannot reinterpret
    // selected "DNS" evidence as an address literal.
    if host.parse::<IpAddr>().is_ok() || looks_like_legacy_ipv4_number(host) {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::IpLiteralHost,
        ));
    }
    if host.is_empty() || host.len() > MAX_OCSP_AIA_HOST_BYTES {
        return Err(X509OcspAiaLocatorError::new(
            X509OcspAiaLocatorErrorKind::InvalidHost,
        ));
    }
    for label in host.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(X509OcspAiaLocatorError::new(
                X509OcspAiaLocatorErrorKind::InvalidHost,
            ));
        }
    }
    Ok(())
}

fn looks_like_legacy_ipv4_number(host: &str) -> bool {
    let mut component_count = 0usize;
    for component in host.split('.') {
        component_count += 1;
        if component_count > 4 || component.is_empty() || !is_ipv4_number_component(component) {
            return false;
        }
    }
    component_count != 0
}

fn is_ipv4_number_component(component: &str) -> bool {
    if let Some(hexadecimal) = component
        .strip_prefix("0x")
        .or_else(|| component.strip_prefix("0X"))
    {
        !hexadecimal.is_empty() && hexadecimal.bytes().all(|byte| byte.is_ascii_hexdigit())
    } else {
        component.bytes().all(|byte| byte.is_ascii_digit())
    }
}

fn valid_component(component: &str, allow_question_mark: bool) -> bool {
    let bytes = component.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'%' {
            if !bytes
                .get(index + 1..index + 3)
                .is_some_and(|pair| pair.iter().all(u8::is_ascii_hexdigit))
            {
                return false;
            }
            index += 3;
            continue;
        }
        let allowed = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'.'
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
                    | b'/'
            )
            || (allow_question_mark && byte == b'?');
        if !allowed {
            return false;
        }
        index += 1;
    }
    true
}
