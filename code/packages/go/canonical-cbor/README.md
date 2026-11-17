# `canonical-cbor` (Go)

A from-scratch, zero-dependency, deterministic CBOR ([RFC 8949](https://www.rfc-editor.org/rfc/rfc8949))
encoder and decoder implementing the **§4.2.3 "Length-First Map Key Ordering"** profile
(standard for CTAP2 / WebAuthn-PRF and COSE, RFC 9052).

## Why Deterministic CBOR

Standard CBOR permits multiple equivalent wire encodings for identical logical objects:
variable integer widths, non-deterministic map ordering, and indefinite streaming lengths.
In cryptographic vaults, authenticated storage, and distributed ledgers, non-deterministic
serialization causes verification failures:

- **AEAD Associated Data (AAD)**: Ciphertext authenticated against associated metadata
  fails to open if that metadata re-serializes differently.
- **Delta Sync & Deduplication**: Comparing revision trees byte-for-byte requires
  deterministic wire bytes across all languages, runtimes, and architectures.
- **Hardware Token Interoperability**: FIDO2 authenticators and CTAP2 clients mandate
  length-first sorted map keys.

This package guarantees a strict bijection between canonical values and on-wire octets:
`EncodeChecked(v)` produces deterministic bytes, and `Decode(b)` enforces that input
bytes adhere strictly to the canonical profile.

## The CBR01 Profile

The encoder produces, and the decoder strictly enforces, the following rules:

| Rule | Encoder Enforcement | Decoder Enforcement |
|---|---|---|
| **Definite lengths only** | Always emits definite headers | Rejects indefinite info (31) and break markers (0xFF) |
| **Smallest-form representation** | Shortest of inline (0..23), 1, 2, 4, or 8 bytes | Rejects non-minimal integer and length headers |
| **Length-first map keys** | Key length ascending, ties broken by bytewise lex | Requires strictly increasing keys under same ordering |
| **Unique map keys** | Rejects duplicate encoded keys (`duplicate-map-key`) | Rejects duplicate keys (`non-canonical-map-order`) |
| **Reserved codes** | Never emits info codes 28, 29, 30 | Rejects info codes 28, 29, 30 (`reserved`) |
| **UTF-8 scalar validation** | Accepts valid Go strings | Validates UTF-8 scalar byte sequences (`invalid-utf8`) |
| **Closed simple values** | Emits `false`, `true`, `null` | Rejects all other simple values (`unsupported-simple`) |
| **No floats** | Never emits floats | Rejects half, single, double floats (`float-not-supported`) |
| **One item per stream** | Serializes exactly one item | Rejects trailing bytes (`trailing-bytes`) |
| **Payload-blind diagnostics** | Returns static literal diagnostics | Zero reflection of payload bytes, keys, or lengths |

## Resource Bounds & Threat Model

- **MaxNestingDepth (128)**: Protects against stack-overflow attacks from deeply nested
  arrays, maps, or tags.
- **MaxEncodedBytes (1,048,576 / 1 MiB)**: Bounds encoder output size to prevent memory
  exhaustion DoS.
- **Pre-allocation Bounds Check**: Decoders verify that declared slice lengths do not
  exceed remaining input bytes before allocating backing arrays (`length-too-large`).
- **Defensive Slices**: Constructors and accessors make defensive copies of byte slices,
  arrays, and maps to preserve caller-slice mutation isolation.
- **Atomic Rollback**: `EncodeIntoChecked` leaves the destination buffer untouched if
  encoding fails.

## Usage

```go
package main

import (
	"fmt"
	canonicalcbor "github.com/adhithyan15/coding-adventures/code/packages/go/canonical-cbor"
)

func main() {
	// Construct structured canonical CBOR value
	val := canonicalcbor.NewMap([]canonicalcbor.MapEntry{
		{Key: canonicalcbor.NewText("title"), Value: canonicalcbor.NewText("hello")},
		{Key: canonicalcbor.NewText("count"), Value: canonicalcbor.NewUnsigned(42)},
	})

	// Encode checked
	bytes, err := canonicalcbor.EncodeChecked(val)
	if err != nil {
		panic(err)
	}

	// Decode back
	decoded, err := canonicalcbor.Decode(bytes)
	if err != nil {
		panic(err)
	}

	fmt.Printf("Round trip success: %v\n", decoded.Equal(val))
}
```

### Passing Sub-Documents Through with Value Spans

`DecodeMapSpans` decodes a canonical map and reports the exact byte range `[Start, End)`
each entry's value occupied in the input:

```go
spans, ok, err := canonicalcbor.DecodeMapSpans(data)
if err != nil {
    // Malformed CBOR profile error
    return err
}
if !ok {
    // Valid CBOR, but not a map (shape mismatch)
    return nil
}

for _, entry := range spans {
    rawValBytes := data[entry.ValueSpan.Start : entry.ValueSpan.End]
    // Invariant: rawValBytes == EncodeChecked(entry.Value)
}
```
