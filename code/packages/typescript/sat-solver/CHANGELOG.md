# Changelog

## 0.1.0 (2026-11-08)

### Added
- Zero-dependency Level 3 CDCL SAT Solver engine:
  - Two-Watched Literals (2WL) unit propagation with blocker literal optimization.
  - 1-UIP conflict analysis with asserting learned clause generation.
  - Non-chronological backjumping and level 0 unit preservation.
  - Variable State Independent Decaying Sum (VSIDS) decision heuristic with binary heap and periodic activity rescaling.
  - Luby restart schedule.
  - Sound incremental solving under assumptions and failed-core extraction via backward implication graph reachability.
  - Fail-closed contradiction tracking via solver `ok` flag and root-level backjump on incremental clause addition.
  - Per-solve conflict budget (`maxConflicts`) and propagation step budget (`maxSteps`) returning `"unknown"`.
- Tseitin transformation circuit bit-blaster (`CnfBuilder`):
  - Logic gate encodings: `and`, `or`, `xor`, `not`, `ite`, `equiv`.
  - 1-bit full adder and multi-bit ripple-carry adder (`bvAdd`, `bvEq`).
- DIMACS CNF reader (`parseDimacs`) and writer (`writeDimacs`) with strict validation.
