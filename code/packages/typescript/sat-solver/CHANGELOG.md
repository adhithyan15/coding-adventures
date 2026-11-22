# Changelog

## Unreleased

### Fixed
- Load clauses with set-backed duplicate/tautology detection (linear) instead
  of a nested scan (quadratic on hostile long clauses, outside budget cover).
- Preserve pending trail work when a propagation step budget is exhausted, and
  retain permanent root conflicts discovered through public propagation.
- Clear stale failed-assumption cores after known-root contradictions and reject
  malformed DIMACS headers with prefixes or extra fields.
- Document failed cores as sound, duplicate-free subsets without minimality or
  ordering guarantees.
- Add 64 seeded stateful differential traces and a bounded 2,500-iteration
  mutation campaign with independent signed-CNF model and core checks.

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
