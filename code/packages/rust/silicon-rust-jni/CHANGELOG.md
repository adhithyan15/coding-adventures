# Changelog — silicon-rust-jni

## [0.1.0] — 2026-06-13

### Added

Initial release.  JNI native library (`cdylib`) for the silicon simulation
stack.

**9 constant accessors** (all `static native double`):
`kBoltzmann`, `qElectron`, `eps0`, `epsSi`, `epsOx`, `niAt300K`,
`egSiAt300K`, `muN300K`, `muP300K`

**10 device-physics functions**:
`thermalVoltage` (infallible),
`intrinsicConcentration`, `fermiPotential`, `pnJunctionBuiltInVoltage`,
`pnJunctionDepletionWidth`, `pnJunctionSaturationCurrent`,
`pnJunctionCurrent`, `mosfetThresholdVoltage`

**2 mosfet-models functions** returning `MosResult` Java objects via JNI
`NewObjectA`: `evaluateLevel1`, `evaluateLevel1Defaults`

**9 fab-process-simulation functions**:
`deposit`, `etch`, `implant`, `diffuse`, `diffuseWithTemp`,
`dealGroveOxidation`, `dealGroveOxidationCustom`,
`implantRange` (returns `double[2]`), `diffusivityCm2PerS`

**Wire-format injection guard** — `deposit`, `etch`, `implant` reject
material/species names containing `|` or `:`.

**`cs_from_wire` validates** all material names on deserialisation, returning
`Err` for any injected or malformed entry.

**12 Rust unit tests** covering wire-format round-trips (including whole-
number decimal preservation), `validate_name`, and `cs_from_wire` rejection
paths.

### Dependencies

- `jni-bridge` (zero-dep JNI helper crate, workspace-local)
- `device-physics`, `mosfet-models`, `fab-process-simulation` (workspace)
