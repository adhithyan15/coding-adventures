## 0.1.0

- Initial Dart lane of the scrypt identity: `scrypt`/
  `scryptHex` (PBKDF2 expansion, flat-table ROMix, PBKDF2
  extraction) with the Salsa20/8 core in a public-in-source
  library and the eight-variant `ScryptError` type; bounds use
  BigInt where the reference uses saturating/checked 64-bit
  arithmetic.
- Tests mirror the reference suite (55 unit tests incl. the RFC
  7914 vectors), add 5 extras, 2 differential tests over 48
  seeded CPython-hashlib vectors, 4 seeded property tests over
  the in-lane `pbt` package, and a 2,000-iteration differential
  fuzz campaign over the in-lane `fuzz` package; 100%
  line/function coverage.
