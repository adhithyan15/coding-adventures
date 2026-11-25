# coding_adventures_pbkdf2 (Dart)

PBKDF2 password hashing (RFC 8018). The Dart lane of the Rust
`pbkdf2` crate.

```dart
import 'package:coding_adventures_pbkdf2/pbkdf2.dart';

final dk = pbkdf2HmacSha256(
  'password'.codeUnits,
  'salt'.codeUnits,
  iterations: 600000,
  keyLength: 32,
);
```

## API

- `pbkdf2HmacSha1` / `pbkdf2HmacSha256` / `pbkdf2HmacSha512`
  (`password`, `salt`, `{iterations, keyLength,
  allowEmptyPassword}`) — the derived key bytes.
- `pbkdf2HmacSha1Hex` / `pbkdf2HmacSha256Hex` /
  `pbkdf2HmacSha512Hex` — lowercase hex conveniences.
- Errors: [Pbkdf2Error] with five subtypes carrying the
  reference's exact messages (thrown — Dart has no `Result`
  channel). An empty password is permitted only with
  [allowEmptyPassword], which routes to the guard-free generic
  HMAC core (RFC-compliant, e.g. for scrypt's vectors).

The PRF is the in-lane HMAC (named variants, or the generic core
for the empty-password path); failures map to [Pbkdf2PrfError]
defensively. Key length is capped at 2^20 (1 MiB), far below the
RFC's hard cap, against memory-exhaustion DoS.

## Validation

- **Unit** (`test/pbkdf2_test.dart`): mirrors all 21 reference
  tests (RFC 6070/7914 vectors, SHA-512 sanity, hex variants,
  validation, binding), plus SHA-512 hex, the 2^20 cap,
  negative counts, exact error texts, and value equality. 26
  tests.
- **Differential** (`test/pbkdf2_vectors_test.dart`): 90 seeded
  CPython-hashlib vectors (30 per PRF) over passwords, salts,
  iteration counts, and output lengths, with a
  fixture-diversity guard. Regenerate with
  `python3 test/generate_pbkdf2_vectors.py`.
- **Property** (`test/properties_test.dart`): determinism, input
  sensitivity, truncation consistency, the allow-flag contract,
  and the validation matrix — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile (variant, password, salt, iterations,
  length, flag) decodings; only [Pbkdf2Error] escapes.
  Ad-hoc 20,000-iteration runs over two more seeds: zero crashes.
- **Coverage**: 100% lines and functions.
