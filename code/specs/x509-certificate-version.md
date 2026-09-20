# X.509 Certificate Version

## Status

Implemented as a zero-external-dependency Rust semantic layer above
`der-asn1`.

## Purpose

Decode the optional RFC 5280 certificate version field without parsing the
rest of `TBSCertificate`:

```asn1
Version ::= INTEGER { v1(0), v2(1), v3(2) }
version [0] EXPLICIT Version DEFAULT v1
```

An omitted field is represented by `X509CertificateVersion::V1`. A present
wrapper must be constructed context-specific tag zero and contain exactly one
canonical INTEGER. Explicit `v1` is rejected because DER requires a value equal
to its ASN.1 default to be omitted. Only explicit `v2` and `v3` are accepted.

## Public contract

```rust
pub enum X509CertificateVersion { V1, V2, V3 }

impl X509CertificateVersion {
    pub fn as_u8(self) -> u8;
}

pub fn decode_explicit_x509_certificate_version(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'_>,
) -> Result<X509CertificateVersion, X509CertificateVersionError>;
```

The surrounding certificate parser owns optional-field detection. It uses the
enum default when tag zero is absent and calls this function when it is present.

## Limits and diagnostics

The explicit child shares the caller's DER framing, depth, and total-element
budgets. Production code allocates nothing. Errors contain only a stable
category and local byte offset and never retain or format source bytes.

## Tests

Tests cover the omitted default, explicit v2 and v3, explicit-default and
unsupported-value rejection, negative and oversized integers, exact wrapper
class/form/tag, malformed child framing, canonical integer enforcement, shared
depth and element budgets, and redacted diagnostics.

## Deliberate exclusions

- optional-field lookahead in a complete `TBSCertificate` parser;
- enforcing version-dependent unique-ID or extension presence rules;
- certificate path construction, signature verification, or algorithms;
- clocks, revocation, hostname verification, trust-root loading, TLS, sockets,
  or OAuth transport.
