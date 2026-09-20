# X.509 Extension

## Status

Shipped zero-external-dependency Rust composition above `der-asn1`.

## Purpose

Decode one generic RFC 5280 `Extension` container without recognizing the
extension identifier or interpreting its encapsulated value:

```asn1
Extension ::= SEQUENCE {
    extnID     OBJECT IDENTIFIER,
    critical   BOOLEAN DEFAULT FALSE,
    extnValue  OCTET STRING
}
```

This package supplies reusable certificate syntax only. A later data-driven
extension registry must decide whether an extension OID is supported and
decode the exact DER value encapsulated by `extnValue`.

## Input contract

The caller supplies one already-framed `Asn1Element` and the same mutable
`Asn1Decoder` that framed it. The root must be one constructed universal
`SEQUENCE` containing exactly:

1. one valid extension `OBJECT IDENTIFIER`;
2. an optional canonical `BOOLEAN` whose only accepted value is `TRUE`; and
3. one primitive `OCTET STRING`.

RFC 5280 declares `critical` with `DEFAULT FALSE`. DER requires a component
equal to its default value to be omitted, so an explicitly encoded `FALSE` is
rejected. An absent field decodes as `false`; encoded `TRUE` decodes as `true`.
The OCTET STRING contents are returned without interpreting or recursively
framing the extension-specific value inside them.

## Public contract

```rust
pub struct X509Extension<'a> { /* borrowed fields */ }

impl<'a> X509Extension<'a> {
    pub fn extension_id(self) -> ObjectIdentifier<'a>;
    pub fn critical(self) -> bool;
    pub fn extension_value(self) -> &'a [u8];
}

pub fn decode_x509_extension<'a>(
    decoder: &mut Asn1Decoder,
    element: Asn1Element<'a>,
) -> Result<X509Extension<'a>, X509ExtensionError>;
```

The result is allocation-free and borrows only from the caller's input.

## Limits and offsets

The outer sequence, extension OID, optional BOOLEAN, and OCTET STRING share the
caller's DER framing, nesting, total-element, and OID-arc limits. Successful
decoding consumes the already-decoded root plus each framed child.

Errors contain only a stable category and a byte offset local to the complete
`Extension` element. Child-framing and typed-value offsets are translated into
that complete-element coordinate space. Errors never retain or format source
bytes.

## Tests

Tests must cover:

- absent `critical` decoding as `false`;
- explicitly encoded canonical `TRUE` decoding as `true`;
- rejection of explicitly encoded `FALSE`;
- empty and non-empty opaque OCTET STRING contents;
- primitive and non-SEQUENCE roots;
- missing, wrongly tagged, malformed, and over-budget extension OIDs;
- invalid BOOLEAN encodings;
- missing, wrongly tagged, and malformed extension values;
- malformed framing in every possible child position;
- a fourth child;
- shared decoder depth and total-element exhaustion;
- whole-element offsets and redacted diagnostics.

## Deliberate exclusions

- an extension or OID registry;
- `Extensions` sequence decoding or duplicate-extension detection;
- parsing the DER value encapsulated by `extnValue`;
- Basic Constraints, Key Usage, Extended Key Usage, Subject Alternative Name,
  Authority Key Identifier, or any other extension semantics;
- critical-extension processing policy;
- certificate, name, or path decoding;
- public-key parsing, signature algorithms, signing, or verification;
- clocks, hostname validation, trust roots, revocation, TLS records,
  handshakes, sockets, or OAuth transport.
