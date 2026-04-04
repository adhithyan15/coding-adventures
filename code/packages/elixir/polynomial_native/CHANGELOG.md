# Changelog — CodingAdventures.PolynomialNative

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-03

### Added

- Initial release: Elixir NIF wrapping the Rust `polynomial` crate via
  `erl-nif-bridge`.
- Exposed functions: `normalize/1`, `degree/1`, `zero/0`, `one/0`,
  `add/2`, `subtract/2`, `multiply/2`, `divmod/2`, `divide/2`,
  `modulo/2`, `evaluate/2`, `gcd/2`.
- Polynomials passed as Erlang lists of floats (index = degree).
- Division-by-zero guarded with `catch_unwind` → `badarg`.
- 30 unit tests covering all exported functions.
