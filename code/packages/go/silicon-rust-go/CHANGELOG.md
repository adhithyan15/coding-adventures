# Changelog — silicon-rust-go

## [0.1.0] — 2026-06-13

### Added

Initial release.  Go CGo wrapper for the Rust silicon simulation stack.

**9 constant accessors** (all return `float64`):
`KBoltzmann`, `QElectron`, `Eps0`, `EpsSi`, `EpsOx`, `NiAt300K`,
`EgSiAt300K`, `MuN300K`, `MuP300K`

**9 device-physics functions** (1 infallible + 8 returning `(float64, error)`):
`ThermalVoltage`, `IntrinsicConcentration`, `FermiPotential`,
`PNJunctionBuiltInVoltage`, `PNJunctionDepletionWidth`,
`PNJunctionSaturationCurrent`, `PNJunctionCurrent`,
`MosfetThresholdVoltage`

**2 mosfet-models functions** returning `(MosResult, error)`:
`EvaluateLevel1` (12 numeric args), `EvaluateLevel1Defaults` (4 args)

`MosResult` struct: `Id, Gm, Gds, Gmb, Cgs, Cgd, Cgb, Cbs, Cbd float64`
plus `Region string`.

**9 fab-process-simulation functions:**
`Deposit`, `Etch`, `Implant`,
`Diffuse`, `DiffuseWithTemp`,
`DealGroveOxidation`, `DealGroveOxidationCustom`,
`ImplantRange` (returns `rp, straggle float64, err error`),
`DiffusivityCm2PerS` (infallible, returns `float64`)

**Wire-format injection guard** — `Deposit`, `Etch`, `Implant` reject
material/species names containing `|` or `:`.

**41 test functions** in `silicon_rust_go_test.go` covering constants,
physics functions, process simulation, MosResult struct, error cases, and
the injection guard.

### Dependencies (indirect)

- `silicon-rust-cgo` (Rust cdylib) — built separately with
  `cargo build -p silicon-rust-cgo --release`.
