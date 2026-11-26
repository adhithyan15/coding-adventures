## 0.1.0

- Initial Dart lane of the pbkdf2 identity: `pbkdf2HmacSha1`/
  `pbkdf2HmacSha256`/`pbkdf2HmacSha512` over the in-lane HMAC
  (named variants, or the guard-free generic core for the
  explicitly allowed empty-password path), three hex
  conveniences, and the five-variant `Pbkdf2Error` type with the
  reference's exact messages.
- Tests mirror the reference suite (21 unit tests incl. the RFC
  6070/7914 vectors), add 5 extras, 2 differential tests over 90
  seeded CPython-hashlib vectors, 5 seeded property tests over
  the in-lane `pbt` package, and a 2,000-iteration differential
  fuzz campaign over the in-lane `fuzz` package; 100%
  line/function coverage.
