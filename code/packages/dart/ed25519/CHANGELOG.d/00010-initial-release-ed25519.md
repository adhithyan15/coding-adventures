## 0.1.0

- Initial Dart lane of the ed25519 identity: `generateKeypair`,
  `sign`, `verify`, and `isValidPublicKey` over exact BigInt field
  arithmetic (instead of the reference's 51-bit limbs), with
  length-checked key/signature containers and in-lane SHA-512 for
  all hashing.
- Tests mirror the reference suite (29 unit tests incl. the four
  RFC 8032 vectors), add 4 differential tests over 100 seeded
  Node.js vectors, 4 seeded property tests over the in-lane `pbt`
  package, and a 500-iteration differential fuzz campaign over the
  in-lane `fuzz` package; 100% line/function coverage.
