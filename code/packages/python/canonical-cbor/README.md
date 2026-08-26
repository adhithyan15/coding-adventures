# Canonical CBOR for Python

This package is the native Python implementation of
[`CBR01`](../../../specs/CBR01-canonical-cbor.md): a from-scratch,
zero-production-dependency codec for the RFC 8949 section 4.2.3 length-first
deterministic CBOR profile.

The closed value model contains unsigned and negative 64-bit arguments, byte
and UTF-8 text strings, arrays, ordered map-entry lists, opaque tags, booleans,
and null. Floats, undefined values, indefinite lengths, streaming decode, and
I/O are deliberately outside v1.

## Example

```python
from canonical_cbor import (
    CborMap,
    CborMapEntry,
    CborText,
    CborUnsigned,
    encode_checked,
)

value = CborMap(
    (
        CborMapEntry(CborText("b"), CborUnsigned(2)),
        CborMapEntry(CborText("a"), CborUnsigned(1)),
    )
)

wire = encode_checked(value)
assert wire.hex(" ") == "a2 61 61 01 61 62 02"
```

`encode_checked` stages a complete result and returns immutable `bytes`.
`encode_into_checked` atomically appends that result to a caller-owned
`bytearray`. `decode` accepts immutable or mutable bytes-like input,
defensively copies it, and returns one exact `CborValue`.

## Contract and safety

- Encoding uses the shortest argument form and sorts map keys by encoded
  length, then unsigned byte order.
- Decoding accepts exactly one item and rejects non-minimal arguments,
  indefinite items, non-canonical or duplicate on-wire keys, floats,
  unsupported simple values, and invalid UTF-8.
- Root depth is zero and nesting is capped at 128. Checked encoding is capped
  at 1,048,576 complete bytes; decoding may accept a larger valid item.
- Python `int` preserves the complete unsigned 64-bit domain. Constructors and
  the encoder reject `bool`, subclasses, mutated frozen values, and integers
  outside that domain.
- Text construction rejects surrogate code points and decoding accepts only
  strict UTF-8. Hostile declared lengths and collection counts are rejected
  before allocation.
- `CborError.error_id` exposes exactly the 14 stable CBR01 identifiers. Static
  messages begin with `canonical-cbor:` and contain no payload data.
- Mutable byte and collection inputs are defensively copied. Production uses
  only the Python standard library, requests no repository capabilities, and
  performs pure in-memory computation.

## Development

Run `sh BUILD` on POSIX or `BUILD_windows` on Windows. The front door creates
an isolated Python 3.13 environment, installs the package, executes all 55
shared language-neutral cases plus Python-specific adversarial tests, enforces
95% statement and branch coverage, and runs Ruff and strict mypy checks.
