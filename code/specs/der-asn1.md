# Typed ASN.1 DER Values

## Status

Specification for a zero-external-dependency, allocation-free decoder of the
small typed ASN.1 value vocabulary needed by a later repository-owned X.509
certificate parser. It composes the canonical identifier and length framing
from `der-tlv` with shared depth and total-element budgets, exact schema-tag
checks, and canonical DER value checks.

This package is not an X.509 parser, certificate-path validator, signature
verifier, trust store, TLS implementation, PEM decoder, or network transport.
It does not make concrete OAuth HTTPS safe by itself.

## Standards Boundary

ITU-T X.690 defines the Distinguished Encoding Rules used here. `der-tlv`
already proves canonical identifier and definite-length framing. This layer
adds only the universal and context-specific value rules required to build a
later certificate schema decoder:

- BOOLEAN has exactly one contents octet and uses only `0x00` or DER's
  canonical TRUE value `0xff`;
- INTEGER is non-empty, uses minimal two's-complement contents, and can be
  converted to `u64` only when non-negative and representable;
- BIT STRING begins with an unused-bit count from zero through seven, uses zero
  for an empty payload, and has zero in every declared unused trailing bit;
- OCTET STRING is returned as a borrowed byte slice;
- IA5String is returned as a borrowed string only after every contents octet
  is proven to be seven-bit ASCII;
- NULL has an empty contents encoding;
- OBJECT IDENTIFIER has at least its combined first two arcs, uses minimal
  base-128 subidentifiers, terminates each subidentifier, and cannot overflow
  `u64`;
- SEQUENCE and SET must carry their exact constructed universal tags;
- an explicit context-specific wrapper must be constructed and contain exactly
  one complete DER element.
- schema-selected implicit OCTET STRING, IA5String, and OBJECT IDENTIFIER
  values must carry the exact primitive context-specific tag supplied by the
  caller and reuse the same value validation as their universal forms.

Other universal types remain raw tagged elements for later schema-specific
decoders. In particular this layer does not interpret time, DirectoryString,
GeneralName, Name, AlgorithmIdentifier, SubjectPublicKeyInfo, extensions, or
certificate fields.

## Bounded Decoder

```rust
pub struct Asn1Limits {
    pub der: der_tlv::DerLimits,
    pub max_depth: usize,
    pub max_total_elements: usize,
    pub max_oid_arcs: usize,
}

pub struct Asn1Decoder {
    // private shared work count
}
```

The certificate-oriented defaults retain `DerLimits::default()` and add:

- 32 constructed levels including the root;
- 16,384 total elements across one decoder;
- 128 decoded OID arcs.

`Asn1Decoder::decode_exact` creates a depth-zero root and rejects trailing
data. Every successful root, sibling, or explicit child consumes one shared
total-element unit. `sequence`, `set`, and explicit-wrapper entry derive the
child depth from an unforgeable element wrapper and reject entry before parsing
when the depth limit would be exceeded.

An `Asn1Cursor` is allocation-free and iterative. Its `read` method accepts the
same mutable decoder, so sibling cursors and nested cursors can share one work
budget without self-referential borrows. Failure advances neither the cursor
nor the decoder's element count.

The bounded-work claim applies only when descendants are opened through this
API. Borrowed values remain available for schema interpretation; callers that
independently invoke lower-level framing start a separate budget and cannot
claim that it is the same bounded document walk.

## Public Contract

```rust
pub struct Asn1Element<'a> { /* private framing and depth */ }

impl<'a> Asn1Element<'a> {
    pub fn tag(&self) -> der_tlv::DerTag;
    pub fn header(&self) -> &'a [u8];
    pub fn value(&self) -> &'a [u8];
    pub fn encoded(&self) -> &'a [u8];
    pub fn depth(&self) -> usize;
}

impl Asn1Decoder {
    pub fn new(limits: Asn1Limits) -> Self;
    pub fn decode_exact<'a>(
        &mut self,
        input: &'a [u8],
    ) -> Result<Asn1Element<'a>, Asn1Error>;
    pub fn sequence<'a>(
        &self,
        element: Asn1Element<'a>,
    ) -> Result<Asn1Cursor<'a>, Asn1Error>;
    pub fn set<'a>(
        &self,
        element: Asn1Element<'a>,
    ) -> Result<Asn1Cursor<'a>, Asn1Error>;
    pub fn explicit<'a>(
        &mut self,
        element: Asn1Element<'a>,
        tag_number: u32,
    ) -> Result<Asn1Element<'a>, Asn1Error>;
    pub fn elements_read(&self) -> usize;
}

impl<'a> Asn1Cursor<'a> {
    pub fn read(
        &mut self,
        decoder: &mut Asn1Decoder,
    ) -> Result<Option<Asn1Element<'a>>, Asn1Error>;
    pub fn finish(self) -> Result<(), Asn1Error>;
}
```

