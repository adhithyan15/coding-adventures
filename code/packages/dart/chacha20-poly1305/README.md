# coding_adventures_chacha20_poly1305 (Dart)

ChaCha20-Poly1305 AEAD (RFC 8439) and XChaCha20-Poly1305. The Dart
lane of the Rust `chacha20-poly1305` crate.

```dart
import 'package:coding_adventures_chacha20_poly1305/chacha20_poly1305.dart';

final (ct, tag) = aeadEncrypt(plain, key, nonce, aad);
final back = aeadDecrypt(ct, key, nonce, aad, tag); // null on failure
```

## API

- `chacha20Encrypt(plaintext, key, nonce, counter)` /
  `chacha20XorFixed(input, key, nonce, counter, output)` —
  stream cipher, compat and fixed-memory.
- `poly1305Mac(message, key)` / `poly1305MacFixed(message, key,
  output)` — one-time MAC, exact-BigInt and 26-bit-limb
  implementations cross-checked against each other.
- `aeadEncrypt` (returns `(ciphertext, tag)`) / `aeadDecrypt`
  (returns plaintext or null), `aeadEncryptFixed` /
  `aeadDecryptFixed` into caller-owned buffers.
- `hchacha20Subkey`, `xchacha20Encrypt`,
  `xchacha20Poly1305AeadEncrypt` / `...Decrypt` (null on
  failure).
- Errors: [FixedChaCha20Error] and [FixedAeadError] with the
  reference's exact messages (thrown); fixed-size violations
  throw [ArgumentError] per lane convention.

The fixed paths mirror the reference's hygiene via the in-lane
zeroize package (explicit `dispose()` in `finally` blocks).
Dart offers no optimizer barrier like the reference's
`black_box`, so the mask discipline is best-effort; there is no
`export_name` equivalent either.

## Validation

- **Unit** (`test/chacha20_poly1305_test.dart`): mirrors all 35
  reference tests (ChaCha20/Poly1305 vectors, fixed boundaries,
  AEAD vectors and tamper rejection, H/XChaCha20 vectors,
  length preflights), plus a fixed-AEAD RFC vector, fixed-size
  checks, error values, counter validation, and the pinned
  compat wrap. 40 tests.
- **Differential** (`test/node_vectors_test.dart`): 90 seeded
  Node.js-OpenSSL AEAD vectors with a fixture-diversity guard.
  Regenerate with `node test/generate_node_vectors.mjs`.
- **Property** (`test/properties_test.dart`): stream and AEAD
  round-trips, fixed agreement, tamper rejection, nonce/key
  sensitivity, XChaCha20 round-trips, and the validation matrix
  — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile (operation, key, nonce, AAD, message)
  decodings; only fixed-ChaCha20, fixed-AEAD, and argument
  errors escape. Ad-hoc 20,000-iteration runs over two more
  seeds: zero crashes.
- **Coverage**: 100% lines and functions.
