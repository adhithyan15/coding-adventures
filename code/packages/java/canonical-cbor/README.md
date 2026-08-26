# Canonical CBOR for Java

This package is the Java implementation of
[`CBR01`](../../../specs/CBR01-canonical-cbor.md): a from-scratch,
zero-production-dependency codec for RFC 8949 section 4.2.3
length-first deterministic CBOR.

The supported value model contains unsigned and negative 64-bit arguments,
byte and UTF-8 text strings, arrays, maps, opaque tags, booleans, and null.
Floats, undefined, indefinite lengths, streaming, and I/O are deliberately
outside v1.

## Example

```java
import com.codingadventures.canonicalcbor.CanonicalCbor;
import com.codingadventures.canonicalcbor.CborValue;

var value = new CborValue.Map(java.util.List.of(
    new CborValue.MapEntry(new CborValue.Text("b"), new CborValue.Unsigned(2)),
    new CborValue.MapEntry(new CborValue.Text("a"), new CborValue.Unsigned(1))
));

byte[] wire = CanonicalCbor.encodeChecked(value);
// a2 61 61 01 61 62 02 -- keys are sorted by canonical encoded bytes.
var decoded = CanonicalCbor.decode(wire);
```

Java has no unsigned primitive `long`. `Unsigned`, `Negative`, and `Tag`
therefore preserve all 64 wire bits in a signed `long`; callers may use
`Long.parseUnsignedLong` and `Long.toUnsignedString` at boundaries.

## Contract and safety

- Encoding always uses the shortest argument form and canonicalizes map keys
  by encoded length, then unsigned byte order.
- Decoding accepts exactly one item and rejects non-minimal arguments,
  indefinite lengths, non-canonical or duplicate on-wire map keys, floats,
  unsupported simple values, and invalid UTF-8.
- Nesting is capped at 128. Checked encoding publishes no bytes when the
  complete result would exceed 1,048,576 bytes.
- Text construction rejects unpaired UTF-16 surrogates, and UTF-8 size is
  preflighted before allocating an encoded payload. Map-key retention is
  bounded by the same complete-item limit.
- `CborException.id()` exposes the 14 stable CBR01 identifiers. Messages begin
  with `canonical-cbor:` and never include input bytes, lengths, keys, or
  offsets.
- Production code performs pure in-memory computation. The shared JSON corpus
  is read only by tests.

## Validation

`gradle test jacocoTestReport jacocoTestCoverageVerification` runs the complete
55-case `canonical-cbor-v1` corpus and a 95% line-coverage gate. Java and Kotlin
both compare every successful operation to the same exact expected bytes, so
their cross-lane equality is checked byte-for-byte rather than by decoded
meaning alone.
