# coding_adventures_scrypt (Dart)

scrypt password hashing (RFC 7914). The Dart lane of the Rust
`scrypt` crate.

```dart
import 'package:coding_adventures_scrypt/scrypt.dart';

final dk = scrypt(
  'password'.codeUnits,
  'salt'.codeUnits,
  n: 16384,
  r: 8,
  p: 1,
  dkLen: 32,
);
```

## API

- `scrypt(password, salt, {n, r, p, dkLen})` — the derived key:
  PBKDF2 expansion, p ROMix passes over an `N * 128 * r`-byte
  table, PBKDF2 extraction. Empty passwords and salts are
  accepted.
- `scryptHex(...)` — lowercase hex convenience.
- Errors: [ScryptError] with eight subtypes (thrown — Dart has no
  `Result` channel). The reference enum carries no messages, so
  the texts are lane-defined; the variant set, trigger
  conditions, and check order mirror the reference exactly
  ([ScryptNTooLarge] before [ScryptInvalidN]).

ROMix keeps its V table in one flat buffer (same access pattern
as the reference's nested vectors). Bounds use [BigInt] where
the reference uses saturating/checked 64-bit arithmetic, since
Dart ints wrap silently. PBKDF2 failures map to
[ScryptHmacError].

## Validation

- **Unit** (`test/scrypt_test.dart`): mirrors all 55 reference
  tests (Salsa structure, RFC 7914 vectors, hex wrapper, output
  lengths, determinism, sensitivity, empty inputs, validation),
  plus negative counts, the PBKDF2-cap overflow path, exact
  error texts, value equality, and the Salsa length check. 43
  tests. The heavy RFC
  vectors and the N=2^20 boundary run in the default suite with
  generous timeouts.
- **Differential** (`test/scrypt_vectors_test.dart`): 48 seeded
  CPython-hashlib vectors over passwords, salts, cost
  parameters, and output lengths, with a fixture-diversity
  guard. Regenerate with
  `python3 test/generate_scrypt_vectors.py`.
- **Property** (`test/properties_test.dart`): determinism and
  lengths, input sensitivity, the validation matrix, and hex
  shape — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile (password, salt, N, r, p, dkLen)
  decodings; only [ScryptError] escapes. Ad-hoc 5,000-iteration
  runs over two more seeds: zero crashes.
- **Coverage**: 100% lines and functions.

## Reference divergence

The reference's `#[ignore]`d vector-3 expected string is a
transcription typo from byte 34 (proven: its own implementation
disagrees with its stored string, while this lane, hashlib, and
the reference implementation agree byte-for-byte). The lane pins
the correct value; the Rust fix is logged as a follow-up.
