# coding_adventures_argon2i (Dart)

Argon2i data-independent memory-hard password hashing (RFC 9106).
The Dart lane of the Rust `argon2i` crate (spec `KD03-argon2.md`).

```dart
import 'package:coding_adventures_argon2i/argon2i.dart';

final tag = argon2i(
  'password'.codeUnits,
  'saltsalt'.codeUnits,
  timeCost: 3,
  memoryCost: 32,
  parallelism: 4,
  tagLength: 32,
);
```

## API

- `argon2i(password, salt, {timeCost, memoryCost, parallelism,
  tagLength, opts})` — the tag bytes; `argon2iHex(...)` renders
  lowercase hex.
- `Argon2Options({key, associatedData, version})` — optional secret
  key, associated data, and version override (only 0x13 accepted).
- Errors: [Argon2Error] with twelve subtypes carrying the offending
  values and the reference's exact messages; thrown (Dart has no
  `Result` channel to preserve).

Block words are exact [BigInt] integers masked to 64 bits — the
reference's `2·a·b` term overflows 64 bits by design, and Dart has
no wrapping multiply, so halves arithmetic cannot express it
directly. Observable behavior is identical; variable-length hashing
delegates to the in-lane blake2b package.

## Validation

- **Unit** (`test/argon2i_test.dart`): mirrors all 16 reference
  tests (the RFC 9106 §5.2 vector, hex agreement, the validation
  matrix, determinism, binding, tag lengths, multi-lane,
  multi-pass), plus empty passwords, exact error texts/values,
  and the 32-bit tag-length cap. 22 tests.
- **Differential** (`test/cli_vectors_test.dart`): 60 seeded vectors
  from the reference C `argon2` CLI in Argon2i mode over passwords,
  salts, and cost sweeps. Regenerate with
  `python3 test/generate_argon2_vectors.py` (requires the CLI).
- **Property** (`test/properties_test.dart`): determinism,
  input/key/AD binding, cost-parameter binding, tag lengths, and
  the validation matrix — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 300-iteration differential
  campaign (fewer than sibling lanes: up to three real hashes per
  case) over hostile inputs and parameters; only exact
  [Argon2Error] escapes. Ad-hoc 2,000-iteration runs over two more
  seeds: zero crashes.
- **Coverage**: 99%+ lines and functions gate (residue: the two
  provably-unreachable defensive blake2b-wrap lines).
