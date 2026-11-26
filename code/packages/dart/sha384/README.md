# coding_adventures_sha384 (Dart)

Fixed-memory SHA-384 from FIPS 180-4. The Dart lane of the Rust
`sha384` crate.

```dart
import 'package:coding_adventures_sha384/sha384.dart';

print(sha384Hex('abc'.codeUnits));
// cb00753f45a35e8b...

final h = Sha384Hasher();
h.update([1, 2, 3]);
print(h.digest().length); // 48
h.dispose();
```

## API

- `sha384(data)` / `sha384Hex(data)` — one-shot 48-byte digest.
- `Sha384Hasher()` — streaming hasher: `update` (throws
  [Sha384MessageTooLong] past the 2^127-byte limit, checked before
  any state changes), non-destructive `digest`, explicit `dispose()`
  (use afterwards throws [StateError] instead of hashing scrubbed
  state).
- Memory hygiene mirrors the reference via the in-lane zeroize
  package: consumed buffers and digest temporaries are wiped, and
  `dispose()` scrubs all retained state. Word temporaries are
  immutable values with no wipe equivalent — see the zeroize lane
  docs for the exact platform limits.

Dart integers are signed 64-bit, so literals above 2^63-1 do not
compile. The lane-private `_U64` type stores high/low 32-bit halves
(the same shape the in-lane sha512 and blake2b packages use); the
compression function is a direct transliteration of the reference.

## Validation

- **Unit** (`test/sha384_test.dart`): mirrors all 9 reference tests
  (FIPS vectors incl. the padding-boundary and million-`a` cases,
  chunk partitions, non-destructive digests, empty updates,
  transactional length failure, width), plus dispose semantics,
  one-shot agreement, and error values. 12 tests.
- **Differential** (`test/hashlib_vectors_test.dart`): 200 seeded
  CPython-hashlib vectors over boundary-heavy lengths, one-shot and
  streamed. Regenerate with
  `python3 test/generate_hashlib_vectors.py`.
- **Property** (`test/properties_test.dart`): chunking equivalence,
  idempotence + continuation, avalanche, digest width, dispose
  locking, and counter-boundary transactions — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile messages and chunkings; only exact errors
  escape. Ad-hoc 20,000-iteration runs over two more seeds: zero
  crashes.
- **Coverage**: 100% lines and functions.
