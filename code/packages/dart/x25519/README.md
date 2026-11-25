# coding_adventures_x25519 (Dart)

X25519 key agreement on Curve25519 (RFC 7748). The Dart lane of the
Rust `x25519` crate.

```dart
import 'package:coding_adventures_x25519/x25519.dart';

final alicePublic = x25519Base(alicePrivate); // 32 bytes
final shared = x25519(alicePrivate, bobPublic); // 32 bytes
```

## API

- `x25519(scalar, uCoordinate)` — the 32-byte shared secret.
- `x25519Base(scalar)` — scalar times the base point (u = 9):
  public-key derivation. `generateKeypair(privateKey)` is the same
  function under a clearer name.
- `clampScalar(scalar)` — RFC 7748 §5 clamping (also applied
  internally by [x25519], so callers never need it first).
- `basePoint` — the 32-byte base-point encoding.
- Errors: [X25519LowOrderPoint] with the reference's exact message
  when the ladder yields all zeros; [ArgumentError] unless inputs
  are exactly 32 bytes (fixed-size arrays have no
  unrepresentable lengths in Dart).

Field elements are exact [BigInt] integers reduced mod p (see
`lib/src/field.dart`, following the ed25519 lane) instead of the
reference's 51-bit limbs: encodings, ladder outputs, and shared
secrets are identical, while limb packing, carry chains, and the
inversion addition chain have no reason to exist here. The
conditional swap keeps the reference's branch-free mask structure,
but like the ed25519 lane this port makes no full constant-time
claim — [BigInt] operations are variable-time regardless.

## Validation

- **Unit** (`test/x25519_test.dart`): mirrors all 21 reference
  tests (field arithmetic, cswap, clamping, RFC 7748 vectors,
  Alice/Bob exchange, keypair alias, 1- and 1000-iteration
  chains, base point, codec round-trip), plus length checks, the
  low-order error, and value equality. 24 tests.
- **Differential** (`test/node_vectors_test.dart`): 100 seeded
  Node.js-OpenSSL vectors (both keypairs plus the shared secret,
  oracle-computed in both directions), with a fixture-diversity
  guard. Regenerate with
  `node test/generate_node_vectors.mjs`.
- **Property** (`test/properties_test.dart`): DH agreement,
  determinism, clamp idempotence and bit shape, scalar and
  u-coordinate sensitivity, and keypair aliasing — fixed seeds.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration differential
  campaign over hostile scalars and u-coordinates; only
  [X25519LowOrderPoint] escapes. Ad-hoc 2,000-iteration runs over
  two more seeds: zero crashes.
- **Coverage**: 100% lines and functions.
