## 0.1.0

- Initial Dart lane of the zeroize identity: `zeroizeBytes` (+
  `Uint8List.zeroize()` extension) and the `ZeroizingBytes`
  owning wrapper with explicit idempotent `dispose()`,
  `takeBytes()` opt-out, and post-dispose `StateError`s. No
  string/int impls (immutable values cannot be wiped) and no
  automatic drop (Dart has no destructors); the library docs state
  the exact platform limits.
- Tests mirror the portable reference suite (12 unit tests), add 5
  seeded property tests over the in-lane `pbt` package and a
  2,000-iteration op-script fuzz campaign over the in-lane `fuzz`
  package; 100% line/function coverage.
