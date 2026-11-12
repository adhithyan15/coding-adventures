# Changelog — CodingAdventures.PolynomialNative

All notable changes to this package are documented here.

## [Unreleased]

### Changed

- `BUILD_windows` now builds the NIF and runs the Mix tests on Windows (same steps as the Unix `BUILD`, copying `polynomial_native.dll` to `priv\polynomial_native.dll`) instead of skipping with `ELIXIR_WINDOWS_NIF_LINK_UNAVAILABLE`. The Windows entry point takes the BEAM's callback table and hands it to `erl-nif-bridge`; no `enif_*` import is needed.

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
