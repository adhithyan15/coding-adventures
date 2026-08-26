# Canonical CBOR for C#

This package is the native C# implementation of
[`CBR01`](../../../specs/CBR01-canonical-cbor.md): a from-scratch,
zero-production-NuGet-dependency codec for the RFC 8949 section 4.2.3
length-first deterministic CBOR profile.

The value model contains unsigned and negative `ulong` arguments, byte and
UTF-8 text strings, arrays, ordered map-entry lists, opaque tags, booleans,
and null. Floats, undefined, indefinite lengths, streaming decode, and
filesystem I/O are deliberately outside v1.

## Example

```csharp
using CodingAdventures.CanonicalCbor.CSharp;

var value = new CborMap([
    new(new CborText("b"), new CborUnsigned(2)),
    new(new CborText("a"), new CborUnsigned(1)),
]);

byte[] wire = CanonicalCbor.EncodeChecked(value);
// a2 61 61 01 61 62 02 -- canonical encoded-key order.
var decoded = CanonicalCbor.Decode(wire);
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
- Mutable byte and collection inputs are defensively copied. Production uses
  only the .NET runtime and performs pure in-memory computation.

## Validation

Run `sh BUILD` or `BUILD_windows`. The front door executes all 55 shared
language-neutral cases, the .NET-specific adversarial tests, and a 95% line
coverage gate scoped to the production assembly.
