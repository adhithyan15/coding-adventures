# Changelog

All notable changes to the `transistors` package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.3.0] - 2026-04-02

### Changed

- **Operations pattern**: Wrapped all public constructors and analysis functions with `StartNew` for automatic timing, structured logging, and panic recovery. Functions covered: `DefaultMOSFETParams`, `DefaultBJTParams`, `DefaultCircuitParams`, `NewNMOS`, `NewPMOS`, `NewNPN`, `NewPNP`, all transistor methods (`Region`, `DrainCurrent`, `IsConducting`, `OutputVoltage`, `Transconductance`, `CollectorCurrent`, `BaseCurrent`), `NewCMOSInverter`, `NewCMOSNand`, `NewCMOSNor`, `NewCMOSAnd`, `NewCMOSOr`, `NewCMOSXor`, `NewCMOSXnor`, `NewTTLNand`, `NewRTLInverter`, `AnalyzeCommonSourceAmp`, `AnalyzeCommonEmitterAmp`, `ComputeNoiseMargins`, `AnalyzePower`, `AnalyzeTiming`, `CompareCMOSvsTTL`, `DemonstrateCMOSScaling`. The public API is fully backward-compatible.

## [0.2.0] - 2026-03-29

### Added

- **`CMOSXnor` gate** (`cmos_gates.go`): XNOR implemented as XOR followed by an
  Inverter (8 transistors total). Includes `Evaluate` (analog voltages) and
  `EvaluateDigital` (0/1 inputs) methods. Truth table: (0,0)→1, (0,1)→0,
  (1,0)→0, (1,1)→1.

## [0.1.0] - 2026-03-21

### Added

- **MOSFET simulation** (`mosfet.go`):
  - NMOS and PMOS transistor types with Region, DrainCurrent, IsConducting, OutputVoltage, and Transconductance methods
  - Three operating regions: cutoff, linear, saturation with Shockley model equations
  - Default 180nm CMOS process parameters

- **BJT simulation** (`bjt.go`):
  - NPN and PNP transistor types with Region, CollectorCurrent, BaseCurrent, IsConducting, and Transconductance methods
  - Simplified Ebers-Moll model with overflow-safe exponent clamping
  - Default 2N2222-style transistor parameters

- **CMOS logic gates** (`cmos_gates.go`):
  - CMOSInverter (2T), CMOSNand (4T), CMOSNor (4T), CMOSAnd (6T), CMOSOr (6T), CMOSXor (6T)
  - Analog voltage evaluation with full GateOutput (voltage, current, power, delay)
  - Digital evaluation with input validation
  - Voltage transfer characteristic (VTC) generation
  - Dynamic power calculation (P = C * Vdd^2 * f)

- **TTL/RTL logic gates** (`ttl_gates.go`):
  - TTLNand with 7400-series simplified circuit model (3 NPN transistors)
  - RTLInverter with Apollo Guidance Computer-era circuit topology
  - Static power dissipation calculation showing TTL's fatal flaw

- **Amplifier analysis** (`amplifier.go`):
  - AnalyzeCommonSourceAmp for NMOS amplifier (gain, gm, impedance, bandwidth)
  - AnalyzeCommonEmitterAmp for NPN amplifier with r_pi input impedance

- **Electrical analysis** (`analysis.go`):
  - ComputeNoiseMargins for CMOS inverter and TTL NAND
  - AnalyzePower with static/dynamic/total breakdown
  - AnalyzeTiming with propagation delay, rise/fall times, max frequency
  - CompareCMOSvsTTL side-by-side comparison
  - DemonstrateCMOSScaling across technology nodes (180nm to 3nm)

- **Comprehensive test suite** (105 tests, 95.2% coverage):
  - types_test.go, mosfet_test.go, bjt_test.go, cmos_gates_test.go
  - ttl_gates_test.go, amplifier_test.go, analysis_test.go
