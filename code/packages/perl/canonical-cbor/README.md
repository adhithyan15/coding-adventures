# CodingAdventures::CanonicalCbor (Perl)

Deterministic canonical CBOR codec implementing RFC 8949 §4.2.3 Length-First Map Key Ordering (CBR01 profile).

## Specification & Conformance

This package implements the **CBR01** specification for deterministic CBOR:

| Feature | RFC 8949 Section | Implementation Guarantee |
|---|---|---|
| Map Key Ordering | §4.2.3 | Length-first sorting: shorter serialized keys precede longer ones; ties broken lexicographically by byte values |
| Minimal Argument Encoding | §4.2.1 | Smallest valid integer/length representation strictly required; non-minimal forms rejected (`non-minimal-integer`) |
| Unsigned 64-Bit Boundaries | §3.1 / §4.2.1 | Full unsigned 64-bit integer space (`0` through `18446744073709551615`) verified via `B::svref_2object` flags (`SVf_IOK && !SVf_NOK`) and `pack('Q>')`/`unpack('Q>')` |
| Reject Floats | Profile rule | Floating-point numbers (NVs) rejected at constructor boundary and wire decode (`float-not-supported` / `unsupported-simple`) |
| Reject Indefinite Lengths | §3.2.1 | Indefinite-length bytes, strings, arrays, maps, and break stop codes rejected (`indefinite`) |
| Strict UTF-8 Validation | §3.1 | Text validated via strict `Encode::decode('UTF-8', ..., FB_CROAK \| LEAVE_SRC)`; surrogates (U+D800..U+DFFF) and overlong forms rejected (`invalid-utf8`) |
| Raw Bytes Integrity | Profile rule | Raw bytes validated via `utf8::downgrade($copy, 1)` to eliminate wide-char byte confusion |
| Nesting Limit | Profile rule | Hard recursion depth limit of 128 container/tag levels (`too-deep` / `encode-too-deep`) |
| Encoded Size Limit | Profile rule | Hard encoded byte ceiling of 1,048,576 bytes (1 MiB) (`encode-too-large`) |
| Pre-Allocation Defense | Security rule | Bounds check against remaining unread buffer bytes prevents hostile pre-allocation DoS (`length-too-large`) |

## Module Layout

- `CodingAdventures::CanonicalCbor`: Top-level facade exporting constructors, `encode_checked`, `encode_into_checked`, `decode`, and `decode_map_spans`.
- `CodingAdventures::CanonicalCbor::Constants`: Centralized zero-magic-constant module declaring protocol constants and wire byte markers.
- `CodingAdventures::CanonicalCbor::Messages`: Closed payload-blind diagnostic messages (Standing Rule #20) and `$REDACTED_PLACEHOLDER = '<redacted>'`.
- `CodingAdventures::CanonicalCbor::Types`: Inside-out unforgeable algebraic value models (`CborUnsigned`, `CborNegative`, `CborBytes`, `CborText`, `CborArray`, `CborMapEntry`, `CborMap`, `CborTag`, `CborBool`, `CborNull`), `Span`, `SpannedMapEntry`, `DecodeMapSpansResult`, and `CborError`. Objects use lexical refaddr registry encapsulation to prevent forgery or state tampering.
- `CodingAdventures::CanonicalCbor::Codec`: Deterministic encoder and zero-allocation cursor decoder with atomic buffer rollback.

## 7-Class Perl CVE Resilience Showcase

1. **CVE-1: 64-Bit Integer Wraparound and NV Float Precision Trap**: Perl numbers can silently coerce between integers (IV/UV) and floating-point doubles (NV). A double cannot represent integers above $2^{53}$ without precision loss. This package verifies integer provenance using `B::svref_2object` ensuring `SVf_IOK` is set, `SVf_NOK` is absent, and the value is non-negative, supporting full 64-bit boundaries (`0`, `2^32-1`, `2^32`, `2^64-1`).
2. **CVE-2: Pre-Allocation OOM Bomb Defense**: Declared byte counts, text lengths, array items, and map entries are checked against remaining buffer capacity via `_check_length` before any memory allocation occurs.
3. **CVE-3: Deep Nesting Stack Exhaustion**: Enforces an exact nesting cap of 128 levels across arrays, maps, and tags during both encode and decode. Depth 128 succeeds; depth 129 fails immediately with `too-deep` or `encode-too-deep`.
4. **CVE-4: Canonical Length-First Key Ordering & Duplicate Key Defense**: Map keys are sorted strictly by encoded byte length, then lexicographical byte comparison (`cmp`). Duplicate keys are rejected at encode (`duplicate-map-key`) and decode (`non-canonical-map-order`).
5. **CVE-5: Strict UTF-8 vs Surrogate Smuggling & Wide-Char Byte Confusion**: Strict `Encode::decode('UTF-8', ..., FB_CROAK | LEAVE_SRC)` rejects surrogate halves (U+D800..U+DFFF) and non-shortest UTF-8 encodings. `utf8::downgrade($str, 1)` verifies byte strings have no wide characters.
6. **CVE-6: Trailing-Byte Smuggling & Bless/Tie TOCTOU Defense**: Trailing bytes after a root decoded value trigger `trailing-bytes`. Inside-out lexical registries reject forged blessed references, and tied variables are detected and rejected (`unsupported-simple`).
7. **CVE-7: Secret Payload Reflection Leak Defense & Redaction**: 100% of error diagnostics are static fixed strings with zero user payload reflection. `SpannedMapEntry` redacts keys and values with `<redacted>` during stringification.

## Usage Example

```perl
use CodingAdventures::CanonicalCbor qw(:all);

# Deterministic encoding:
my $map = CborMap([
    CborMapEntry(CborText("key"), CborUnsigned(42)),
    CborMapEntry(CborText("a"), CborBool(1)),
]);
my $wire = encode_checked($map);

# Deterministic decoding:
my $decoded = decode($wire);
print "Matches!\n" if $decoded == $map;

# Spanned map decoding for signature validation:
my $res = decode_map_spans($wire);
if ($res->is_map) {
    for my $entry (@{$res->entries}) {
        my $span = $entry->value_span;
        my $slice = substr($wire, $span->start, $span->length);
        die "Invariant violation" unless $slice eq encode_checked($entry->value);
    }
}
```

## Running Tests & Coverage

```bash
prove -l -v t/
```

Runs the full test suite including:
- `t/00-load.t`: Module compilation and symbol export validation.
- `t/01-corpus.t`: Full 55/55 test vectors from the shared CBR01 specification corpus.
- `t/02-canonical-cbor.t`: 7-class Perl CVE showcase, property-based PRNG roundtripping, and 10,000-iteration adversarial fuzzing.
- `t/99-coverage.t`: Zero-dependency, offline line-coverage profiler asserting >= 95% line coverage.
