# X.509 SubjectPublicKeyInfo

## Status

Proposed zero-external-dependency Rust composition above `der-asn1` and
`x509-algorithm-identifier`.

## Purpose

Decode the generic RFC 5280 `SubjectPublicKeyInfo` container without
recognizing a public-key algorithm or interpreting key material:

```asn1
SubjectPublicKeyInfo ::= SEQUENCE {
    algorithm         AlgorithmIdentifier,
    subjectPublicKey  BIT STRING
}
```

This package supplies reusable certificate syntax only. A later data-driven
algorithm registry must decide whether an algorithm OID is supported, validate
its parameters, and interpret the exact BIT STRING payload.

## Input contract

The caller supplies one already-framed `Asn1Element` and the same mutable
`Asn1Decoder` that framed it. The root must be one constructed universal
`SEQUENCE` containing exactly:

1. one valid generic `AlgorithmIdentifier`; and
2. one canonical DER `BIT STRING`.

The BIT STRING is returned exactly as validated by `der-asn1`, including a
possibly empty payload and a non-zero unused-bit count when canonical. Those
properties are algorithm policy rather than generic container syntax. Missing,
malformed, or additional children fail closed.

## Public contract

```rust
pub struct X509SubjectPublicKeyInfo<'a> { /* borrowed fields */ }

impl<'a> X509SubjectPublicKeyInfo<'a> {
    pub fn algorithm(self) -> X509AlgorithmIdentifier<'a>;
    pub fn subject_public_key(self) -> DerBitString<'a>;
}

pub fn decode_x509_subject_public_key_info<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509SubjectPublicKeyInfo<'a>, X509SubjectPublicKeyInfoError>;
```

The result is allocation-free and borrows only from the caller's input.

## Limits and offsets

The outer sequence, nested algorithm identifier, and BIT STRING share the
caller's DER framing, nesting, total-element, and OID-arc limits. Successful
decoding consumes the already-decoded root plus every framed child visited by
the composition.

Errors contain only a stable category and a byte offset local to the complete
`SubjectPublicKeyInfo` element. Nested AlgorithmIdentifier, BIT STRING, and
child-framing offsets are translated into that complete-element coordinate
space. Errors never retain or format source bytes.

## Tests

Tests must cover:

- a valid algorithm with absent and present parameters;
- byte-aligned, non-byte-aligned, and empty canonical key BIT STRINGs;
- primitive and non-SEQUENCE roots;
- missing, wrongly tagged, malformed, and over-budget algorithm identifiers;
- missing, wrongly tagged, and invalid key BIT STRINGs;
- malformed framing in every possible child position;
- a third child;
- shared decoder depth and total-element exhaustion;
- whole-element offsets and redacted diagnostics.

## Deliberate exclusions

- an algorithm or OID registry;
- validation of algorithm-specific parameters or BIT STRING shape;
- RSA, elliptic-curve, EdDSA, or other public-key parsing;
- key access, signing, or signature verification;
- certificate, extension, name, or path decoding;
- clocks, hostname validation, trust roots, revocation, TLS records,
  handshakes, sockets, or OAuth transport.
