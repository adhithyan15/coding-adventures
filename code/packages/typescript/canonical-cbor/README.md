# Canonical CBOR for TypeScript

This package is the native TypeScript implementation of
[`CBR01`](../../../specs/CBR01-canonical-cbor.md): a from-scratch,
zero-production-dependency codec for the RFC 8949 section 4.2.3 length-first
deterministic CBOR profile.

The value model contains unsigned and negative `bigint` arguments, byte and
UTF-8 text strings, arrays, ordered map-entry lists, opaque tags, booleans,
and null. Floats, undefined, indefinite lengths, streaming decode, and I/O
are deliberately outside v1.

## Example

```typescript
import {
  CborMap,
  CborMapEntry,
  CborText,
  CborUnsigned,
  encodeChecked,
} from "@coding-adventures/canonical-cbor";

const value = new CborMap([
  new CborMapEntry(new CborText("b"), new CborUnsigned(2n)),
  new CborMapEntry(new CborText("a"), new CborUnsigned(1n)),
]);

const wire = encodeChecked(value);
// a2 61 61 01 61 62 02 -- canonical encoded-key order.
```

## Contract and safety

- Encoding uses the shortest argument form and sorts map keys by encoded
  length, then unsigned byte order.
- Decoding accepts exactly one item and rejects non-minimal arguments,
  indefinite items, non-canonical or duplicate on-wire keys, floats,
  unsupported simple values, and invalid UTF-8.
- Root depth is zero and nesting is capped at 128. Checked encoding is staged
  atomically and capped at 1,048,576 complete bytes; decoding may accept a
  larger valid item.
- All unsigned, negative, and tag arguments remain `bigint`, preserving the
  full unsigned 64-bit domain without JavaScript number rounding.
- Text construction rejects lone UTF-16 surrogates; decoding uses fatal UTF-8
  conversion and preserves U+FEFF. Hostile declared lengths are rejected
  before allocation.
- `CborError.id` exposes exactly the 14 stable CBR01 identifiers. Static
  messages begin with `canonical-cbor:` and contain no payload data.
- Mutable byte and collection inputs are defensively copied. Production uses
  only the JavaScript runtime and performs pure in-memory computation.

## Validation

Run `sh BUILD` or `BUILD_windows`. The front door compiles the package and
executes all 55 shared language-neutral cases, TypeScript-specific adversarial
tests, and a 90% production line-coverage gate.
