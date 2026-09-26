# Typed ASN.1 DER Values

## Status

Specification for a zero-external-dependency typed layer over the
repository-owned DER TLV decoder, covering the small ASN.1 value vocabulary
needed by a later repository-owned X.509 certificate parser.
It composes the canonical identifier and length framing from `der-tlv` with
shared depth and total-element budgets, exact schema-tag checks, and canonical
DER value checks. Allocation and ownership strategy are language-specific;
portable behavior requires byte-exact projections, bounded work, and defensive
results rather than one representation technique.

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
  converted into the unsigned 64-bit range only when non-negative and
  representable;
- BIT STRING begins with an unused-bit count from zero through seven, uses zero
  for an empty payload, and has zero in every declared unused trailing bit;
- OCTET STRING is returned as a byte-exact sequence;
- IA5String is returned as text only after every contents octet is proven to be
  seven-bit ASCII;
- NULL has an empty contents encoding;
- OBJECT IDENTIFIER has at least its combined first two arcs, uses minimal
  base-128 subidentifiers, terminates each subidentifier, and no arc may exceed
  `2^64 - 1`;
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

An `Asn1Cursor` is semantically iterative. Its `read` method accepts the same
decoder state, so sibling cursors and nested cursors share one work budget.
Failure advances neither the cursor nor the decoder's element count. A lane may
borrow immutable input or retain bounded defensive copies without changing this
contract.

The bounded-work claim applies only when descendants are opened through this
API. Validated values remain available for schema interpretation; callers that
independently invoke lower-level framing start a separate budget and cannot
claim that it is the same bounded document walk.

## Public Contract

The Rust signatures below define the reference surface. Other lanes use
idiomatic equivalents while preserving the same validated values, stable
errors, offsets, limits, and shared decoder state; borrowing, copying, and
native integer representation are not portable requirements.

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

The Rust element wrapper is `Copy` because it contains only borrowed immutable
slices, a tag, and a depth. Other lanes may expose immutable value wrappers or
defensive snapshots. Reusing an already-counted element never consumes more
parsing work and cannot create a different depth.

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

`DerInteger` exposes the canonical signed contents, `is_negative`, and a checked
nonnegative conversion whose portable projection is an exact decimal string in
the range zero through `2^64 - 1`. The positive conversion ignores only a
single required sign-protection `0x00`; it never truncates. Lanes without a
native unsigned 64-bit value may use an exact big integer or another lossless
representation.

`DerBitString` exposes the unused-bit count, payload bytes, and checked bit
length. `ObjectIdentifier` retains the encoded contents and exposes the
complete ordered validated arcs, lazily or as an immutable materialized
collection, plus exact comparison against caller-provided arcs. Construction
validates the complete encoding and arc budget first, so access is infallible
and never exposes a partially validated identifier.

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
perform unbounded allocation derived from a hostile wire length, advance a
cursor, or consume shared work after a failed parse.

## Adversarial Matrix

Tests cover exact tags and constructed bits; root trailing data; nested depth
at and over the limit; total-element exhaustion shared across sibling and
nested cursors; non-advancement after failure; BOOLEAN canonical values and
all alternate encodings; INTEGER sign boundaries, redundant sign octets,
negative conversion, and unsigned-64 overflow; BIT STRING empty and partial-byte
boundaries plus nonzero padding; empty/non-empty NULL; OCTET STRING byte
equality;
universal and implicit IA5String ASCII validation with exact offsets; OID
first-arc folding, multi-octet arcs, minimality, truncation, overflow, and arc
limits in universal and implicit forms; exact primitive context-specific tags
and constructed bits; exact explicit wrappers; and SEQUENCE/SET tag confusion.

## Language-Neutral Conformance

The normative portable behavior corpus is
`code/specs/fixtures/der-asn1-v1/cases.json`, validated by its closed schema.
It composes with the DER TLV v1 corpus through stable case references instead
of duplicating identifier and length bytes. Every DER TLV reference must name
an exact-decode row, and every consumer must preserve its raw-element or
framing-error projection before applying typed semantics.

Portable success values use byte-exact lowercase-hex projections, decimal
strings for values in the unsigned 64-bit range and OID arcs, and explicit depth and shared-work
counters. This avoids host numeric precision becoming an accidental protocol
rule. Portable errors use the stable typed category, a numeric byte offset, and
an explicit `operation-input` or `container-value` domain. A lower-level
failure additionally carries the exact DER TLV framing identifier.

The contract targets all 15 established implementation lanes through
package-native tests and empty capability manifests. The aggregate registry
gate closes the final consumer set and requires each package's real BUILD
fronts, metadata, README, changelog, production source, and fixture test. Until
that gate closes, the registry may list planned consumers whose implementation
is still in progress; the roadmap records the measured lane count. C, C++, and
OCaml remain emerging lanes; explicit OCaml DER TLV and DER ASN.1 consumer
owners are registered behind the current-contract OCaml build tool without
changing the established denominator.

## Non-Goals

This slice adds no external dependency and no ambient authority. It does not
provide DER encoding, BER/CER acceptance, REAL or decimal conversion, time or
string semantics, ASN.1 schema generation, PEM/Base64, X.509 certificate or
extension parsing, name constraints, signature verification, certificate-path
construction, revocation, trust roots, server-identity integration, TLS
handshakes or records, sockets, provider data, or OAuth transport.
