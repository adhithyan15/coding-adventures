## 0.1.0

- Initial Dart lane of the argon2id identity: `argon2id`/
  `argon2idHex` over exact BigInt block words (the reference's
  overflowing `2·a·b` term has no halves equivalent), with
  `Argon2Options` and the twelve-variant `Argon2Error` type;
  variable-length hashing delegates to the in-lane blake2b package.
- Tests mirror the reference suite (16 unit tests incl. the RFC
  9106 §5.3 vector), add 4 extras, 2 differential tests over 60
  seeded reference-CLI vectors, 5 seeded property tests over the
  in-lane `pbt` package, and a 300-iteration differential fuzz
  campaign over the in-lane `fuzz` package; 99%+ coverage gate.
