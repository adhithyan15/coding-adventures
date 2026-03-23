# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- **types.lua** -- Operating region constants (MOSFET cutoff/linear/saturation, BJT cutoff/active/saturation), parameter constructors (MOSFETParams, BJTParams, CircuitParams), result type constructors (GateOutput, AmplifierAnalysis, NoiseMargins, PowerAnalysis, TimingAnalysis), and validate_bit helper
- **mosfet.lua** -- NMOS and PMOS transistor simulation with region detection, drain current (Shockley model), is_conducting, output_voltage, and transconductance
- **bjt.lua** -- NPN and PNP transistor simulation with region detection, collector current (Ebers-Moll model with exponent clamping), base current, is_conducting, and transconductance
- **cmos_gates.lua** -- CMOS logic gates built from MOSFET pairs: CMOSInverter (2T), CMOSNand (4T), CMOSNor (4T), CMOSAnd (6T = NAND + INV), CMOSOr (6T = NOR + INV), CMOSXor (6T via 4 NANDs). Each supports analog evaluate() and digital evaluate_digital(). Inverter also provides static_power(), dynamic_power(), and voltage_transfer_characteristic()
- **ttl_gates.lua** -- TTL NAND gate (7400-series style, 3 NPN transistors) and RTL inverter (Apollo-era, 1 NPN transistor). Both support analog and digital evaluation with static_power() for TTL
- **amplifier.lua** -- Common-source MOSFET amplifier analysis and common-emitter BJT amplifier analysis, computing voltage gain, transconductance, input/output impedance, bandwidth, and DC operating point
- **analysis.lua** -- Noise margin computation (CMOS and TTL), power analysis (static + dynamic), timing analysis (propagation delay, rise/fall time, max frequency), CMOS-vs-TTL comparison, and CMOS scaling demonstration across process nodes (180 nm down to 3 nm)
- **init.lua** -- Top-level module re-exporting all constructors, constants, and analysis functions
- Comprehensive busted test suite with 170 tests covering all modules
- Ported from the Go implementation at `code/packages/go/transistors/`
