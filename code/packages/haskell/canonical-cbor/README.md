# canonical-cbor (Haskell)

`canonical-cbor` provides a strict, pure, deterministic implementation of RFC 8949 Canonical CBOR (specifically the **CBR01 profile**) for Haskell.

## Conformance Profile: CBR01

1. **RFC 8949 §4.2.3 Length-First Map Key Ordering**:
   - Keys are sorted first by raw byte length (shorter preceding longer).
   - Keys of identical length are sorted lexicographically by raw byte values.
2. **Shortest Integer Encoding**:
   - Integers in `0..23` are encoded directly in the initial byte.
   - Integers in `24..0xFF` use 1-byte arguments.
   - Integers in `0x100..0xFFFF` use 2-byte arguments.
   - Integers in `0x10000..0xFFFFFFFF` use 4-byte arguments.
   - Integers in `0x100000000..0xFFFFFFFFFFFFFFFF` use 8-byte arguments.
3. **Indefinite-Length Items Rejected**:
   - Streaming/chunked arrays, maps, byte strings, and text strings with additional info `31` (`0x1F`) are strictly rejected.
4. **Nesting Depth Limit**:
   - Symmetrically bounded at `maxNestingDepth = 128` across arrays, tags, map values, and map keys (`depth + 1`).
5. **Pre-Allocation Capacity Guard**:
   - Length headers for strings and collections are verified against remaining wire bytes prior to allocation.

---

## 7-Class Haskell/GHC CVE Resilience Showcase

| CVE Class | Threat & Vector | Haskell Defense in `canonical-cbor` |
|---|---|---|
| **CVE-1: Word64 -> signed Int wraparound** | Wire argument `0x5b80000000000001` wraps to negative `Int64` if converted naively | All length comparisons (`arg <= remaining`, `arg <= maxEncodedBytes`) are performed in unsigned `Word64` before any conversion to `Int` |
| **CVE-2: Billion laughs / allocation bomb** | High length header followed by truncated or small payload | Capacity guard enforces `arg <= fromIntegral (BS.length rest)` before allocating slices or items |
| **CVE-3: Recursion stack bomb** | Depth 129+ deeply nested arrays, tags, or map keys | Symmetrical `depth > maxNestingDepth` guard on decode and encode; depth 128 succeeds, depth 129 fails cleanly |
| **CVE-4: Non-canonical map keys** | Duplicate keys or out-of-order map keys on wire | Strict sorting check on decode (`NonCanonicalMapOrder`, `DuplicateMapKey`) and length-first sort on encode |
| **CVE-5: Secret leakage via logging / Show** | Stringification of `SpannedMapEntry` dumps raw confidential payload | Custom `Show` on `SpannedMapEntry` unconditionally redacts keys and values with `<redacted>` |
| **CVE-6: Invalid UTF-8 injection** | Ill-formed byte sequence in major type 3 text string | Validated using `Data.Text.Encoding.decodeUtf8'` without throwing exceptions or inserting replacement characters |
| **CVE-7: Structural-sharing memory explosion** | Repeated references to shared memory blocks allocating gigabytes | Incremental byte budget tracking in `pushBytes` fails fast with `EncodeTooLarge` before memory exhaustion |

---

## Standing Rule #20: 100% Extracted Payload-Blind Diagnostics

All 14 CBR01 error diagnostics, `<redacted>` placeholder, `<invalid>` placeholder, and unknown error fallbacks are consolidated exclusively in `CodingAdventures.CanonicalCbor.Messages`.
No user-supplied inputs, keys, values, offsets, or sizes are reflected in error diagnostics.

---

## Build and Test

```bash
# Unix (Darwin CLT linker guard included)
sh BUILD

# Windows
BUILD_windows
```
