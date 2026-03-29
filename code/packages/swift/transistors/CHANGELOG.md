# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-28

### Added

- `Types.swift` — `MOSFETRegion`, `BJTRegion`, `MOSFETParams` (NMOS/PMOS defaults), `BJTParams`, `CircuitParams` (1.8 V CMOS / 5 V TTL presets), `GateOutput`, `AmplifierAnalysis`, `NoiseMargins`, `PowerAnalysis`, `TimingAnalysis`
- `MOSFET.swift` — `NMOS` and `PMOS` structs with Shockley square-law physics: `region(vgs:vds:)`, `drainCurrent(vgs:vds:)`, `transconductance(vgs:vds:)`, `outputVoltage(vgs:vdd:)`, `isConducting(vgs:)`
- `BJT.swift` — `NPN` and `PNP` structs with Ebers-Moll model: `collectorCurrent`, `isConducting`, `transconductance`; exponent clamped at 40.0 to avoid overflow
- `CMOSGates.swift` — `CMOSInverter` (2 transistors), `CMOSNand` (4), `CMOSNor` (4), `CMOSAnd` (6), `CMOSOr` (6), `CMOSXor` (12, built from 4 NANDs); all expose `evaluateDigital` for use by logic-gates
- `TTLGates.swift` — `TTLNand` (BJT-based, 5 V, 10 ns delay) and `RTLInverter` (Apollo AGC style, 3.6 V, 20 ns delay)
- `Amplifier.swift` — `analyzeCommonSource` (NMOS common-source) and `analyzeCommonEmitter` (NPN common-emitter) returning voltage gain, bandwidth, operating point
- `Analysis.swift` — `computeNoiseMargins`, `computeTTLNoiseMargins`, `analyzePower`, `analyzeTTLPower`, `analyzeTiming`, `compareCMOSvsTTL`, `demonstrateCMOSScaling` (Dennard scaling)
- ~70 XCTest cases covering every struct and free function