The element wrapper is `Copy` because it contains only borrowed immutable
slices, a tag, and a depth. Copying an already-counted element does not consume
more parsing work and cannot create a different depth.

Typed functions accept an `Asn1Element` and require the exact universal class,
constructed bit, and tag number:

```rust
pub fn decode_boolean(element: Asn1Element<'_>) -> Result<bool, Asn1Error>;
pub fn decode_integer(
    element: Asn1Element<'_>,
) -> Result<DerInteger<'_>, Asn1Error>;
pub fn decode_bit_string(
    element: Asn1Element<'_>,
) -> Result<DerBitString<'_>, Asn1Error>;
pub fn decode_octet_string(
    element: Asn1Element<'_>,
) -> Result<&[u8], Asn1Error>;
pub fn decode_implicit_octet_string(
    element: Asn1Element<'_>,
    tag_number: u32,
) -> Result<&[u8], Asn1Error>;
pub fn decode_ia5_string(
    element: Asn1Element<'_>,
) -> Result<&str, Asn1Error>;
pub fn decode_implicit_ia5_string(
    element: Asn1Element<'_>,
    tag_number: u32,
) -> Result<&str, Asn1Error>;
pub fn decode_null(element: Asn1Element<'_>) -> Result<(), Asn1Error>;
pub fn decode_object_identifier(
    element: Asn1Element<'_>,
    limits: Asn1Limits,
) -> Result<ObjectIdentifier<'_>, Asn1Error>;
pub fn decode_implicit_object_identifier(
    element: Asn1Element<'_>,
    tag_number: u32,
    limits: Asn1Limits,
) -> Result<ObjectIdentifier<'_>, Asn1Error>;
```

`DerInteger` exposes the canonical signed contents without allocation,
`is_negative`, and a checked `to_u64`. The positive conversion ignores only a
single required sign-protection `0x00`; it never truncates.

`DerBitString` exposes the unused-bit count, payload bytes, and checked bit
length. `ObjectIdentifier` retains the encoded contents and provides a
cloneable, exact-size-independent arc iterator plus comparison against a caller
slice. Construction validates the complete encoding and arc budget first, so
iteration is infallible and never exposes a partially validated identifier.

## Error Contract

Errors contain only a stable category and a byte offset local to the element or
container being decoded. They never retain or format hostile input. Required
categories include:

- lower-level framing failure;
- unexpected class, primitive/constructed bit, or tag number;
- a child cursor used with a decoder configured with different limits;
- depth or total-element budget exhausted;
- invalid BOOLEAN length or value;
- empty or non-minimal INTEGER, negative unsigned conversion, or unsigned
  overflow;
- missing/invalid BIT STRING unused-bit count or nonzero padding;
- non-empty NULL;
- non-ASCII IA5String contents;
- empty, unterminated, non-minimal, overflowing, or over-budget OID;
- malformed explicit contents or trailing sibling data.

Offsets never contain secret or certificate bytes. No error path may panic,
allocate from a wire value, advance a cursor, or consume shared work after a
failed parse.

## Adversarial Matrix

Tests cover exact tags and constructed bits; root trailing data; nested depth
at and over the limit; total-element exhaustion shared across sibling and
nested cursors; non-advancement after failure; BOOLEAN canonical values and
all alternate encodings; INTEGER sign boundaries, redundant sign octets,
negative conversion, and `u64` overflow; BIT STRING empty and partial-byte
boundaries plus nonzero padding; empty/non-empty NULL; OCTET STRING borrowing;
universal and implicit IA5String ASCII validation with exact offsets; OID
first-arc folding, multi-octet arcs, minimality, truncation, overflow, and arc
limits in universal and implicit forms; exact primitive context-specific tags
and constructed bits; exact explicit wrappers; and SEQUENCE/SET tag confusion.

## Non-Goals

This slice adds no external dependency and no ambient authority. It does not
provide DER encoding, BER/CER acceptance, REAL or decimal conversion, time or
string semantics, ASN.1 schema generation, PEM/Base64, X.509 certificate or
extension parsing, name constraints, signature verification, certificate-path
construction, revocation, trust roots, server-identity integration, TLS
handshakes or records, sockets, provider data, or OAuth transport.
