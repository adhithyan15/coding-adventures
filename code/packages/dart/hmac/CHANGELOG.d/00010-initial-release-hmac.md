## 0.1.0

- Initial Dart lane of the hmac identity: generic higher-order
  `hmac` over any in-lane hash, five named variants (`hmacMd5`,
  `hmacSha1`, fixed-memory `hmacSha256`/`hmacSha384` with
  `...Segments` forms, `hmacSha512`), five hex conveniences,
  constant-time `verifyHmac`, and the `HmacEmptyKey` /
  `HmacInputTooLong` errors with payload-free reference messages;
  memory hygiene via the in-lane zeroize package (explicit
  `dispose()` in `finally` blocks, exact-capacity nested inputs).
- Tests mirror the reference suite (37 unit tests), add 3 extras, 3
  differential tests over 200 seeded CPython-hmac vectors, 9 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration differential fuzz campaign over the in-lane `fuzz`
  package; 100% line/function coverage.
