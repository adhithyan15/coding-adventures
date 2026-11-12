# Changelog — CodingAdventures.GF256Native

## [Unreleased]

### Changed

- `BUILD_windows` now builds the NIF and runs the Mix tests on Windows (same steps as the Unix `BUILD`: `cargo build --release`, copy `gf256_native.dll` to `priv\gf256_native.dll`, `mix compile`, `mix test`) instead of skipping with `ELIXIR_WINDOWS_NIF_LINK_UNAVAILABLE`. The Windows entry point takes the BEAM's callback table and hands it to `erl-nif-bridge`; no `enif_*` import is needed.

## [0.1.0] — 2026-04-03

### Added

- Initial release: Elixir NIF wrapping the Rust `gf256` crate via
  `erl-nif-bridge`.
- Exposed functions: `add/2`, `subtract/2`, `multiply/2`, `divide/2`,
  `power/2`, `inverse/1`.
- All elements passed as Erlang integers (0–255).
- Division-by-zero and inverse-of-zero guarded with `catch_unwind` → `badarg`.
- 24 unit tests covering all exported functions and edge cases.
