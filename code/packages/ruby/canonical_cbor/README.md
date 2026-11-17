# `canonical_cbor` (Ruby)

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
`encode_checked(v)` produces deterministic bytes, and `decode(b)` enforces that input
bytes adhere strictly to the canonical profile.

## The CBR01 Profile

The encoder produces, and the decoder strictly enforces, the following rules:

| Rule | Encoder Enforcement | Decoder Enforcement |
|---|---|---|
| **Definite lengths only** | Always emits definite headers | Rejects indefinite info (31) and break markers (0xFF) (`indefinite`) |
| **Smallest-form representation** | Shortest of inline (0..23), 1, 2, 4, or 8 bytes | Rejects non-minimal integer and length headers (`non_minimal_integer`) |
| **Length-first map keys** | Key length ascending, ties broken by bytewise lex | Requires strictly increasing keys under same ordering (`non_canonical_map_order`) |
| **Unique map keys** | Rejects duplicate encoded keys (`duplicate_map_key`) | Rejects duplicate keys (`non_canonical_map_order`) |
| **Reserved codes** | Never emits info codes 28, 29, 30 | Rejects info codes 28, 29, 30 (`reserved`) |
| **UTF-8 validation** | Rejects non-UTF-8 encodings and invalid byte sequences | Validates UTF-8 byte sequences strictly (`invalid_utf8`) |
| **Closed simple values** | Emits `false`, `true`, `null` | Rejects all other simple values (`unsupported_simple`) |
| **No floats** | Never emits floats | Rejects half, single, double floats (`float_not_supported`) |
| **One item per stream** | Serializes exactly one item | Rejects trailing bytes (`trailing_bytes`) |
| **Nesting & size limits** | Max depth 128 (`encode_too_deep`), max size 1 MiB (`encode_too_large`) | Max depth 128 (`too_deep`), max size 1 MiB (`length_too_large`) |
| **Payload-blind diagnostics** | Returns static literal diagnostics | Zero reflection of payload bytes, keys, or lengths (`<redacted>`) |

## Installation & Requirements

- Ruby `>= 3.3.0`
- Zero runtime dependencies (`capabilities: []`)

Add to your `Gemfile`:

```ruby
gem "coding_adventures_canonical_cbor"
```

## Usage

```ruby
require "coding_adventures_canonical_cbor"

# Encode canonical values
val = CodingAdventures::CanonicalCbor::CborMap.new([
  CodingAdventures::CanonicalCbor::CborMapEntry.new(
    CodingAdventures::CanonicalCbor::CborText.new("id"),
    CodingAdventures::CanonicalCbor::CborUnsigned.new(42)
  ),
  CodingAdventures::CanonicalCbor::CborMapEntry.new(
    CodingAdventures::CanonicalCbor::CborText.new("active"),
    CodingAdventures::CanonicalCbor::CborBool.new(true)
  )
])

wire = CodingAdventures::CanonicalCbor.encode_checked(val)

# Decode wire octets
decoded = CodingAdventures::CanonicalCbor.decode(wire)

# Extract half-open byte spans for zero-copy signature verification
result = CodingAdventures::CanonicalCbor.decode_map_spans(wire)
if result.map?
  result.entries.each do |entry|
    span = entry.value_span
    entry_bytes = wire.byteslice(span.start, span.length)
    # entry_bytes is guaranteed equal to encode_checked(entry.value)
  end
end
```

## Security & Resilience

This implementation features defenses against 7 distinct vulnerability classes:

1. **CVE-1: Integer Wraparound & Bignum Boundaries**: Ruby integers are checked against `[0, MAX_UINT64]` bounds to prevent overflow.
2. **CVE-2: Pre-allocation OOM Bombs**: Declared collection lengths are validated against remaining stream bytes before allocation.
3. **CVE-3: Recursion Stack Exhaustion**: Enforces a strict maximum nesting depth of 128 during both encode and decode.
4. **CVE-4: Canonical Key Sorting & Signature Bypass**: Map keys must be strictly ordered by byte length first, then lexicographically.
5. **CVE-5: UTF-8 Smuggling & Encoding Confusion**: Rejects non-UTF-8 encodings (e.g. `ASCII-8BIT`), overlong sequences, and surrogate codepoints (`CESU-8`).
6. **CVE-6: Mid-Map Atomic Rollback**: `encode_into_checked` guarantees that the destination buffer remains completely untouched if serialization fails halfway through a map.
7. **CVE-7: Secret Payload Leakage**: Exceptions use fixed, closed diagnostics starting with `canonical-cbor: ` with zero reflection of user payload bytes; `SpannedMapEntry#inspect` redacts keys and values with `<redacted>`.

## Development & Testing

```bash
# Run unit tests and conformance suite with SimpleCov
bundle exec rake test

# Run style checks
bundle exec standardrb lib/ test/
```
