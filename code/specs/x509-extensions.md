# X.509 Extensions

## Status

Implemented as a zero-external-dependency Rust composition above
`x509-extension`.

## Purpose

Decode one bounded RFC 5280 `Extensions` container without recognizing an
extension identifier or interpreting an encapsulated value:

```asn1
Extensions ::= SEQUENCE SIZE (1..MAX) OF Extension
```

RFC 5280 requires an extension identifier to appear no more than once in a
certificate. This package enforces that syntax invariant while preserving the
wire order of fully validated generic extension values.

## Input contract

The caller supplies one already-framed `Asn1Element` and the same mutable
`Asn1Decoder` that framed it. The root must be one constructed universal
`SEQUENCE` containing from 1 through 64 values accepted by
`decode_x509_extension`.

The result stores those borrowed values in a fixed-capacity array. It performs
no attacker-sized allocation and rejects a 65th extension. Duplicate extension
identifiers are compared by their validated canonical DER contents and rejected
at the duplicate extension's offset.

## Public contract

```rust
pub const MAX_X509_EXTENSIONS: usize = 64;

pub struct X509Extensions<'a> { /* fixed-capacity borrowed fields */ }

impl<'a> X509Extensions<'a> {
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn get(&self, index: usize) -> Option<X509Extension<'a>>;
    pub fn iter(&self) -> impl Iterator<Item = X509Extension<'a>> + '_;
}

pub fn decode_x509_extensions<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509Extensions<'a>, X509ExtensionsError>;
```

## Limits and offsets

The outer sequence and every nested generic extension share the caller's DER
framing, nesting, total-element, and OID-arc limits. Successful decoding
consumes the already-decoded root, each extension element, and every nested
extension field exactly once.

Errors contain only a stable category and a byte offset local to the complete
`Extensions` element. Child-framing and nested-extension offsets are translated
into that coordinate space. Errors never retain or format source bytes.

## Tests

Tests must cover:

- one and multiple extensions with stable wire-order iteration;
- the exact fixed capacity;
- empty-sequence and 65th-extension rejection;
- primitive and non-SEQUENCE roots;
- duplicate identifiers with different critical flags and values;
- nested generic-extension validation and translated offsets;
- malformed child framing;
- shared decoder depth and total-element exhaustion;
- redacted diagnostics.

## Deliberate exclusions

- an extension or OID registry;
- parsing the DER value encapsulated by `extnValue`;
- Basic Constraints, Key Usage, Extended Key Usage, Subject Alternative Name,
  Authority Key Identifier, or any other extension semantics;
- unknown-critical-extension processing policy;
- certificate, name, or path decoding;
- public-key parsing, signature algorithms, signing, or verification;
- clocks, hostname validation, trust roots, revocation, TLS records,
  handshakes, sockets, or OAuth transport.
