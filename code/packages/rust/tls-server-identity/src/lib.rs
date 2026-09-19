//! # TLS server identity matching
//!
//! A TLS connection is authenticated only when its certificate path is trusted
//! **and** its leaf certificate names the intended server. Those are separate
//! decisions. This crate deliberately owns only the second one.
//!
//! The inputs are typed values that a future repository-owned X.509 parser can
//! extract from `subjectAltName`: DNS-ID strings and parsed IP-ID octets. There
//! is no Common Name input, so the obsolete and ambiguous CN fallback cannot
//! appear by accident. There is no socket or certificate-path authority either;
//! a successful match is not a claim that a certificate is trusted.

#![deny(unsafe_code)]

use std::error::Error;
use std::fmt;
use std::net::IpAddr;

/// RFC 1035's maximum textual domain length without the root dot.
pub const MAX_DNS_NAME_LEN: usize = 253;
/// RFC 1035's maximum label length.
pub const MAX_DNS_LABEL_LEN: usize = 63;
/// A hard work bound for a hostile certificate's decoded SAN collection.
pub const MAX_PRESENTED_IDENTIFIERS: usize = 256;

/// One validated ASCII DNS reference name.
///
/// The stored form has no trailing root dot. Case is retained for diagnostics,
/// while matching uses ASCII case folding as RFC 9525 requires.
#[derive(Clone, PartialEq, Eq)]
pub struct DnsName(String);

impl DnsName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for DnsName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DnsName")
            .field("length", &self.0.len())
            .field("labels", &self.0.split('.').count())
            .finish()
    }
}

/// The direct, unrestricted identity selected by the application.
#[derive(Clone, PartialEq, Eq)]
pub enum ReferenceIdentity {
    DnsName(DnsName),
    IpAddress(IpAddr),
}

impl ReferenceIdentity {
    /// Parse an application-selected identity, preferring strict IP parsing.
    ///
    /// The order matters: an IPv4 textual address is also syntactically made of
    /// DNS labels, but RFC 9525 requires it to match an IP-ID, not a DNS-ID.
    pub fn parse(input: &str) -> Result<Self, IdentityError> {
        if let Ok(address) = input.parse::<IpAddr>() {
            return Ok(Self::IpAddress(address));
        }

        let normalized = validate_dns_name(input, WildcardPolicy::Reject)
            .map_err(IdentityError::InvalidReference)?;
        Ok(Self::DnsName(DnsName(normalized.to_string())))
    }
}

impl fmt::Debug for ReferenceIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DnsName(name) => formatter
                .debug_tuple("ReferenceIdentity::DnsName")
                .field(name)
                .finish(),
            Self::IpAddress(IpAddr::V4(_)) => {
                formatter.write_str("ReferenceIdentity::IpAddress(V4)")
            }
            Self::IpAddress(IpAddr::V6(_)) => {
                formatter.write_str("ReferenceIdentity::IpAddress(V6)")
            }
        }
    }
}

/// Already-decoded SAN values from one leaf certificate.
#[derive(Clone, Copy, Debug)]
pub struct PresentedIdentities<'a> {
    pub dns_names: &'a [&'a str],
    pub ip_addresses: &'a [IpAddr],
}

impl<'a> PresentedIdentities<'a> {
    pub const fn new(dns_names: &'a [&'a str], ip_addresses: &'a [IpAddr]) -> Self {
        Self {
            dns_names,
            ip_addresses,
        }
    }
}

/// Which typed SAN established the match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchKind {
    ExactDns,
    WildcardDns,
    IpAddress,
}

/// Opaque evidence that identity matching succeeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedServerIdentity {
    kind: MatchKind,
}

impl VerifiedServerIdentity {
    pub const fn kind(self) -> MatchKind {
        self.kind
    }
}

/// A reference-name syntax failure. Raw input is intentionally absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidDnsName {
    Empty,
    TooLong,
    NonAscii,
    EmptyLabel,
    LabelTooLong,
    InvalidCharacter,
    LeadingHyphen,
    TrailingHyphen,
    WildcardNotAllowed,
    InvalidWildcard,
}

