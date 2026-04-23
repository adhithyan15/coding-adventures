# Changelog

## [0.1.0] - 2026-04-13

### Added
- Initial WASM bindings for DT28 Markov Chain via wasm-bindgen
- `WasmMarkovChain`: full API mirroring the Rust crate
- JSON serialization for `HashMap` return types (`stationaryDistributionJson`, `transitionMatrixJson`)
- Native `cargo test` suite (10 tests, non-wasm32 guard)
