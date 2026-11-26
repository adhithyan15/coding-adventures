# coding_adventures_blake2b (Dart)

BLAKE2b cryptographic hash function (RFC 7693), from scratch. The
Dart lane of the Rust `blake2b` crate (spec `HF06-blake2b.md`).

```dart
import 'package:coding_adventures_blake2b/blake2b.dart';

print(blake2bHex('abc'.codeUnits));
// ba80a53f981c4d0...

final h = Blake2bHasher(const Blake2bOptions(digestSize: 32));
h.update([1, 2, 3]).update([4, 5]);
print(h.hexDigest());
```

## API

- `blake2b(data, [options])` / `blake2bHex(data, [options])` —
  one-shot hashing to bytes or lowercase hex.
- `Blake2bHasher([options])` — streaming hasher: `update` (returns
  `this` for chaining), non-destructive `digest`/`hexDigest`, and
  independent `copy`.
- `Blake2bOptions({digestSize, key, salt, personal})` — 1..64-byte
  digests, 0..64-byte keys, 16-byte-or-empty salt/personalization.
  Sequential mode only, like the reference.
- Errors: [Blake2bError] with [InvalidDigestSize]/[KeyTooLong]/
  [InvalidSaltLength]/[InvalidPersonalLength] subtypes carrying the
  offending value and the reference's exact messages; thrown (Dart
  has no `Result` channel to preserve).

Dart integers are signed 64-bit, so literals above 2^63-1 do not
compile. The lane-private `_U64` type stores high/low 32-bit halves
(the same shape the in-lane sha512 package uses) with wrapping add,
XOR, rotate-right, and little-endian conversion; the compression
function is then a direct transliteration of the reference.

## Validation

- **Unit** (`test/blake2b_test.dart`): mirrors all 23 reference
  tests case-for-case (hashlib vectors, block boundaries, digest
  sizes, keyed variants, salt/personal, streaming, validation,
  hex), plus builder defaults, chaining, omitted options, and error
  value semantics. 28 tests.
- **Differential** (`test/hashlib_vectors_test.dart`): 200 seeded
  CPython-hashlib vectors over boundary-heavy lengths and option
  shapes, one-shot and streamed. Regenerate with
  `python3 test/generate_hashlib_vectors.py`.
- **Property** (`test/properties_test.dart`): chunking equivalence,
  copy independence + idempotence, parameter avalanche (including
  the non-prefix truncated digest), exact option boundaries,
  empty-is-absent, and digest lengths — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile messages, chunkings, and option shapes;
  only exact [Blake2bError] escapes. Ad-hoc 20,000-iteration runs
  over two more seeds: zero crashes.
- **Coverage**: 100% lines and functions.
