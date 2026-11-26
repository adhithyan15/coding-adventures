## 0.1.0

- Initial Dart lane of the sha384 identity: streaming `Sha384Hasher`
  with explicit `dispose()`, one-shot `sha384`/`sha384Hex`, and the
  `Sha384MessageTooLong` error; u64 arithmetic via a lane-local
  high/low-halves word type, memory hygiene via the in-lane zeroize
  package.
- Tests mirror the reference suite (9 unit tests), add 3 extras, 3
  differential tests over 200 seeded hashlib vectors, 6 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration differential fuzz campaign over the in-lane `fuzz`
  package; 100% line/function coverage.
