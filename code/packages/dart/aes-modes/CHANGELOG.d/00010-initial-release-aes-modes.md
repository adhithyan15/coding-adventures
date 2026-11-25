## 0.1.0

- Initial Dart lane of the aes-modes identity: PKCS#7 plus
  ECB/CBC/CTR/GCM over the in-lane AES block cipher, mask-coded
  `ghashMultiplyFixed`/`ghashFixed`, and fixed-memory
  AES-128-GCM with wipe-on-drop scratch; the eight-variant
  `AesModesError`, `GhashInputTooLong`, and the four-variant
  `FixedAes128GcmError` carry the reference's exact messages.
- Tests mirror the reference suite (35 unit tests incl. the NIST
  vectors), add 4 extras, 3 differential tests over 120 seeded
  Node.js-OpenSSL vectors, 6 seeded property tests over the
  in-lane `pbt` package, and a 2,000-iteration differential fuzz
  campaign over the in-lane `fuzz` package; 100% line/function
  coverage.
