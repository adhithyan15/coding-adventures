## 0.1.0

- Initial Dart lane of the chacha20-poly1305 identity:
  ChaCha20 stream (compat + fixed), Poly1305 (exact-BigInt +
  26-bit-limb, cross-checked), ChaCha20-Poly1305 AEAD (compat +
  fixed with wipe-on-drop scratch), and H/XChaCha20; the
  `FixedChaCha20Error`/`FixedAeadError` types carry the
  reference's exact messages.
- Tests mirror the reference suite (35 unit tests incl. the RFC
  vectors), add 5 extras, 3 differential tests over 90 seeded
  Node.js-OpenSSL vectors, 6 seeded property tests over the
  in-lane `pbt` package, and a 2,000-iteration differential fuzz
  campaign over the in-lane `fuzz` package; 100% line/function
  coverage.
