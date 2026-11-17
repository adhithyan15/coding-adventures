# Canonical CBOR (Swift)

A pure-Swift implementation of deterministic Canonical CBOR following RFC 8949 (CBR01 profile) with length-first map key ordering.

## Specification & Profile (CBR01)

The CBR01 profile enforces a closed, deterministic subset of RFC 8949:
- **Canonical Map Key Ordering (RFC 8949 §4.2.3)**: Keys are sorted first by wire byte length (ascending), then lexicographically by byte values (ascending). Duplicate keys are strictly rejected.
- **Shortest Integer Encodings**: Minimal-width arguments only (0–23 inline, 24–255 in 1 byte, 256–65535 in 2 bytes, 65536–4294967295 in 4 bytes, >=4294967296 in 8 bytes). Non-minimal encodings throw `non-minimal-integer`.
- **Indefinite Length Prohibited**: Indefinite-length strings, arrays, maps, and the break stop code (`0xFF`) are rejected with `indefinite`.
- **IEEE 754 Floats Prohibited**: Major 7 additional info 25, 26, 27 (`0xF9`, `0xFA`, `0xFB`) reject with `float-not-supported`.
- **Strict Scalar UTF-8**: Text strings must strictly conform to Unicode scalars. Overlong sequences, surrogate code points (`U+D800`–`U+DFFF`), and malformed bytes reject with `invalid-utf8`.
- **Protocol Limits**: Maximum nesting depth is strictly 128 (`too-deep` / `encode-too-deep`). Maximum encoded payload size is 1,048,576 bytes (`encode-too-large`).

## Features

- **Pure Computation (`capabilities: []`)**: Zero dependencies on `Foundation` in library code. Built entirely on Swift standard library primitives (`[UInt8]`, `String`, `UInt64`).
- **Strict Concurrency**: All public types conform to `Sendable` and `Equatable`.
- **Atomic Rollback**: `encodeIntoChecked` rolls back destination buffers to their original state upon any encoding failure.
- **Top-Level Span Extraction**: `decodeMapSpans` parses map entries and emits byte ranges `[start, finish)` for downstream zero-copy workflows.

## API Usage

```swift
import CanonicalCbor

// 1. Encode a canonical CBOR data item
let value = CborValue.map([
    CborMapEntry(key: .text("id"), value: .unsigned(101)),
    CborMapEntry(key: .text("active"), value: .boolean(true)),
])
let encodedBytes: [UInt8] = try encodeChecked(value)

// 2. Decode wire bytes
let decodedValue: CborValue = try decode(encodedBytes)
assert(decodedValue == value)

// 3. Decode top-level map spans
let result = try decodeMapSpans(encodedBytes)
if case .map(let spannedEntries) = result {
    for entry in spannedEntries {
        print("Key: \(entry.key), Span: [\(entry.valueSpan.start), \(entry.valueSpan.finish))")
    }
}
```

## Security & Resilience Showcase

| CVE Class | Vulnerability Vector | Defense in Swift Canonical CBOR |
|:---|:---|:---|
| **CVE-1** | `Int(arg)` runtime crash on 64-bit platforms when `arg >= 2^63` | Pre-flight bounds check (`arg <= remainingBytes`) guarantees safe `Int` conversion without traps. |
| **CVE-2** | Deep recursion stack exhaustion / OOM | Hard cap at nesting depth 128 evaluated on recursion entry. |
| **CVE-3** | Malformed UTF-8, overlong ASCII, surrogate injections | Stateful stdlib `UTF8()` scalar decoding strictly validating each code point. |
| **CVE-4** | IEEE float parser confusion | Float additional-info codes (25, 26, 27) rejected with `float-not-supported` before integer minimal-width checks. |
| **CVE-5** | Information disclosure in logs / reflection mirrors | `SpannedMapEntry` implements `CustomReflectable`, `CustomStringConvertible`, and `CustomDebugStringConvertible` with `<redacted>`. |
| **CVE-6** | Non-minimal integer canonical violations | Canonical shortest-form validation across all 7 major types. |
| **CVE-7** | Memory exhaustion / Unbounded buffer growth | Shared 1,048,576-byte item budget threaded across all containers and nested map keys, with atomic buffer rollback on failure. |

## Build & Test

```sh
# POSIX (macOS, Linux)
sh BUILD

# Windows
BUILD_windows.cmd
```
