# X.509 Key Usage

## Status

Implemented as a zero-external-dependency Rust semantic layer above
`x509-extension` and `der-asn1`.

## Purpose

Recognize exactly the RFC 5280 Key Usage extension identifier and decode its
encapsulated DER value:

```asn1
id-ce-keyUsage OBJECT IDENTIFIER ::= { id-ce 15 }

KeyUsage ::= BIT STRING {
    digitalSignature  (0),
    nonRepudiation    (1), -- renamed to contentCommitment
    keyEncipherment   (2),
    dataEncipherment  (3),
    keyAgreement      (4),
    keyCertSign       (5),
    cRLSign           (6),
    encipherOnly      (7),
    decipherOnly      (8)
}
```

This boundary validates one extension value. It does not authorize any key
operation or assign a certificate role.

## Input contract

The caller supplies one already-decoded generic `X509Extension` and a mutable
`Asn1Decoder`. The extension identifier must be exactly `2.5.29.15`. Its OCTET
STRING contents must contain exactly one canonical DER BIT STRING.

At least one defined usage bit must be asserted, as required by RFC 5280. Bits
are numbered from the most significant bit of the first content octet. Bits
beyond `decipherOnly` are rejected. DER named-bit canonicalization is enforced:
the final significant encoded bit must be one, so trailing zero named bits
cannot be retained by lowering the unused-bit count.

## Public contract

```rust
pub const KEY_USAGE_OID: &[u64] = &[2, 5, 29, 15];

pub enum KeyUsageBit {
    DigitalSignature,
    ContentCommitment,
    KeyEncipherment,
    DataEncipherment,
    KeyAgreement,
    KeyCertSign,
    CrlSign,
    EncipherOnly,
    DecipherOnly,
}

pub struct KeyUsage { /* normalized validated bits */ }

impl KeyUsage {
    pub fn contains(self, usage: KeyUsageBit) -> bool;
    pub fn bits(self) -> u16;
}

pub fn decode_key_usage(
    decoder: &mut Asn1Decoder,
    extension: X509Extension<'_>,
) -> Result<KeyUsage, KeyUsageError>;
```

## Limits and diagnostics

The encapsulated value uses the caller's DER framing, total-element, and value
limits. Decoding the inner document shares the decoder's total-element budget
with prior generic-extension work. Because Key Usage is a primitive BIT STRING,
it adds no constructed depth.

Errors contain only a stable category and byte offset local to the encapsulated
Key Usage value. They never retain or format certificate or extension bytes.

## Tests

Tests must cover:

- every defined usage bit and representative combinations;
- the second-octet `decipherOnly` bit and exact normalized masks;
- wrong extension identifiers and non-BIT-STRING payloads;
- empty values, undefined bits, and non-minimal named-bit encodings;
- malformed unused-bit counts, non-zero padding, and trailing DER;
- shared decoder element and value limits;
- payload-local offsets and redacted diagnostics;
- leaving the outer critical flag to certificate policy.

## Deliberate exclusions

- authorizing signing, encryption, agreement, CA, or CRL operations;
- enforcing whether the outer extension is critical;
- combining Key Usage with Basic Constraints or Extended Key Usage;
- certificate version, issuer, subject, or name chaining;
- certificate path construction, path-length processing, or name constraints;
- public-key or signature-algorithm parsing and signature verification;
- clocks, revocation, hostname verification, trust-root loading, TLS, sockets,
  or OAuth transport.
