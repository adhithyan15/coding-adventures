# coding_adventures_aes (Dart)

AES block cipher (FIPS 197). The Dart lane of the Rust `aes`
crate.

```dart
import 'package:coding_adventures_aes/aes.dart';

final ct = encryptBlock(plain16, key16); // AES-128
final back = decryptBlock(ct, key16);
```

## API

- `encryptBlock(block, key)` / `decryptBlock(block, key)` —
  single-block AES with 16-, 24-, or 32-byte keys (10/12/14
  rounds).
- `expandKey(key)` — key schedule: 11/13/15 round keys of 16
  bytes, flat column-major.
- `sbox()` / `invSbox()` — the 256-byte tables, returned by copy
  so callers cannot mutate the shared state.
- `aes128EncryptBlockFixed(plaintext, key, output)` — AES-128
  into caller-owned output with fixed memory, wipe-on-drop
  scratch, and a table-free S-box circuit (no secret-indexed
  lookups).
- Errors: [AesInvalidKeyLength] with the reference's exact
  message; [ArgumentError] unless blocks are exactly 16 bytes
  (the reference takes fixed-size arrays, which admit no other
  lengths).

State is a flat 16-byte column-major buffer — the reference's
4×4 matrix in arrival order. The S-box tables are built at
startup from lane-local 0x11B field arithmetic (the in-lane
gf256 package only implements 0x11D). The fixed path mirrors
the reference's hygiene via the in-lane zeroize package; the
zeroize lane docs' platform limits apply unchanged.

## Validation

- **Unit** (`test/aes_test.dart`): mirrors all 38 reference tests
  (FIPS vectors, fixed-path agreement, S-box properties, key
  schedule, round-trips, avalanche, errors), plus block-length
  checks, fixed-path lengths, and error values. 41 tests.
- **Differential** (`test/node_vectors_test.dart`): 90 seeded
  Node.js-OpenSSL AES-ECB vectors (30 per key size) with a
  fixture-diversity guard. Regenerate with
  `node test/generate_node_vectors.mjs`.
- **Property** (`test/properties_test.dart`): round-trips,
  fixed-vs-compat agreement, determinism and avalanche, key and
  plaintext sensitivity, and the validation matrix — fixed
  seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile (key size, key, block) decodings; only
  [AesInvalidKeyLength] and [ArgumentError] escape.
  Ad-hoc 20,000-iteration runs over two more seeds: zero crashes.
- **Coverage**: 100% lines and functions.
