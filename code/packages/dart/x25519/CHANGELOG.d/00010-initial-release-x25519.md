## 0.1.0

- Initial Dart lane of the x25519 identity: `x25519`/
  `x25519Base`/`generateKeypair` over exact BigInt field elements
  (instead of 51-bit limbs), with public `clampScalar`, the
  `basePoint` encoding, and the `X25519LowOrderPoint` error
  carrying the reference's exact message; wrong-length inputs
  throw `ArgumentError`.
- Tests mirror the reference suite (21 unit tests incl. the RFC
  7748 vectors and the 1000-iteration chain), add 3 extras, 3
  differential tests over 100 seeded Node.js-OpenSSL vectors, 6
  seeded property tests over the in-lane `pbt` package, and a
  2,000-iteration differential fuzz campaign over the in-lane
  `fuzz` package; 100% line/function coverage.
