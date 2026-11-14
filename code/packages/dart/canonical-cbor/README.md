# `canonical-cbor` (Dart)

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
`encodeChecked(v)` produces deterministic bytes, and `decode(b)` enforces that input
bytes adhere strictly to the canonical profile.

## The CBR01 Profile

The encoder produces, and the decoder strictly enforces, the following rules:

| Rule | Encoder Enforcement | Decoder Enforcement |
|---|---|---|
| **Definite lengths only** | Always emits definite headers | Rejects indefinite info (31) and break markers (0xFF) |
| **Smallest-form representation** | Shortest of inline (0..23), 1, 2, 4, or 8 bytes | Rejects non-minimal integer and length headers |
| **Length-first map keys** | Key length ascending, ties broken by bytewise lex | Requires strictly increasing keys under same ordering |
| **Unique map keys** | Rejects duplicate encoded keys (`duplicateMapKey`) | Rejects duplicate keys (`nonCanonicalMapOrder`) |
| **Reserved codes** | Never emits info codes 28, 29, 30 | Rejects info codes 28, 29, 30 (`reserved`) |
| **UTF-8 scalar validation** | Scans UTF-16 code units; rejects lone surrogates | Validates UTF-8 scalar byte sequences (`invalidUtf8`) |
| **Closed simple values** | Emits `false`, `true`, `null` | Rejects all other simple values (`unsupportedSimple`) |
| **No floats** | Never emits floats | Rejects half, single, double floats (`floatNotSupported`) |
| **One item per stream** | Serializes exactly one item | Rejects trailing bytes (`trailingBytes`) |
| **Payload-blind diagnostics** | Returns static literal diagnostics | Zero reflection of payload bytes, keys, or lengths |

## Dart Implementation Highlights

- **Sealed Class Hierarchy**: `CborValue` is a sealed class with exhaustive subtype branches
  (`CborUnsigned`, `CborNegative`, `CborBytes`, `CborText`, `CborArray`, `CborMap`, `CborTag`,
  `CborBool`, `CborNull`), enabling compiler-checked pattern matching.
- **Exact 64-bit Unsigned Range (`BigInt`)**: Dart VM `int` is a signed 64-bit integer,
  meaning numbers in `[2^63, 2^64 - 1]` cannot be represented without signed overflow or sign-bit
  distortion. `CborUnsigned`, `CborNegative`, and `CborTag` wrap `BigInt`, guaranteeing exact
  unsigned 64-bit range fidelity up to `18446744073709551615`.
- **UTF-16 Surrogate Defense (CVE-5)**: In Dart, `utf8.encode(String.fromCharCode(0xD800))`
  silently outputs replacement character bytes `0xEF 0xBF 0xBD` (`U+FFFD`) rather than throwing.
  This package proactively scans `String.codeUnits` for lone surrogate codes (`0xD800..0xDFFF`)
  and rejects them with `CborErrorId.invalidUtf8` before encoding, eliminating hash-collision
  and signature-bypass attacks.
- **Pure Computation**: Zero dependencies (`required_capabilities.json: []`). Uses only `dart:typed_data`
  and `dart:convert`. `dart:io`, `dart:mirrors`, and `dart:ffi` are completely absent from `lib/`.
- **Atomic Rollback**: `encodeIntoChecked` records the initial buffer length and truncates it back
  to original length if any serialization or limit error occurs.

## Resource Bounds & Threat Model

- **MaxNestingDepth (128)**: Protects against stack-overflow attacks from deeply nested
  arrays, maps, or tags.
- **MaxEncodedBytes (1,048,576 / 1 MiB)**: Bounds encoder output size to prevent memory
  exhaustion DoS.
- **Pre-allocation Bounds Check**: Decoders verify that declared slice lengths do not
  exceed remaining input bytes before allocating collections (`lengthTooLarge`).
- **Defensive Slices**: Constructors and accessors make defensive copies of `Uint8List` byte buffers,
  arrays, and maps to preserve caller mutation isolation.
- **Redacted Diagnostics**: `SpannedMapEntry.toString()` redacts sensitive key/value payloads
  to prevent credentials or secrets from leaking into application logs.

## Usage

```dart
import 'dart:typed_data';
import 'package:coding_adventures_canonical_cbor/canonical_cbor.dart';

void main() {
  // Construct structured canonical CBOR value
  final val = CborMap([
    CborMapEntry(CborText('title'), CborText('hello')),
    CborMapEntry(CborText('count'), CborUnsigned.fromInt(42)),
  ]);

  // Encode checked
  final bytes = encodeChecked(val);

  // Decode back
  final decoded = decode(bytes);

  print('Round trip match: ${decoded == val}');
}
```

### Passing Sub-Documents Through with Value Spans

`decodeMapSpans` decodes a canonical map and reports the exact byte range `[start, end)`
each entry's value occupied in the input:

```dart
final result = decodeMapSpans(data);
if (!result.isMap) {
  // Valid CBOR, but not a map (shape mismatch)
  return;
}

for (final entry in result.entries) {
  // byte span matches exact re-encoding of entry.value
  final subBytes = data.sublist(entry.span.start, entry.span.end);
  assert(identical(subBytes, encodeChecked(entry.value)));
}
```