/// Closed identity-verification failures with bounded, redacted context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityError {
    InvalidReference(InvalidDnsName),
    TooManyPresentedIdentifiers { maximum: usize, actual: usize },
    Mismatch { invalid_dns_identifiers: usize },
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReference(reason) => {
                write!(formatter, "invalid reference server identity: {reason:?}")
            }
            Self::TooManyPresentedIdentifiers { maximum, actual } => write!(
                formatter,
                "presented identity count {actual} exceeds maximum {maximum}"
            ),
            Self::Mismatch {
                invalid_dns_identifiers,
            } => write!(
                formatter,
                "server identity mismatch ({invalid_dns_identifiers} invalid DNS identifiers ignored)"
            ),
        }
    }
}

impl Error for IdentityError {}

/// Match one application-selected identity against typed SAN values.
pub fn verify_server_identity(
    reference: &ReferenceIdentity,
    presented: PresentedIdentities<'_>,
) -> Result<VerifiedServerIdentity, IdentityError> {
    let total = presented
        .dns_names
        .len()
        .checked_add(presented.ip_addresses.len())
        .ok_or(IdentityError::TooManyPresentedIdentifiers {
            maximum: MAX_PRESENTED_IDENTIFIERS,
            actual: usize::MAX,
        })?;
    if total > MAX_PRESENTED_IDENTIFIERS {
        return Err(IdentityError::TooManyPresentedIdentifiers {
            maximum: MAX_PRESENTED_IDENTIFIERS,
            actual: total,
        });
    }

    match reference {
        ReferenceIdentity::IpAddress(reference_address) => {
            if presented
                .ip_addresses
                .iter()
                .any(|presented_address| presented_address == reference_address)
            {
                return Ok(VerifiedServerIdentity {
                    kind: MatchKind::IpAddress,
                });
            }
            Err(IdentityError::Mismatch {
                invalid_dns_identifiers: 0,
            })
        }
        ReferenceIdentity::DnsName(reference_name) => {
            verify_dns_identity(reference_name, presented.dns_names)
        }
    }
}

fn verify_dns_identity(
    reference: &DnsName,
    presented_names: &[&str],
) -> Result<VerifiedServerIdentity, IdentityError> {
    let mut invalid = 0usize;
    for presented in presented_names {
        let Ok(normalized) = validate_dns_name(presented, WildcardPolicy::AllowPresented) else {
            invalid += 1;
            continue;
        };

        if normalized.starts_with("*.") {
            if wildcard_matches(reference.as_str(), normalized) {
                return Ok(VerifiedServerIdentity {
                    kind: MatchKind::WildcardDns,
                });
            }
        } else if reference.as_str().eq_ignore_ascii_case(normalized) {
            return Ok(VerifiedServerIdentity {
                kind: MatchKind::ExactDns,
            });
        }
    }

    Err(IdentityError::Mismatch {
        invalid_dns_identifiers: invalid,
    })
}

fn wildcard_matches(reference: &str, presented: &str) -> bool {
    let Some((_, suffix)) = presented.split_once('.') else {
        return false;
    };
    let Some((leftmost, reference_suffix)) = reference.split_once('.') else {
        return false;
    };
    !leftmost.is_empty() && reference_suffix.eq_ignore_ascii_case(suffix)
}

#[derive(Clone, Copy)]
enum WildcardPolicy {
    Reject,
    AllowPresented,
}

