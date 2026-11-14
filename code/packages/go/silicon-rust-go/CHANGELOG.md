# Changelog — silicon-rust-go

## [Unreleased]

### Fixed

- **Windows: link the Rust crate statically.** The Windows `#cgo` line linked the cdylib's
  import library, so the test binary exited `0xc0000135` (STATUS_DLL_NOT_FOUND) because
  `silicon_rust_cgo.dll` was not on PATH. It now links the GNU-ABI staticlib
  (`x86_64-pc-windows-gnu`, `libsilicon_rust_cgo.a`) the way `go/conduit` does, built by
  `tools/run-tests.ps1` through the shared `build-conduit-capi-windows-gnu.ps1` helper
  (`-CrateDir silicon-rust-cgo -ArchiveName libsilicon_rust_cgo.a`). Linux and macOS are unchanged.
  Validated by building the archive, cross-compiling the cgo test with MinGW and running it
  under Wine (the whole suite passes).
- `BUILD_windows` runs `tools/run-tests.ps1` instead of the Unix command.

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
