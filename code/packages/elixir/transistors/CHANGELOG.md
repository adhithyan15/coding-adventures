# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- `CodingAdventures.Transistors.Types` — parameter structs (MOSFETParams, BJTParams,
  CircuitParams) and result structs (GateOutput, AmplifierAnalysis, NoiseMargins,
  PowerAnalysis, TimingAnalysis)
- `CodingAdventures.Transistors.MOSFET` — NMOS and PMOS transistor functions
  (region detection, drain current, transconductance, conducting check)
- `CodingAdventures.Transistors.BJT` — NPN and PNP transistor functions
  (region detection, collector/base current, transconductance)
- `CodingAdventures.Transistors.CMOSGates` — CMOS logic gates built from
  MOSFET pairs (inverter, NAND, NOR, AND, OR, XOR) with analog and digital
  evaluation, VTC generation, power analysis
- `CodingAdventures.Transistors.TTLGates` — TTL NAND gate and RTL inverter
  with analog and digital evaluation, static power computation
- `CodingAdventures.Transistors.Amplifier` — Common-source (MOSFET) and
  common-emitter (BJT) amplifier analysis
- `CodingAdventures.Transistors.Analysis` — Noise margin computation, power
  analysis, timing analysis, CMOS vs TTL comparison, CMOS scaling demonstration
- Full test suite porting all tests from the Python transistors package
- BUILD file, README.md, CHANGELOG.md
