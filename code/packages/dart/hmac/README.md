# coding_adventures_hmac (Dart)

HMAC message authentication (RFC 2104 / FIPS 198-1). The Dart lane of
the Rust `hmac` crate.

```dart
import 'package:coding_adventures_hmac/hmac.dart';

print(hmacSha256Hex('key'.codeUnits, 'Hi There'.codeUnits));
// df7fe41c... (64 hex chars)

// Fixed-memory SHA-384 over segments: no concatenated buffer.
final tag = hmacSha384Segments('key'.codeUnits, [
  'part one '.codeUnits,
  'part two'.codeUnits,
]);
print(verifyHmac(tag, tag)); // true
```

## API

- `hmac(hashFn, blockSize, key, message)` — generic higher-order
  core over any hash function.
- `hmacMd5` / `hmacSha1` / `hmacSha256` / `hmacSha384` /
  `hmacSha512` — named variants returning fixed-length tags; empty
  keys throw [HmacEmptyKey].
- `hmacSha256Segments` / `hmacSha384Segments` — fixed-memory paths
  over message segments; boundaries never affect the tag.
- `hmacMd5Hex` / `hmacSha1Hex` / `hmacSha256Hex` / `hmacSha384Hex` /
  `hmacSha512Hex` — lowercase hex conveniences (the reference
  panics on empty keys; Dart has no panics, so [HmacEmptyKey]
  propagates instead).
- `verifyHmac(expected, actual)` — constant-time tag comparison
  over the in-lane ct-compare primitive; mismatched lengths return
  `false` without scanning.
- Memory hygiene mirrors the reference via the in-lane zeroize
  package: key-derived intermediates live in `ZeroizingBytes` and
  are disposed in `finally` blocks (Dart has no destructors),
  nested inputs are built at exact capacity, and the generic core
  wipes the heap tag copy before returning the fixed-size tag. The
  Dart sha256 lane reports no length failures, so [HmacInputTooLong]
  is unreachable on the SHA-256 path; the sha384 lane's
  `Sha384MessageTooLong` maps to it on the SHA-384 path. The
  zeroize lane docs' platform limits apply unchanged.

## Validation

- **Unit** (`test/hmac_test.dart`): mirrors all 37 reference tests
  (RFC 4231/2202 vectors, fixed-vs-generic agreement across block
  boundaries, segmented equivalence, error categories, lengths,
  authentication properties), plus hex-variant error propagation,
  error equality, and generic key-shape equivalence. 40 tests.
- **Differential** (`test/hmac_vectors_test.dart`): 200 seeded
  CPython-hmac vectors (40 per variant) over boundary-heavy key and
  message lengths, plus segmented agreement. Regenerate with
  `python3 test/generate_hmac_vectors.py`.
- **Property** (`test/properties_test.dart`): exact tag lengths,
  determinism, key/message sensitivity with avalanche, generic
  agreement, random-split segments equivalence, verify symmetry,
  hex shape, and empty-key rejection on all 12 entry points —
  fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile (key, message, variant, splits) decodings;
  only [HmacEmptyKey] escapes. Ad-hoc 20,000-iteration runs over
  two more seeds: zero crashes.
- **Coverage**: 100% lines and functions.
