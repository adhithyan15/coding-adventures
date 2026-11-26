## 0.1.0

- Initial Dart lane of the blake2b identity: streaming
  `Blake2bHasher` with `Blake2bOptions` (digest size, key, salt,
  personalization), one-shot `blake2b`/`blake2bHex`, and the
  four-variant `Blake2bError` type; u64 arithmetic via a lane-local
  high/low-halves word type.
- Tests mirror the reference suite (23 unit tests), add 5 extras, 3
  differential tests over 200 seeded hashlib vectors, 6 seeded
  property tests over the in-lane `pbt` package, and a
  2,000-iteration differential fuzz campaign over the in-lane `fuzz`
  package; 100% line/function coverage.
