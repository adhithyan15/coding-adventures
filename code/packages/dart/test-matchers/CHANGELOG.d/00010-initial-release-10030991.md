## 0.1.0

- Initial Dart lane of the test-matchers identity: `expect` with the full
  Jest matcher surface (`toBe`, `toEqual`, `toStrictEqual`, truthiness,
  numerics, `toBeCloseTo`, `toContain`, `toHaveLength`, `toMatch`,
  `toHaveProperty`, `toThrow`), `.not` negation, `resolves`/`rejects` async
  assertions, and the bit-exact comparison engine (`toValue`, `valuesEqual`,
  `deepEqual`, `encodeValue`, f64 bit utilities).
- Dart adaptations, all documented in the README porting table: no
  `toBeUndefined`/`toBeDefined`, bool-only truthiness, generic
  `toBeInstanceOf<T>()`, `Type` arguments for `toThrow`, `int`/`double` kept
  distinct, objects compared by rendering, int-only list indices for
  `toHaveProperty` (null keys allowed).
- Zero runtime dependencies. Tests mirror the reference suite (55 unit
  tests), add 8 seeded property tests over the in-lane `pbt` package and a
  2,000-iteration bounded fuzz campaign over the in-lane `fuzz` package that
  caught one real bug pre-release (null map keys in `toHaveProperty`); 100%
  line/function and 99.6% branch coverage against the 95% gate in BUILD.
