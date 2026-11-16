# Canonical CBOR for Elixir (`CodingAdventures.CanonicalCbor`)

Deterministic, canonical [CBOR (RFC 8949)](https://www.rfc-editor.org/rfc/rfc8949) encoder and decoder in pure Elixir.

## Features

- **Strict RFC 8949 §4.2.3 Length-First Map Key Ordering** (CBR01 profile):
  - Deterministic serialization guarantees `decode(encode(v)) == v` and identical wire bytes across all languages, platforms, and architectures.
  - Shortest-form integer encoding (§4.2.1) across all major types.
  - Symmetrical recursion depth enforcement (`MAX_NESTING_DEPTH = 128`) on both map keys and values.
  - Definite length only; indefinite-length items and breaks are rejected.
  - Floats are strictly rejected (no float smuggling or NaN variance).
- **Pure Computation**: Zero runtime Hex dependencies (`deps: []`), pure Elixir (`capabilities: []`), no filesystem, process, or network authority.
- **Standing Rule #20 Compliance**: 100% of error diagnostics and `<redacted>` security placeholder are centralized exclusively in `Messages.ex` (zero inline error strings, zero user/payload reflection, zero dynamic atom creation via `String.to_atom/1`).

## 7-Class BEAM/Elixir CVE Resilience Showcase

| CVE Class | Vulnerability Vector | Defense Mechanism in `canonical_cbor` |
|:---|:---|:---|
| **CVE-1** | 64-bit integer wraparound and float precision traps | Validates integers strictly via `is_integer/1` within `0 .. 18_446_744_073_709_551_615`. Rejects floats in integer and tag positions without truncation. |
| **CVE-2** | Pre-allocation OOM expansion bombs | Evaluates declared item and byte lengths against remaining buffer capacity before allocating (`_check_length/3`). |
| **CVE-3** | Deep nesting recursion stack exhaustion | Caps container nesting depth at `MAX_NESTING_DEPTH = 128` symmetrically across arrays, tags, map values, and map keys (`depth + 1`). |
| **CVE-4** | Map key collision and atom table exhaustion | Enforces length-first then lexicographical map key ordering. Rejects duplicate keys on encode (`:duplicate_map_key`) and decode (`:non_canonical_map_order`). Zero `String.to_atom/1` calls. |
| **CVE-5** | Strict UTF-8 vs surrogate smuggling & unaligned bitstrings | Enforces `String.valid?/1` to reject surrogates (U+D800..U+DFFF) and overlong UTF-8 encodings. Validates byte buffers via `is_binary/1` to reject unaligned bitstrings (`<<1::3>>`). |
| **CVE-6** | Trailing bytes and forged struct tampering | Rejects unconsumed bytes at root with `:trailing_bytes`. Re-validates all struct fields on encode to prevent forged struct manipulation. |
| **CVE-7** | Secret leakage via diagnostics and stringification | All 14 CBR01 error diagnostics are fixed payload-blind strings without input reflection. `SpannedMapEntry` implements `Inspect` redacting keys and values with `<redacted>`. |

## Usage

```elixir
alias CodingAdventures.CanonicalCbor

# Construct CBOR values
u = CanonicalCbor.cbor_unsigned(42)
t = CanonicalCbor.cbor_text("hello")
m = CanonicalCbor.cbor_map([
  CanonicalCbor.cbor_map_entry(CanonicalCbor.cbor_text("title"), t),
  CanonicalCbor.cbor_map_entry(CanonicalCbor.cbor_text("count"), u)
])

# Deterministic encoding
{:ok, wire} = CanonicalCbor.encode_checked(m)

# Decoding
{:ok, decoded} = CanonicalCbor.decode(wire)
assert decoded == m

# Spanned map entry decoding
{:ok, res} = CanonicalCbor.decode_map_spans(wire)
for entry <- res.entries do
  IO.puts(inspect(entry))
  # => SpannedMapEntry(key: <redacted>, value: <redacted>, value_span: [17, 24))
end
```
