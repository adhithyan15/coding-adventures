# coding_adventures_aes_modes (Dart)

AES modes of operation (ECB/CBC/CTR/GCM). The Dart lane of the
Rust `aes-modes` crate.

```dart
import 'package:coding_adventures_aes_modes/aes_modes.dart';

final (ct, tag) = gcmEncrypt(plain, key, iv, aad);
final back = gcmDecrypt(ct, key, iv, aad, tag);
```

## API

- `ecbEncrypt` / `ecbDecrypt` (INSECURE — educational only),
  `cbcEncrypt` / `cbcDecrypt`, `ctrEncrypt` / `ctrDecrypt`,
  `gcmEncrypt` (returns `(ciphertext, tag)`) / `gcmDecrypt`,
  `pkcs7Pad` / `pkcs7Unpad`.
- `ghashMultiplyFixed` / `ghashFixed` — GF(2^128) multiply and
  GHASH into caller-owned output, with mask-selected
  branch-free-coded decisions.
- `aes128GcmEncryptFixed` / `aes128GcmDecryptFixed` —
  fixed-memory AES-128-GCM (RFC 5116 96-bit nonces) with
  complete preflight validation and wipe-on-drop scratch.
- Errors: [AesModesError] with eight subtypes carrying the
  reference's exact messages (thrown); [GhashInputTooLong];
  [FixedAes128GcmError] with four redacted subtypes; key
  failures propagate from the AES lane unchanged.
  Fixed-size violations the reference cannot express (blocks,
  tags, nonces, GHASH buffers) throw [ArgumentError], per lane
  convention.

The fixed paths mirror the reference's hygiene via the in-lane
zeroize package (explicit `dispose()` in `finally` blocks);
the zeroize lane docs' platform limits apply unchanged. Dart
offers no optimizer barrier like the reference's `black_box`,
so the mask discipline is best-effort rather than verified.

## Validation

- **Unit** (`test/aes_modes_test.dart`): mirrors all 35
  reference tests (PKCS#7, ECB/CBC/CTR vectors and round-trips,
  GHASH incl. the branching reference model, fixed-GCM
  vectors and failure atomicity, compat-GCM tamper rejection),
  plus decrypt-side widths, fixed-buffer lengths, and error
  values. 39 tests.
- **Differential** (`test/node_vectors_test.dart`): 120 seeded
  Node.js-OpenSSL vectors (30 per mode) with a
  fixture-diversity guard. Regenerate with
  `node test/generate_node_vectors.mjs`.
- **Property** (`test/properties_test.dart`): padding
  round-trips, round-trips on every mode and key size,
  fixed-vs-compat agreement, tamper rejection, IV/nonce
  sensitivity, and the validation matrix — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile (mode, key, IV, AAD, plaintext)
  decodings; only mode, key-length, GHASH, fixed-GCM, and
  argument errors escape. Ad-hoc 20,000-iteration runs over two
  more seeds: zero crashes.
- **Coverage**: 100% lines and functions.
