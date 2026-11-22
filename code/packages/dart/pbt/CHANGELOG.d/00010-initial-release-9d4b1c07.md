## 0.1.0

- Initial Dart PBT lane (VER05b): `SplitMix64`/`fnv1a64`/`subSeed` over
  exact `BigInt` state, `Gen<T>` algebra (bool, u8, u64, integer, bytes,
  list, constant, oneOf, tuple2/3, map, filter, flatMap), `shrinkInt` /
  `shrinkList` / `shrinkBytes` / `simulateReplay`, and `checkProperty` /
  `assertProperty` with the closed four-id failure vocabulary and CI
  repro-digest redaction.
- Replays 25/25 pbt-v1 corpus cases with exact outputs, minimals, and step
  counts; zero runtime dependencies; 95% branch+function coverage gate.
