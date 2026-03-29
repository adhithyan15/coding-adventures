# Changelog

All notable changes to the `transistors` crate will be documented in this file.

## [0.2.0] - 2026-03-29

### Added

- `CMOSXnor` struct in `cmos_gates` — CMOS XNOR gate implemented as XOR
  followed by an Inverter (8 transistors total). Provides `evaluate(va, vb)`
  for analog voltage simulation and `evaluate_digital(a, b)` for 0/1 logic.
  `TRANSISTOR_COUNT` constant equals `CMOSXor::TRANSISTOR_COUNT + 2`.

## [0.1.0] - 2026-03-21

### Added
- `types` module with operating region enums (`MOSFETRegion`, `BJTRegion`, `TransistorType`),
  parameter structs (`MOSFETParams`, `BJTParams`, `CircuitParams`), and result types
  (`GateOutput`, `AmplifierAnalysis`, `NoiseMargins`, `PowerAnalysis`, `TimingAnalysis`).
- `mosfet` module with `NMOS` and `PMOS` transistor models supporting region detection,
  drain current calculation (Shockley model), digital abstraction, and transconductance.
- `bjt` module with `NPN` and `PNP` transistor models supporting region detection,
  collector/base current calculation (Ebers-Moll model), and transconductance.
- `cmos_gates` module with complete CMOS logic gates: `CMOSInverter` (2T),
  `CMOSNand` (4T), `CMOSNor` (4T), `CMOSAnd` (6T), `CMOSOr` (6T), `CMOSXor` (6T).
  Each gate supports both analog voltage evaluation and digital (0/1) evaluation.
- `ttl_gates` module with `TTLNand` (7400-style) and `RTLInverter` (Apollo AGC-style).
- `amplifier` module with `analyze_common_source_amp` (MOSFET) and
  `analyze_common_emitter_amp` (BJT) amplifier analysis functions.
- `analysis` module with `compute_noise_margins`, `analyze_power`, `analyze_timing`,
  `compare_cmos_vs_ttl`, and `demonstrate_cmos_scaling` functions.
- Comprehensive integration tests for all modules.
- Literate programming style documentation throughout.
