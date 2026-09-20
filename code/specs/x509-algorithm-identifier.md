# X.509 AlgorithmIdentifier

## Status

Proposed zero-external-dependency Rust boundary above `der-asn1`.

## Purpose

Decode the generic RFC 5280 `AlgorithmIdentifier` container without selecting,
enabling, or implementing any cryptographic algorithm:

```asn1
AlgorithmIdentifier ::= SEQUENCE {
    algorithm   OBJECT IDENTIFIER,
    parameters  ANY DEFINED BY algorithm OPTIONAL
}
```

The same structure identifies certificate signature algorithms and subject
public-key algorithms. This package supplies only reusable syntax. A later
data-driven algorithm registry must decide whether a particular OID is
supported and whether its parameters must be absent, present, `NULL`, or a
specific structure.

## Input contract

The caller supplies one already-framed `Asn1Element` and the same mutable
`Asn1Decoder` that framed it. The root must be one constructed universal
`SEQUENCE` containing:

1. exactly one `OBJECT IDENTIFIER` algorithm field;
2. zero or one parameter element of any canonical DER tag.

The algorithm OID is fully validated through `der-asn1` under the caller's OID
arc limit. An optional parameter is returned as its already-framed borrowed
element, including its exact encoded bytes and tag, but receives no
algorithm-specific interpretation. A missing OID, malformed OID, malformed
parameter framing, or third child fails closed.

## Public contract

```rust
pub struct X509AlgorithmIdentifier<'a> { /* borrowed fields */ }

impl<'a> X509AlgorithmIdentifier<'a> {
    pub fn algorithm(self) -> ObjectIdentifier<'a>;
    pub fn parameters(self) -> Option<Asn1Element<'a>>;
}

pub fn decode_x509_algorithm_identifier<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509AlgorithmIdentifier<'a>, X509AlgorithmIdentifierError>;
```

The result is allocation-free and borrows only from the caller's input.

## Limits and offsets

All framing, nesting, total-element, and OID-arc limits come from the caller's
decoder. Successful decoding consumes one child element for the algorithm and
one additional child element only when parameters are present, in addition to
the already-decoded root.

Errors contain only a stable category and a byte offset local to the complete
`AlgorithmIdentifier` element. Nested child framing and OID offsets are
translated from the sequence value to that complete-element coordinate space.
Errors never retain or format source bytes.

## Tests

Tests must cover:

- a valid algorithm OID with absent parameters;
- `NULL`, constructed, and otherwise arbitrary canonical parameter elements;
- primitive and non-SEQUENCE roots;
- missing, wrongly tagged, malformed, and over-budget OIDs;
- malformed optional-parameter framing;
- a third child;
- shared decoder depth and total-element exhaustion;
- offsets for failures in both child positions;
- redacted diagnostics and exact borrowed parameter bytes.

## Deliberate exclusions

- an algorithm or OID registry;
- validation of OID-specific parameter presence, absence, type, or value;
- comparison of the certificate and TBSCertificate signature identifiers;
- public-key parsing or signature verification;
- certificate, extension, name, or path decoding;
- cryptographic key access, signing, verification, clocks, trust roots,
  revocation, TLS records, handshakes, sockets, or OAuth transport.
