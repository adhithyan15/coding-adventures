# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- MOSFET transistor models (NMOS, PMOS) with region detection, drain current,
  transconductance, and digital abstraction methods
- BJT transistor models (NPN, PNP) with region detection, collector/base current,
  transconductance, and digital abstraction methods
- CMOS logic gates: CMOSInverter, CMOSNand, CMOSNor, CMOSAnd, CMOSOr, CMOSXor
  with both analog (evaluate) and digital (evaluate_digital) interfaces
- TTL logic gates: TTLNand and RTLInverter with analog and digital evaluation,
  static power computation
- Amplifier analysis: common-source (MOSFET) and common-emitter (BJT) amplifier
  configurations with gain, impedance, bandwidth, and operating point analysis
- Electrical analysis: noise margins, power consumption (static + dynamic),
  timing characteristics, CMOS vs TTL comparison, and technology scaling
- Type definitions: MOSFETParams, BJTParams, CircuitParams, GateOutput,
  AmplifierAnalysis, NoiseMargins, PowerAnalysis, TimingAnalysis
- Region enums: MOSFETRegion, BJTRegion, TransistorType
- Comprehensive test suite with 90+ tests covering all modules
- RBS type signatures
