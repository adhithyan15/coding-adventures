# Changelog

All notable changes to `@coding-adventures/transistors` will be documented in this file.

## [0.2.0] - 2026-03-23

### Added

- **Vacuum tube triode model** — the historical predecessor to MOSFET and BJT, now a first-class citizen in the package alongside modern transistors.
  - `TriodeParams` interface (mu, K, plateVoltage)
  - `defaultTriodeParams()` — 12AX7-style small-signal triode defaults
  - `triodePlateCurrent(gridVoltage, params?)` — Child-Langmuir 3/2 power law
  - `isConducting(gridVoltage, params?)` — digital abstraction (on/off switch)
- 11 new tests covering cutoff, conducting, monotonicity, custom parameters
- Comparison table in source documentation: triode vs MOSFET properties

## [0.1.0] - 2026-03-21

### Added
- MOSFET transistors (NMOS, PMOS) with region detection, drain current, transconductance
- BJT transistors (NPN, PNP) with Ebers-Moll model, current gain, transconductance
- CMOS logic gates: CMOSInverter, CMOSNand, CMOSNor, CMOSAnd, CMOSOr, CMOSXor
- TTL logic gates: TTLNand, RTLInverter
- Analog amplifier analysis: common-source (MOSFET), common-emitter (BJT)
- Electrical analysis: noise margins, power consumption, timing, CMOS vs TTL comparison
- Technology scaling demonstration across 180nm to 3nm nodes
- Comprehensive test suite with 80+ test cases
- Full literate programming documentation throughout
