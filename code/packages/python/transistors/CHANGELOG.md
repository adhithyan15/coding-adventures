# Changelog

All notable changes to the transistors package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-21

### Added
- MOSFET transistor models (`NMOS`, `PMOS`) with operating region detection (cutoff, linear, saturation), drain current calculation using Shockley equations, transconductance, and output voltage modeling
- BJT transistor models (`NPN`, `PNP`) with operating region detection (cutoff, active, saturation), collector/base current calculation using Ebers-Moll model, current gain (beta), and transconductance
- CMOS logic gates (`CMOSInverter`, `CMOSNand`, `CMOSNor`, `CMOSAnd`, `CMOSOr`, `CMOSXor`) built from NMOS/PMOS transistor instances with both digital (`evaluate_digital`) and analog (`evaluate`) interfaces
- TTL logic gates (`TTLNand`, `RTLInverter`) modeling historical BJT-based logic with static power dissipation
- Analog amplifier analysis (`analyze_common_source_amp`, `analyze_common_emitter_amp`) computing voltage gain, input impedance, transconductance, bandwidth, and operating point
- Electrical analysis functions: `compute_noise_margins`, `analyze_power` (static + dynamic P=CV²f), `analyze_timing` (propagation delay, rise/fall times, max frequency), `compare_cmos_vs_ttl`, `demonstrate_cmos_scaling`
- Comprehensive type system with enums (`MOSFETRegion`, `BJTRegion`, `TransistorType`) and dataclasses (`MOSFETParams`, `BJTParams`, `CircuitParams`, `GateOutput`, `AmplifierAnalysis`, `NoiseMargins`, `PowerAnalysis`, `TimingAnalysis`)
- 116 tests with 98.14% code coverage
- Literate programming style with extensive docstrings, ASCII circuit diagrams, and real-world analogies
