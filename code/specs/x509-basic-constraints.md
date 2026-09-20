# X.509 Basic Constraints

## Status

Implemented as a zero-external-dependency Rust semantic layer above
`x509-extension` and `der-asn1`.

## Purpose

Recognize exactly the RFC 5280 Basic Constraints extension identifier and
decode its encapsulated DER value:

```asn1
id-ce-basicConstraints OBJECT IDENTIFIER ::= { id-ce 19 }

BasicConstraints ::= SEQUENCE {
    cA                  BOOLEAN DEFAULT FALSE,
    pathLenConstraint   INTEGER (0..MAX) OPTIONAL
}
```

This boundary validates one extension value. It does not decide whether a
certificate is a trusted CA.

## Input contract

The caller supplies one already-decoded generic `X509Extension` and a mutable
`Asn1Decoder`. The extension identifier must be exactly `2.5.29.19`. Its
OCTET STRING contents must contain exactly one canonical DER `SEQUENCE`.

An empty sequence represents `cA = FALSE` with no path-length constraint.
Explicitly encoding the default false Boolean is rejected. A present Boolean
must be canonical true. A path-length constraint must be a canonical,
non-negative INTEGER and may appear only when `cA` is true.

The path length remains a borrowed canonical integer rather than being narrowed
to a machine integer. This accepts standards-valid values of any size allowed
by the caller's DER value limit without allocation; later bounded path logic
can compare it with its own maximum path length.

## Public contract

```rust
pub const BASIC_CONSTRAINTS_OID: &[u64] = &[2, 5, 29, 19];

pub struct BasicConstraints<'a> { /* borrowed validated fields */ }

impl<'a> BasicConstraints<'a> {
    pub fn is_ca(self) -> bool;
    pub fn path_len_constraint(self) -> Option<DerInteger<'a>>;
}

pub fn decode_basic_constraints<'a>(
    decoder: &mut Asn1Decoder,
    extension: X509Extension<'a>,
) -> Result<BasicConstraints<'a>, BasicConstraintsError>;
```

## Limits and diagnostics

The encapsulated value uses the caller's DER framing, depth, total-element, and
value limits. Decoding the inner document shares the decoder's total-element
budget with prior generic-extension work. The encapsulated sequence begins a
new DER document depth because RFC 5280 stores it inside an OCTET STRING.

Errors contain only a stable category and byte offset local to the encapsulated
Basic Constraints value. They never retain or format certificate or extension
bytes.

## Tests

Tests must cover:

- the empty default and canonical true CA forms;
- zero and multi-octet path-length constraints;
- a canonical positive path length larger than `u64`;
- wrong extension identifiers and non-SEQUENCE payloads;
- explicit default false, path length without CA, and negative path length;
- malformed and non-minimal DER, trailing fields, and translated offsets;
- shared decoder element and inner-document depth limits;
- redacted diagnostics.

## Deliberate exclusions

- granting CA or trust-anchor authority;
- enforcing whether the outer extension is critical;
- Key Usage or Extended Key Usage processing;
- certificate version, issuer, subject, or name chaining;
- certificate path construction, path-length decrementing, or name constraints;
- public-key or signature-algorithm parsing and signature verification;
- clocks, revocation, hostname verification, trust-root loading, TLS, sockets,
  or OAuth transport.