fn validate_dns_name(input: &str, wildcard_policy: WildcardPolicy) -> Result<&str, InvalidDnsName> {
    if input.is_empty() {
        return Err(InvalidDnsName::Empty);
    }
    if !input.is_ascii() {
        return Err(InvalidDnsName::NonAscii);
    }

    let normalized = input.strip_suffix('.').unwrap_or(input);
    if normalized.is_empty() {
        return Err(InvalidDnsName::Empty);
    }
    if normalized.len() > MAX_DNS_NAME_LEN {
        return Err(InvalidDnsName::TooLong);
    }

    let mut wildcard_count = 0usize;
    for (index, label) in normalized.split('.').enumerate() {
        if label.is_empty() {
            return Err(InvalidDnsName::EmptyLabel);
        }
        if label.len() > MAX_DNS_LABEL_LEN {
            return Err(InvalidDnsName::LabelTooLong);
        }
        if label == "*" {
            wildcard_count += 1;
            match wildcard_policy {
                WildcardPolicy::Reject => return Err(InvalidDnsName::WildcardNotAllowed),
                WildcardPolicy::AllowPresented if index == 0 => continue,
                WildcardPolicy::AllowPresented => return Err(InvalidDnsName::InvalidWildcard),
            }
        }
        if label.contains('*') {
            return Err(match wildcard_policy {
                WildcardPolicy::Reject => InvalidDnsName::WildcardNotAllowed,
                WildcardPolicy::AllowPresented => InvalidDnsName::InvalidWildcard,
            });
        }
        if label.starts_with('-') {
            return Err(InvalidDnsName::LeadingHyphen);
        }
        if label.ends_with('-') {
            return Err(InvalidDnsName::TrailingHyphen);
        }
        if !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(InvalidDnsName::InvalidCharacter);
        }
    }

    if wildcard_count > 1 {
        return Err(InvalidDnsName::InvalidWildcard);
    }
    if wildcard_count == 1 && !normalized.contains('.') {
        return Err(InvalidDnsName::InvalidWildcard);
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn verify_dns(reference: &str, names: &[&str]) -> Result<MatchKind, IdentityError> {
        let reference = ReferenceIdentity::parse(reference)?;
        verify_server_identity(&reference, PresentedIdentities::new(names, &[]))
            .map(VerifiedServerIdentity::kind)
    }

    #[test]
    fn exact_dns_matching_is_ascii_case_insensitive() {
        assert_eq!(
            verify_dns("api.example.test", &["API.Example.TEST"]),
            Ok(MatchKind::ExactDns)
        );
    }

    #[test]
    fn one_trailing_root_dot_is_presentation_only() {
        assert_eq!(
            verify_dns("api.example.test.", &["API.EXAMPLE.TEST."]),
            Ok(MatchKind::ExactDns)
        );
    }

    #[test]
    fn wildcard_matches_exactly_one_leftmost_label() {
        assert_eq!(
            verify_dns("mail.example.test", &["*.example.test"]),
            Ok(MatchKind::WildcardDns)
        );
        assert!(matches!(
            verify_dns("deep.mail.example.test", &["*.example.test"]),
            Err(IdentityError::Mismatch { .. })
        ));
        assert!(matches!(
            verify_dns("example.test", &["*.example.test"]),
            Err(IdentityError::Mismatch { .. })
        ));
    }

    #[test]
    fn malformed_wildcards_are_ignored_while_a_valid_san_can_match() {
        let names = [
            "api.*.test",
            "a*pi.example.test",
            "*.*.example.test",
            "api.example.test",
        ];
        assert_eq!(
            verify_dns("api.example.test", &names),
            Ok(MatchKind::ExactDns)
        );
    }

    #[test]
    fn invalid_presented_names_are_counted_without_echoing_them() {
        let error = verify_dns("api.example.test", &["bad_name", "two..dots"]).unwrap_err();
        assert_eq!(
            error,
            IdentityError::Mismatch {
                invalid_dns_identifiers: 2
            }
        );
        let display = error.to_string();
        assert!(!display.contains("bad_name"));
        assert!(!display.contains("two..dots"));
    }

    #[test]
    fn reference_wildcards_are_never_allowed() {
        assert_eq!(
            ReferenceIdentity::parse("*.example.test"),
            Err(IdentityError::InvalidReference(
                InvalidDnsName::WildcardNotAllowed
            ))
        );
    }

    #[test]
    fn invalid_reference_syntax_fails_closed() {
        let cases = [
            ("", InvalidDnsName::Empty),
            (".", InvalidDnsName::Empty),
            ("two..dots", InvalidDnsName::EmptyLabel),
            ("-api.example", InvalidDnsName::LeadingHyphen),
            ("api-.example", InvalidDnsName::TrailingHyphen),
            ("api_example", InvalidDnsName::InvalidCharacter),
            ("https://example.test", InvalidDnsName::InvalidCharacter),
            ("example.test:443", InvalidDnsName::InvalidCharacter),
            ("[2001:db8::1]", InvalidDnsName::InvalidCharacter),
            ("exämple.test", InvalidDnsName::NonAscii),
        ];
        for (input, reason) in cases {
            assert_eq!(
                ReferenceIdentity::parse(input),
                Err(IdentityError::InvalidReference(reason)),
                "input {input:?}"
            );
        }
    }

    #[test]
    fn dns_length_limits_are_enforced() {
        let long_label = format!("{}.test", "a".repeat(MAX_DNS_LABEL_LEN + 1));
        assert_eq!(
            ReferenceIdentity::parse(&long_label),
            Err(IdentityError::InvalidReference(
                InvalidDnsName::LabelTooLong
            ))
        );

        let long_name = ["a"; 128].join(".");
        assert!(long_name.len() > MAX_DNS_NAME_LEN);
        assert_eq!(
            ReferenceIdentity::parse(&long_name),
            Err(IdentityError::InvalidReference(InvalidDnsName::TooLong))
        );
    }

    #[test]
    fn ascii_a_labels_are_compared_without_claiming_idna_conversion() {
        assert_eq!(
            verify_dns("xn--bcher-kva.example", &["XN--BCHER-KVA.EXAMPLE"]),
            Ok(MatchKind::ExactDns)
        );
    }

    #[test]
    fn ipv4_matches_only_typed_ip_san() {
        let reference = ReferenceIdentity::parse("192.0.2.10").unwrap();
        let dns = ["192.0.2.10"];
        let wrong_ips = [IpAddr::V4(Ipv4Addr::new(192, 0, 2, 11))];
        assert!(matches!(
            verify_server_identity(&reference, PresentedIdentities::new(&dns, &wrong_ips)),
            Err(IdentityError::Mismatch { .. })
        ));

        let matching_ips = [IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10))];
        assert_eq!(
            verify_server_identity(&reference, PresentedIdentities::new(&[], &matching_ips))
                .unwrap()
                .kind(),
            MatchKind::IpAddress
        );
    }

    #[test]
    fn ipv6_matches_canonical_octets_not_textual_spelling() {
        let reference = ReferenceIdentity::parse("2001:db8::1").unwrap();
        let matching = [IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1))];
        assert_eq!(
            verify_server_identity(&reference, PresentedIdentities::new(&[], &matching))
                .unwrap()
                .kind(),
            MatchKind::IpAddress
        );
    }

    #[test]
    fn dns_reference_never_cross_matches_an_ip_san() {
        let reference = ReferenceIdentity::parse("api.example.test").unwrap();
        let ips = [IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10))];
        assert!(matches!(
            verify_server_identity(&reference, PresentedIdentities::new(&[], &ips)),
            Err(IdentityError::Mismatch { .. })
        ));
    }

    #[test]
    fn collection_limit_fails_before_a_match_is_considered() {
        let reference = ReferenceIdentity::parse("api.example.test").unwrap();
        let names = vec!["api.example.test"; MAX_PRESENTED_IDENTIFIERS + 1];
        assert_eq!(
            verify_server_identity(&reference, PresentedIdentities::new(&names, &[])),
            Err(IdentityError::TooManyPresentedIdentifiers {
                maximum: MAX_PRESENTED_IDENTIFIERS,
                actual: MAX_PRESENTED_IDENTIFIERS + 1,
            })
        );
    }

    #[test]
    fn empty_presented_collection_is_a_closed_mismatch() {
        assert_eq!(
            verify_dns("api.example.test", &[]),
            Err(IdentityError::Mismatch {
                invalid_dns_identifiers: 0
            })
        );
    }

    #[test]
    fn debug_output_redacts_reference_values() {
        let dns = ReferenceIdentity::parse("secret-host.example.test").unwrap();
        let ip = ReferenceIdentity::parse("192.0.2.44").unwrap();
        assert!(!format!("{dns:?}").contains("secret-host"));
        assert!(!format!("{ip:?}").contains("192.0.2.44"));
    }
}
