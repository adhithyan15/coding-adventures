# coding_adventures_hkdf (Dart)

HKDF key derivation (RFC 5869). The Dart lane of the Rust `hkdf`
crate.

```dart
import 'package:coding_adventures_hkdf/hkdf.dart';

final okm = hkdf(
  'salt'.codeUnits,
  'input-keying-material'.codeUnits,
  'app-context'.codeUnits,
  32,
  HashAlgorithm.sha256,
);
```

## API

- `hkdfExtract(salt, ikm, algorithm)` / `hkdfExpand(prk, info,
  length, algorithm)` / `hkdf(salt, ikm, info, length, algorithm)`
  — legacy extract-then-expand over [HashAlgorithm]
  (`sha256`/`sha512` HMAC, `blake2b` keyed mode with a 32-byte
  digest).
- `hkdfExtractFixed(salt, ikm, algorithm)` /
  `hkdfExpandFixed(prk, info, output)` — fixed-memory boundaries
  over [FixedHkdfHash] (`sha256`/`sha384`) for production secret
  handling; expand writes into caller-provided output.
- `HkdfPrk` — wipe-on-drop PRK owner (explicit `dispose()`; 48-byte
  backing, algorithm-bound prefix); `HkdfPrk.importExact`
  imports exact-length keys. `asBytes()` borrows a view — do not
  mutate or retain past `dispose()`.
- Errors: [HkdfError] with four subtypes carrying the reference's
  exact messages (thrown — Dart has no `Result` channel);
  [HkdfPrkImportError] on non-exact import lengths, with a
  payload-free message.
- Memory hygiene mirrors the reference via the in-lane zeroize
  package on the fixed paths (PRK storage, previous-block owner,
  temporary tags wiped after copying); the legacy path mirrors
  the reference exactly, which does not wipe. Length failures on
  the fixed path happen before caller output changes, and an
  unexpected HMAC failure wipes all caller output first. The
  zeroize lane docs' platform limits apply unchanged.

## Validation

- **Unit** (`test/hkdf_test.dart`): mirrors all 45 reference tests
  (fixed extract/expand incl. import and redaction, RFC 5869
  vectors, edge cases, BLAKE2b structural tests), plus negative
  lengths, dispose locking, error values, hash lengths, and the
  empty-PRK fallback. 50 tests.
- **Differential** (`test/node_vectors_test.dart`): 60 seeded
  Node.js-OpenSSL vectors (30 per digest) over boundary-heavy
  inputs, including the full 255-block chain per digest, with a
  fixture-diversity guard. Regenerate with
  `node test/generate_node_vectors.mjs`.
- **Property** (`test/properties_test.dart`): round-trips,
  determinism, input sensitivity, fixed-vs-legacy agreement,
  fixed-SHA-384 behavior, the length matrix, default-salt
  equivalence, and import round-trips — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile (algorithm, salt, IKM, info, length)
  decodings; only [HkdfError] and [HkdfPrkImportError] escape.
  Ad-hoc 20,000-iteration runs over two more seeds: zero crashes.
- **Coverage**: 100% lines; functions 39/41 (the two synthetic
  enum constructors are uncoverable, as in the RWS lane).
