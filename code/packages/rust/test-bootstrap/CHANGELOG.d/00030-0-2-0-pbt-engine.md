## 0.2.0

- Implement the VER05 Level 2 property-based testing engine in `pbt.rs`.
- Implement primitive generators (`gen_bool`, `gen_u8`, `gen_u64`, `gen_i128`),
  unbiased bounded integer generation up to 2^64 (`gen_bounded_int`), and
  length-bounded byte generator (`gen_bytes_slice`).
- Implement integrated shrinking with Hypothesis-style length minimization,
  bisection, single-element reduction, and element-wise minimization preserving
  type bounds and refinement invariants (`shrink_int`, `shrink_list`, `shrink_bytes`).
- Implement composable `Gen<T>` combinator algebra with `map`, `filter`,
  `one_of`, `frequency`, and `list`.
- Register the closed PBT failure vocabulary (`property-counterexample`,
  `property-exhausted`, `property-timeout`, `property-panic`) in `verdict::is_registered_failure_id`
  and enforce privacy-preserving reproducer reporting in CI mode (VER05 §5.2).
- Add the `pbt_corpus` integration test executing all 25 `pbt-v1` conformance
  records against the native engine via `Suite`.
