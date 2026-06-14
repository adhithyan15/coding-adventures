# Changelog — silicon-rust-java

## [0.1.0] — 2026-06-13

### Added

Initial release.  Java package wrapping the `silicon-rust-jni` Rust cdylib.

**Classes:**
- `com.codingadventures.silicon.SiliconSim` — 29 static native methods
- `com.codingadventures.silicon.MosResult` — MOSFET DC operating-point
  result (constructed by JNI)
- `com.codingadventures.silicon.SiliconException` — error type

**API surface** (all methods on `SiliconSim`):
- 9 physical constants (return `double`): `kBoltzmann`, `qElectron`,
  `eps0`, `epsSi`, `epsOx`, `niAt300K`, `egSiAt300K`, `muN300K`, `muP300K`
- `thermalVoltage(double t)` — infallible
- `intrinsicConcentration`, `fermiPotential`, `pnJunctionBuiltInVoltage`,
  `pnJunctionDepletionWidth`, `pnJunctionSaturationCurrent`,
  `pnJunctionCurrent`, `mosfetThresholdVoltage` — all `throws SiliconException`
- `evaluateLevel1`, `evaluateLevel1Defaults` — return `MosResult`
- `deposit`, `etch`, `implant`, `diffuse`, `diffuseWithTemp`,
  `dealGroveOxidation`, `dealGroveOxidationCustom` — process simulation
- `implantRange` — returns `double[2]` = {Rp, straggle}
- `diffusivityCm2PerS` — infallible

**40 JUnit 5 test cases** covering constants, physics, MOSFET model,
process simulation, wire injection guard, null handling, and full
NMOS gate-stack process flow.
