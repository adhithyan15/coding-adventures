# DER TLV Framing

## Status

Specification for a zero-external-dependency, allocation-free decoder of the
identifier and definite-length framing shared by ASN.1 Distinguished Encoding
Rules (DER) values. This is the next repository-owned primitive required before
X.509 certificate parsing and therefore before OAuth can gain concrete HTTPS.

This package is not an ASN.1 schema decoder, X.509 parser, certificate-path
validator, cryptographic verifier, trust store, TLS implementation, or network
transport.

## Standards Boundary

RFC 5280 identifies certificates and their signed `tbsCertificate` values as
ASN.1 DER tag-length-value encodings. ITU-T X.690 defines the framing rules.
This slice enforces only the rules necessary to split one encoded value from a
byte slice without accepting alternative BER encodings:

- low tag numbers 0 through 30 use the single identifier octet form;
- tag numbers 31 and above use base-128 high-tag-number form;
- the first high-tag-number payload group cannot be zero;
- high-tag-number form cannot encode a value below 31;
- every continuation must terminate within the configured tag bound;
- DER uses definite length only, never BER's indefinite `0x80` form;
- lengths 0 through 127 use short form;
- long-form length uses the fewest octets, has no leading zero, and cannot
  encode a value below 128;
- `0xff` is not a usable length prefix;
- every header and value boundary is checked before slicing.

The universal end-of-contents identifier (tag zero) is rejected because it is
only meaningful with indefinite-length BER framing, which DER forbids.

This layer does not decide whether a tag is legal for a particular ASN.1
schema, whether a universal type is primitive or constructed, whether INTEGER,
BOOLEAN, BIT STRING, SET, time, or string contents are canonical, or whether a
certificate field is semantically valid. Those checks belong to later typed
decoders and must not be inferred from successful framing.

## Bounded Inputs

```rust
pub struct DerLimits {
    pub max_input_len: usize,
    pub max_value_len: usize,
    pub max_elements: usize,
    pub max_tag_number: u32,
}
```

The default certificate-oriented limits are:

- 1 MiB input;
- 1 MiB value;
- 4096 sibling elements per cursor;
- tag number `u32::MAX`.

The input-length limit is checked before its first byte is read. Declared
length is accumulated into `u64`, checked for arithmetic overflow, converted to
`usize` only after a host-capacity check, compared with the configured value
limit, and then checked against the remaining input. A cursor consumes at most
`max_elements`; depth is deliberately absent because this layer does not walk
constructed children recursively.

The wire-length domain is always the unsigned 64-bit domain, independent of
the host index width. A declared length that cannot be represented by the host,
or that cannot be added to the already-decoded header without overflowing the
host index type, reports `LengthHostOverflow` at the length-prefix byte. This
keeps both the error kind and offset stable on 32-bit and 64-bit consumers.

## Public Contract

```rust
pub enum TagClass {
    Universal,
    Application,
    ContextSpecific,
    Private,
}

pub struct DerTag {
    pub class: TagClass,
    pub constructed: bool,
    pub number: u32,
}

pub struct DerElement<'a> {
    // private offsets and borrowed slices
}

impl<'a> DerElement<'a> {
    pub fn tag(&self) -> DerTag;
    pub fn header(&self) -> &'a [u8];
    pub fn value(&self) -> &'a [u8];
    pub fn encoded(&self) -> &'a [u8];
}

pub fn decode_one(
    input: &[u8],
    limits: DerLimits,
) -> Result<(DerElement<'_>, &[u8]), DerError>;

pub fn decode_exact(input: &[u8], limits: DerLimits)
    -> Result<DerElement<'_>, DerError>;

pub struct DerCursor<'a> { /* private */ }

impl<'a> DerCursor<'a> {
    pub fn new(input: &'a [u8], limits: DerLimits) -> Result<Self, DerError>;
    pub fn read(&mut self) -> Result<Option<DerElement<'a>>, DerError>;
    pub fn finish(self) -> Result<(), DerError>;
}
```

`decode_one` returns the first borrowed element and untouched remainder.
`decode_exact` additionally requires that the element consume all input, so a
canonical zero-length value cannot hide trailing bytes. `DerCursor` performs
bounded, iterative sibling decoding without allocation or recursion. A later
typed decoder may open an element's value with another cursor, but it must own
and enforce one shared tree-depth and total-work budget; this package does not
claim that independent cursor limits compose into a whole-document bound.

## Error Contract

Errors identify only a bounded category and byte offset. They never copy or
format hostile input. Required categories include:

- empty/truncated identifier, high tag, length, or value;
- invalid end-of-contents tag;
- non-minimal or overflowing tag number;
- indefinite, reserved, non-minimal, too-wide, or host-overflowing length;
- configured input/value/element/tag limit exceeded;
- trailing data for exact decoding.

No error path may panic, allocate from a declared wire length, or advance a
cursor after failure.

## Portable Conformance Profile

`code/specs/fixtures/der-tlv-v1/` is the closed, language-neutral expression of
this contract. Its JSON Schema fixes the default limits, the 17 stable error
identifiers, the input-segment representation, the operation set, and the
normalized success/error projections. Consumers materialize only bounded
literal or repeated-byte segments; no fixture case names a file, command, or
host resource.

All established implementation lanes must consume the same `cases.json`
document from package-native tests. Successful projections compare the tag,
header length, encoded length, input-relative element offset, and untouched
remainder offset. The test harness derives header, value, encoded, and
remainder bytes from those ranges so large payloads are not duplicated in the
corpus. Failure projections compare only the stable error identifier and byte
offset. Cursor cases additionally compare every ordered event, final sibling
count, and remaining offset, including a repeated failed read that proves
non-advancement.

`code/specs/fixtures/der-tlv-v1/consumers.json`, validated by the adjacent
closed schema, is the authoritative established-lane denominator and closure
registry. Each entry binds one canonical language to its production API,
fixture test, build fronts, truthful empty capability profile, and the five
required surface roles. The aggregate portable-coverage test rejects an
unregistered lane, a sixteenth consumer, path traversal, cross-wiring, or any
consumer that stops loading the shared 54-case corpus.

The portable error identifiers are the kebab-case forms of the public error
categories: `empty-input`, `truncated-high-tag`, `truncated-length`,
`truncated-value`, `end-of-contents`, `non-minimal-tag`, `tag-overflow`,
`indefinite-length`, `reserved-length`, `non-minimal-length`,
`length-too-wide`, `length-host-overflow`, `input-limit-exceeded`,
`value-limit-exceeded`, `element-limit-exceeded`, `tag-limit-exceeded`, and
`trailing-data`.

## Adversarial Matrix

Tests cover every identifier class and primitive/constructed bit, tag 30/31
transition, multi-octet tags, unterminated and leading-zero tags, tag overflow,
length 0/127/128 transition, long-form leading zero, long-form short values,
indefinite and reserved lengths, length-of-length truncation, declared-value
truncation, checked offset arithmetic, exact trailing rejection, exactly-at-
limit acceptance, over-limit rejection, sibling-budget exhaustion, and cursor
non-advancement after an error.

## Non-Goals

This slice adds no external dependency and no ambient capability. It does not
provide PEM/Base64 decoding, ASN.1 type codecs, OID decoding, X.509 fields or
extensions, certificate signatures, name constraints, validity time, path
construction, revocation, trust roots, server-identity matching integration,
TLS handshake/records, sockets, provider data, or OAuth transport.
