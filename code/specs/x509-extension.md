# X.509 Extension

## Status

Portable zero-external-dependency composition above `der-asn1`, targeting all
15 established implementation lanes through one closed language-neutral v1
corpus. Rust remains the reference implementation. C, C++, and OCaml remain
emerging lanes and do not change the denominator.

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
`Asn1Decoder` state that framed it. The root must be one constructed universal
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

Other lanes expose an idiomatic equivalent while preserving validated OID
arcs, the exact critical Boolean, a byte-exact opaque value, stable errors,
whole-element offsets, and the caller's shared decoder state. Borrowing,
allocation, copying, object identity, and native integer representation are
language-specific rather than portable requirements. Returned values must be
immutable to callers or defensive snapshots, and no public constructor may
forge a validated extension or a typed DER value.

## Limits and offsets

The outer sequence, extension OID, optional BOOLEAN, and OCTET STRING share the
caller's DER framing, nesting, total-element, and OID-arc limits. Successful
decoding consumes the already-decoded root plus each framed child.

Errors contain only a stable category and a byte offset local to the complete
`Extension` element. Child-framing and typed-value offsets are translated into
that complete-element coordinate space. Errors never retain or format source
bytes.

Portable projections use one of these eight extension categories:

- `structure`, carrying the exact nested DER ASN.1 category and, for framing
  failures, the exact DER TLV category;
- `missing-extension-id`;
- `invalid-extension-id`, carrying the exact nested DER ASN.1 category;
- `invalid-critical`, carrying the exact nested DER ASN.1 category;
- `encoded-default-critical`;
- `missing-extension-value`;
- `invalid-extension-value`, carrying the exact nested DER ASN.1 category;
- `trailing-element`.

The extension category is stable across lanes. Nested category names are those
from the versioned `der-asn1-v1` and `der-tlv-v1` contracts rather than host
exception names.

Failure is transactional at the extension boundary. A failed decode must not
return a partial extension. Child-cursor position and the shared successful
element count may change only for children that were successfully framed before
the semantic failure, exactly as projected by the neutral corpus. A framing or
budget failure itself consumes no additional element and does not advance past
the failing child.

## Language-Neutral Conformance

The normative portable corpus is
`code/specs/fixtures/x509-extension-v1/cases.json`, validated by its closed
schema and an independent oracle. Inputs are complete DER `Extension` elements.
The runner first uses the same package-native `der-asn1` decoder to frame the
root, then calls the package-native generic extension decoder with that same
decoder state. This preserves the shared depth, total-element, DER framing, and
OID-arc budgets instead of creating a second walk.

Success projections contain decimal-string OID arcs, the Boolean critical
value, lowercase hexadecimal opaque value bytes, and the shared successful
element count. Decimal strings prevent host numeric precision from becoming a
protocol rule. Error projections contain the stable extension category,
whole-element byte offset, shared successful element count, and any required
nested DER ASN.1 or DER TLV category. A hostile payload listed only for
redaction testing must never appear in a projected result or public diagnostic.

The closed consumer registry lists all 15 established lanes. Registry
membership is a target obligation, not evidence of completion by itself. The
aggregate gate requires each consumer's production source, package-native
fixture test, README, changelog, capability manifest, dependency metadata, and
real BUILD fronts before closure.

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

The neutral suite additionally covers the first two OID arcs folded into one
multi-octet subidentifier, the unsigned-64 arc boundary, exact and exceeded OID
arc budgets, constructed and alternate-tag confusion at every schema position,
repeated failure without partial output, exact shared element-budget boundaries,
and byte-exact opaque values that are deliberately not recursively decoded.

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

This slice also adds no process, filesystem, environment, clock, randomness,
network, credential, native-link, or external-service authority.
