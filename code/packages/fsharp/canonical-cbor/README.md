# Canonical CBOR for F#

This package is the native F# implementation of
[`CBR01`](../../../specs/CBR01-canonical-cbor.md): an independent,
zero-explicit-production-package codec for the RFC 8949 section 4.2.3
length-first deterministic CBOR profile. The normal F# runtime closure includes
`FSharp.Core`; no additional production package is referenced.

The closed value algebra contains unsigned and negative `uint64` arguments,
byte and UTF-8 text strings, arrays, ordered map-entry lists, opaque tags,
booleans, and null. Floats, undefined, indefinite lengths, streaming decode,
and filesystem I/O are deliberately outside v1.

## Example

```fsharp
open CodingAdventures.CanonicalCbor.FSharp

let value =
    CborValue.map
        [ CborValue.text "b", CborValue.unsigned 2UL
          CborValue.text "a", CborValue.unsigned 1UL ]

let wire = CanonicalCbor.encodeChecked value
// a2 61 61 01 61 62 02 -- canonical encoded-key order.
let decoded = CanonicalCbor.decode wire
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
- Text construction rejects lone UTF-16 surrogates. UTF-8 size and retained
  encoded map keys are bounded before publication or attacker-driven growth.
- `CborException.Id` exposes exactly the 14 stable CBR01 identifiers. Static
  messages begin with `canonical-cbor:` and contain no payload data.
- Byte arrays and collection inputs are copied into an immutable public
  algebra. Production performs pure in-memory computation.

## Validation

Run `sh BUILD` or `BUILD_windows`. The front door executes all 55 shared
language-neutral cases, the .NET-specific adversarial tests, and a 95% line
coverage gate scoped to the production assembly.
