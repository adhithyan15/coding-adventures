# coding_adventures_ed25519 (Dart)

Ed25519 digital signatures (RFC 8032), implemented from scratch. The
Dart lane of the Rust `ed25519` crate (spec `ed25519.md`).

```dart
import 'package:coding_adventures_ed25519/ed25519.dart';

final (pub, sec) = generateKeypair(seed);
final sig = sign(message, sec);
print(verify(message, sig, pub)); // true
```

## API

- `generateKeypair(seed)` — 32-byte seed → (32-byte public key,
  64-byte secret key of seed || public key).
- `sign(message, secretKey)` — deterministic 64-byte signature.
- `verify(message, signature, publicKey)` — boolean verdict; never
  throws on invalid input.
- `isValidPublicKey(bytes)` — canonical, non-identity, prime-order
  check for external keys.
- `Ed25519PublicKey`/`Ed25519SecretKey`/`Ed25519Signature` —
  length-checked containers (`ArgumentError` on misshapen input;
  the reference takes fixed-size arrays, which have no
  unrepresentable lengths in Dart).

Field elements are exact [BigInt] integers reduced mod p — not the
reference's five-limb radix-2^51 layout with addition chains and
schoolbook division. Scalar arithmetic likewise reduces mod L
directly. Observable behavior (encodings, vectors, verdicts) is
identical; the limb machinery has no reason to exist here.

## Validation

- **Unit** (`test/ed25519_test.dart`): mirrors every reference test
  (field arithmetic, point operations, encoding round-trips, the
  four RFC 8032 vectors, verification edges, key generation), plus
  length validation and value semantics. 29 tests.
- **Differential** (`test/node_vectors_test.dart`): 100 seeded
  vectors from Node.js built-in Ed25519 (an independent
  implementation family): keygen, signatures, and cross-verified
  vectors. Regenerate with `node test/generate_node_vectors.mjs`.
- **Property** (`test/properties_test.dart`): sign/verify
  round-trips, single-bit tamper rejection, key formats, and
  cross-key isolation — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 500-iteration differential
  campaign (fewer than sibling lanes: each case performs several
  scalar multiplications) over hostile seeds, messages, and
  mutations; `verify` never throws. Ad-hoc 2,000-iteration runs
  over two more seeds: zero crashes.
- **Coverage**: 100% lines and functions.
