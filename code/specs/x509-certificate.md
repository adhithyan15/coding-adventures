# X.509 Certificate Structure

## Status

Implemented as a zero-external-dependency Rust composition above the
repository-owned DER and X.509 field packages.

## Purpose

Decode the complete structural RFC 5280 certificate grammar without enabling
an algorithm or making a trust decision:

```asn1
Certificate ::= SEQUENCE {
  tbsCertificate       TBSCertificate,
  signatureAlgorithm   AlgorithmIdentifier,
  signatureValue       BIT STRING
}

TBSCertificate ::= SEQUENCE {
  version              [0] EXPLICIT Version DEFAULT v1,
  serialNumber             CertificateSerialNumber,
  signature                AlgorithmIdentifier,
  issuer                   Name,
  validity                 Validity,
  subject                  Name,
  subjectPublicKeyInfo     SubjectPublicKeyInfo,
  issuerUniqueID       [1] IMPLICIT UniqueIdentifier OPTIONAL,
  subjectUniqueID      [2] IMPLICIT UniqueIdentifier OPTIONAL,
  extensions           [3] EXPLICIT Extensions OPTIONAL
}
```

The decoder requires canonical DER, the declared field order, a non-empty
issuer Name, unique optional fields, unique identifiers only in v2 or v3,
extensions only in v3, and exact DER agreement between the inner and outer
signature AlgorithmIdentifier.

## Public contract

```rust
pub fn decode_x509_certificate(
    input: &[u8],
    limits: Asn1Limits,
) -> Result<X509Certificate<'_>, X509CertificateError>;
```

`X509Certificate` retains the exact complete encoding, the parsed outer
signature fields, and one `X509TbsCertificate`. The TBS value retains the exact
canonical bytes that a later signature verifier must consume and composes the
existing version, serial-number, algorithm-identifier, name, validity,
subject-public-key-info, and extensions decoders. Optional unique identifiers
are validated canonical borrowed BIT STRINGs.

## Limits and diagnostics

The entire certificate shares one caller-selected DER framing, depth,
total-element, and OID-arc budget. Production decoding allocates nothing from
wire data. Errors contain only a stable category and document-local byte offset
and never retain or format source bytes.

## Tests

Tests cover complete v3 composition, omitted-v1 defaults, signed-byte
retention, a required non-empty issuer, both unique identifiers, extensions,
version constraints, optional field order, inner/outer algorithm mismatch
including parameter mismatch, BIT STRING canonicality, trailing fields,
malformed child framing, shared element limits, and redacted diagnostics.

## Deliberate exclusions

- signature and public-key algorithm registries or parameter policy;
- signature verification or any private-key operation;
- extension recognition, unknown-critical-extension policy, or key-purpose
  authorization;
- DirectoryString normalization or issuer/subject semantic equality;
- path construction, validity-clock decisions, hostname verification,
  revocation, trust-root selection, or platform trust;
- PEM, TLS records or handshakes, sockets, HTTP, or OAuth transport.
