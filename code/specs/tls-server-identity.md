# TLS Server Identity

## Status

Specification for the first repository-owned certificate-verification
primitive required before `tls-platform` can replace `rustls` and
`webpki-roots`. This slice verifies a caller's already-selected DNS name or IP
address against already-decoded X.509 `subjectAltName` identifiers. It does not
parse certificates, build certificate paths, choose trust roots, perform a TLS
handshake, or open a socket.

## Why This Boundary Exists

TLS authenticates an endpoint only when two independent questions both have
closed answers:

1. Does the certificate chain terminate at an authorized trust root?
2. Does the leaf certificate identify the server the caller intended to
   contact?

This package owns only the second question. Keeping it independent makes the
matching rules testable without certificate-parser, transport, clock, or
cryptographic authority. A later repository-owned X.509 parser will provide
typed `subjectAltName` values, and a later path validator will establish that
the leaf certificate is trusted. Neither authority is implied by a successful
identity match.

## Standards Contract

The rules follow RFC 9525 for direct, unrestricted HTTPS identities:

- DNS references match only `subjectAltName` `dNSName` values (`DNS-ID`).
- IP references match only `subjectAltName` `iPAddress` values (`IP-ID`).
- Subject Common Name and every other subject attribute are outside the API,
  so no CN fallback can be introduced accidentally.
- ASCII DNS labels compare case-insensitively.
- A presented wildcard is accepted only when `*` is the complete left-most
  label and is the only wildcard character. It matches exactly one reference
  label.
- A reference identity can never contain a wildcard.
- IP matching is exact on the parsed address octets. A textual IP address in a
  `dNSName` never authenticates an IP reference.
- SRV-ID and URI-ID are not supported by this HTTPS-oriented slice.

Unicode U-label conversion is deliberately absent. Inputs must already be
ASCII. Syntactically valid `xn--` A-labels remain ASCII DNS labels and compare
as such; this package does not claim to implement IDNA mapping.

## Bounded Inputs

- A DNS name is at most 253 bytes excluding an optional presentation-only
  trailing root dot.
- A label is 1 through 63 bytes.
- At most 256 total presented DNS and IP identifiers may be examined.
- Empty labels, repeated dots, control bytes, non-ASCII bytes, leading or
  trailing hyphens, URI syntax, ports, brackets, NULs, and wildcards outside
  the exact presented form above are rejected or ignored as described below.

The reference identity is validated before any presented identifier is
examined. An invalid reference is a closed caller error. Invalid presented DNS
identifiers are ignored, matching RFC 9525's rule for invalid wildcard
identifiers, so another valid SAN can still authenticate the endpoint. An
over-limit SAN collection fails closed before matching.

## Public Contract

```rust
pub enum ReferenceIdentity {
    DnsName(DnsName),
    IpAddress(std::net::IpAddr),
}

impl ReferenceIdentity {
    pub fn parse(input: &str) -> Result<Self, IdentityError>;
}

pub struct PresentedIdentities<'a> {
    pub dns_names: &'a [&'a str],
    pub ip_addresses: &'a [std::net::IpAddr],
}

pub fn verify_server_identity(
    reference: &ReferenceIdentity,
    presented: PresentedIdentities<'_>,
) -> Result<VerifiedServerIdentity, IdentityError>;
```

`ReferenceIdentity::parse` first attempts the standard library's strict
`IpAddr` parser. This prevents an IPv4-looking reference from being treated as
a DNS name. If IP parsing does not succeed, the input must satisfy the DNS
rules. Bracketed IPv6 and `host:port` strings are not identities and are
rejected; URI parsing belongs above this boundary.

`VerifiedServerIdentity` exposes only the matched identifier kind (`ExactDns`,
`WildcardDns`, or `IpAddress`). It does not bless a certificate chain, return
the matched certificate text, or carry a socket-opening capability.

## Matching Table

| Reference | Presented SAN | Result |
| --- | --- | --- |
| `api.example.test` | DNS `API.EXAMPLE.TEST` | exact DNS match |
| `mail.example.test` | DNS `*.example.test` | wildcard DNS match |
| `deep.mail.example.test` | DNS `*.example.test` | mismatch |
| `example.test` | DNS `*.example.test` | mismatch |
| `192.0.2.10` | IP `192.0.2.10` | IP match |
| `192.0.2.10` | DNS `192.0.2.10` | mismatch |
| `api.example.test` | subject CN only | impossible to express |

## Failure Model

Errors distinguish invalid references, excessive presented identifiers, and a
closed mismatch. They never include certificate bytes or an unbounded list of
presented names. Invalid presented entries are counted only for bounded,
redacted diagnostics.

## Non-Goals

This slice adds no external dependency and does not provide:

- DER or ASN.1 parsing;
- X.509 extension processing or name-constraints enforcement;
- certificate path construction or signature verification;
- revocation checking;
- trust-root loading;
- IDNA U-label conversion;
- TLS record, handshake, cipher, key-exchange, or socket logic;
- integration into `tls-platform` or permission to use it for OAuth HTTPS.

Those later boundaries must compose this matcher only after typed SAN decoding
and must still prove the certificate path independently.
