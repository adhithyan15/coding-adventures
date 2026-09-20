# X.509 Extended Key Usage

## Status

Implemented as a zero-external-dependency Rust semantic layer above
`x509-extension` and `der-asn1`.

## Purpose

Recognize exactly the RFC 5280 Extended Key Usage extension identifier and
decode its encapsulated DER value:

```asn1
id-ce-extKeyUsage OBJECT IDENTIFIER ::= { id-ce 37 }

ExtKeyUsageSyntax ::= SEQUENCE SIZE (1..MAX) OF KeyPurposeId

KeyPurposeId ::= OBJECT IDENTIFIER
```

This boundary validates one extension value. It does not authorize a purpose
or decide whether a certificate is acceptable for that purpose.

## Input contract

The caller supplies one already-decoded generic `X509Extension` and a mutable
`Asn1Decoder`. The extension identifier must be exactly `2.5.29.37`. Its OCTET
STRING contents must contain exactly one canonical, non-empty DER `SEQUENCE` of
canonical OBJECT IDENTIFIER values.

Purpose identifiers are data-driven: known and unknown identifiers are
preserved in wire order. The decoder retains at most 64 purposes. Duplicates
are preserved because RFC 5280 does not assign this syntax a uniqueness rule;
later certificate policy may reject or normalize redundancy if required.

## Public contract

```rust
pub const EXTENDED_KEY_USAGE_OID: &[u64] = &[2, 5, 29, 37];
pub const MAX_EXTENDED_KEY_PURPOSES: usize = 64;

pub struct ExtendedKeyUsage<'a> { /* borrowed validated purposes */ }

impl<'a> ExtendedKeyUsage<'a> {
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn get(&self, index: usize) -> Option<ObjectIdentifier<'a>>;
    pub fn iter(&self) -> impl Iterator<Item = ObjectIdentifier<'a>>;
    pub fn contains(&self, purpose: &[u64]) -> bool;
}

pub fn decode_extended_key_usage<'a>(
    decoder: &mut Asn1Decoder,
    extension: X509Extension<'a>,
) -> Result<ExtendedKeyUsage<'a>, ExtendedKeyUsageError>;
```

## Limits and diagnostics

The encapsulated value uses the caller's DER framing, depth, total-element,
value-length, and OID-arc limits. Decoding shares the decoder's total-element
budget with prior generic-extension work. The 64-purpose capacity is an
additional fixed work and retention bound.

Errors contain only a stable category and byte offset local to the encapsulated
Extended Key Usage value. They never retain or format certificate or extension
bytes.

## Tests

Tests must cover:

- one and multiple standard and unknown purpose identifiers in wire order;
- exact capacity and over-capacity inputs;
- wrong extension identifiers, empty lists, and non-SEQUENCE payloads;
- invalid tags, empty and non-minimal OIDs, and malformed child framing;
- duplicate preservation and outer criticality independence;
- shared decoder depth, element, OID-arc, and value-length limits;
- payload-local offsets and redacted diagnostics.

## Deliberate exclusions

- recognizing or authorizing server, client, code-signing, email, timestamping,
  OCSP-signing, any-usage, private, or provider-specific purposes;
- enforcing whether the outer extension is critical;
- combining Extended Key Usage with Key Usage or Basic Constraints;
- certificate version, issuer, subject, or name chaining;
- certificate path construction, path-length processing, or name constraints;
- public-key or signature-algorithm parsing and signature verification;
- clocks, revocation, hostname verification, trust-root loading, TLS, sockets,
  or OAuth transport.
