# coding_adventures.canonical_cbor (Lua)

Deterministic canonical CBOR codec implementing RFC 8949 §4.2.3 Length-First Map Key Ordering (CBR01 profile).

## Specification & Conformance

This package implements the **CBR01** specification for deterministic CBOR:

| Feature | RFC 8949 Section | Implementation Guarantee |
|---|---|---|
| Map Key Ordering | §4.2.3 | Length-first sorting: shorter serialized keys precede longer ones; ties broken lexicographically by byte values |
| Minimal Argument Encoding | §4.2.1 | Smallest valid integer/length representation strictly required; non-minimal forms rejected |
| Unsigned 64-Bit Boundaries | §3.1 / §4.2.1 | Full unsigned 64-bit space supported via Lua 5.4 native 64-bit integers and unsigned `math.ult` comparisons |
| Reject Floats | Profile rule | Half-, single-, and double-precision floats rejected with closed payload-blind diagnostic (`float-not-supported`) |
| Reject Indefinite Lengths | §3.2.1 | Indefinite-length bytes, strings, arrays, maps, and break stop codes rejected (`indefinite`) |
| Strict UTF-8 Validation | §3.1 | Text strings validated via 3-argument `utf8.len(s, 1, -1)`; surrogate halves and overlong sequences rejected (`invalid-utf8`) |
| Nesting Limit | Profile rule | Hard recursion depth limit of 128 container/tag levels (`too-deep` / `encode-too-deep`) |
| Encoded Size Limit | Profile rule | Hard encoded byte ceiling of 1,048,576 bytes (1 MiB) (`encode-too-large`) |
| Pre-Allocation Defense | Security rule | Bounds check against remaining unread buffer bytes prevents hostile pre-allocation DoS (`length-too-large`) |

## Module Layout

- `coding_adventures.canonical_cbor`: Top-level facade exposing constructors, `encode_checked`, `encode_into_checked`, `decode`, and `decode_map_spans`.
- `coding_adventures.canonical_cbor.constants`: Centralized zero-magic-constant table with read-only metatable protection.
- `coding_adventures.canonical_cbor.messages`: Closed payload-blind diagnostic messages (Standing Rule #20) and `REDACTED_PLACEHOLDER = "<redacted>"`.
- `coding_adventures.canonical_cbor.types`: Tagged algebraic CBOR value models (`CborUnsigned`, `CborNegative`, `CborBytes`, `CborText`, `CborArray`, `CborMapEntry`, `CborMap`, `CborTag`, `CborBool`, `CborNull`), `Span`, `SpannedMapEntry`, `DecodeMapSpansResult`, and `CborError`. All types are immutable proxies protected against metatable TOCTOU spoofing.
- `coding_adventures.canonical_cbor.codec`: High-performance deterministic encoder and zero-allocation cursor decoder with atomic rollback.

## 7-Class Lua CVE Resilience Showcase

1. **CVE-1: Lua 5.4 Two's-Complement Signed Comparison Bypass**: In Lua 5.4, 64-bit integers wrap to negative values when interpreted as signed integers (`0xFFFFFFFFFFFFFFFF == -1`). Using signed comparisons (`<` or `<=`) allows huge lengths (e.g. `5bffffffffffffffff`) to bypass checks because `-1 <= remaining` is `true`. This codec strictly employs `math.ult` for all length and integer boundary checks.
2. **CVE-2: Pre-Allocation OOM Bomb Defense**: Declared array/map/byte counts are validated against remaining buffer bytes via `check_length` before any table allocations occur.
3. **CVE-3: Deep Nesting Stack Exhaustion**: Enforces an exact nesting cap of 128 levels across arrays, maps, and tags during both encode and decode.
4. **CVE-4: Canonical Length-First Key Ordering & Duplicate Key Defense**: Map keys are sorted strictly by encoded byte length, then lexicographical byte value. Duplicate keys are rejected at decode and encode time.
5. **CVE-5: UTF-8 Overlong and Lone-Surrogate Smuggling**: Strict 3-argument `utf8.len(s, 1, -1)` rejects lone surrogates (U+D800..U+DFFF) and non-shortest UTF-8 encodings.
6. **CVE-6: Trailing-Byte Smuggling & Metatable TOCTOU Defense**: Trailing bytes after a decoded root item trigger `trailing-bytes`. Caller tables with metatables are rejected upfront to prevent `__index`/`__len` property spoofing.
7. **CVE-7: Secret Payload Reflection Leak Defense**: All 14 error messages are fixed payload-blind strings. `SpannedMapEntry` redacts keys and values with `<redacted>`.

## Usage Example

```lua
local cbor = require("coding_adventures.canonical_cbor")

-- Deterministic encoding:
local map = cbor.CborMap({
  cbor.CborMapEntry(cbor.CborText("key"), cbor.CborUnsigned(42)),
  cbor.CborMapEntry(cbor.CborText("a"), cbor.CborBool(true)),
})
local wire = cbor.encode_checked(map)

-- Deterministic decoding:
local decoded = cbor.decode(wire)
assert(decoded == map)

-- Spanned map decoding for signature validation:
local res = cbor.decode_map_spans(wire)
if res:is_map() then
  for _, entry in ipairs(res.entries) do
    local slice = string.sub(wire, entry.value_span.start + 1, entry.value_span.finish)
    assert(slice == cbor.encode_checked(entry.value))
  end
end
```

## Running Tests & Coverage

```bash
cd tests && busted . --verbose --pattern=test_
cd tests && lua check_coverage.lua
```
Asserts >= 95% executable line coverage across all source files using Lua's built-in `debug.sethook` harness.
