# Changelog — silicon-rust-napi

## [0.1.0] — 2026-06-13

### Added

Initial release.  Node.js N-API addon for the silicon simulation stack.

**26 exported functions covering three Rust crates:**

*Physical constants (9):*
`kBoltzmann`, `qElectron`, `eps0`, `epsSi`, `epsOx`, `niAt300k`,
`egSiAt300k`, `muN300k`, `muP300k`

*`device-physics` (8):*
`thermalVoltage`, `intrinsicConcentration`, `fermiPotential`,
`pnJunctionBuiltInVoltage`, `pnJunctionDepletionWidth`,
`pnJunctionSaturationCurrent`, `pnJunctionCurrent`,
`mosfetThresholdVoltage`

*`mosfet-models` (2):*
`evaluateLevel1`, `evaluateLevel1Defaults`

*`fab-process-simulation` (7):*
`dealGroveOxidation`, `deposit`, `etch`, `implant`, `diffuse`,
`implantRange`, `diffusivityCm2PerS`

**Cross-section wire format** — `CrossSection` travels as a pipe-separated
`material:thickness_nm` string across the FFI boundary (same format as
`silicon-rust-python`).  Material names are validated against wire-format
delimiter injection (`|`, `:`).

**TypeScript declarations** — `silicon_rust_napi.d.ts` provides full type
coverage including `MosResult` and `ImplantRangeResult` interfaces.

**Platform portability** — All N-API code is gated with `#[cfg(not(test))]`
so `cargo test --lib` links cleanly on Windows without `node.lib`.  On
Linux and macOS, `cargo test` also builds the cdylib (undefined N-API
symbols resolved at `dlopen()` time via ELF / `-undefined dynamic_lookup`).

**18 pure-Rust unit tests** covering wire format, constants, PN junction
physics, MOSFET threshold voltage, Level-1 evaluation, and all
fab-process-simulation functions.

### Dependencies

- `device-physics 0.1.0`
- `mosfet-models 0.1.0`
- `fab-process-simulation 0.1.0`
- `node-bridge 0.1.0`
