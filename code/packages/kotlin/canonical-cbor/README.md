# Canonical CBOR for Kotlin

This package is the Kotlin implementation of
[`CBR01`](../../../specs/CBR01-canonical-cbor.md): a from-scratch codec with no
application production dependency beyond the Kotlin runtime closure for RFC
8949 section 4.2.3
length-first deterministic CBOR.

The supported value model contains unsigned and negative 64-bit arguments,
byte and UTF-8 text strings, arrays, maps, opaque tags, booleans, and null.
Floats, undefined, indefinite lengths, streaming, and I/O are deliberately
outside v1.

## Example

```kotlin
import com.codingadventures.canonicalcbor.CanonicalCbor
import com.codingadventures.canonicalcbor.CborValue

val value = CborValue.Map(listOf(
    CborValue.MapEntry(CborValue.Text("b"), CborValue.Unsigned(2u)),
    CborValue.MapEntry(CborValue.Text("a"), CborValue.Unsigned(1u)),
))

val wire = CanonicalCbor.encodeChecked(value)
// a2 61 61 01 61 62 02 -- keys are sorted by canonical encoded bytes.
val decoded = CanonicalCbor.decode(wire)
```

Kotlin's `ULong` preserves the whole CBOR unsigned domain without a wrapper or
lossy conversion. `CborValue.Bytes` defensively copies caller arrays and uses
content equality.

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
- `CborException.id` exposes the 14 stable CBR01 identifiers. Messages begin
  with `canonical-cbor:` and never include input bytes, lengths, keys, or
  offsets.
- Production code performs pure in-memory computation. The shared JSON corpus
  is read only by tests. The resolved runtime closure is Kotlin stdlib 2.1.20
  plus its transitive JetBrains annotations 13.0 library; neither grants
  filesystem, process, environment, network, or other host authority.

## Validation

`gradle test jacocoTestReport jacocoTestCoverageVerification` runs the complete
55-case `canonical-cbor-v1` corpus and a 95% line-coverage gate. Kotlin and Java
both compare every successful operation to the same exact expected bytes, so
their cross-lane equality is checked byte-for-byte rather than by decoded
meaning alone.
