# Changelog — `bit-blaster`

## 0.1.0 — 2026-11-02

Initial release: VER10 part 1, encodings and bit-blasting.

### Added

- `aig`: and-inverter graphs. Edges use the literal packing; `and`
  simplifies and structurally hashes; `or`, `xor`, `xnor`, `implies`,
  `mux`, `and_all` and `or_all` are derived from it; `simulate` evaluates
  64 patterns at once.
- `tseitin`: incremental, stack-based Tseitin encoding of an edge's cone
  into any `ClauseSink` (`cdcl_sat::Solver`, `cdcl_sat::Cnf`), with model
  read-back per input.
- `bv`: `BitVec` and the QF_BV operations as circuits: bitwise operations,
  ripple-carry add/sub/neg, shift-and-add mul, constant and barrel shifts
  (logical and arithmetic, SMT-LIB overflow semantics), equality, unsigned
  and signed comparisons, and `ite`.
- Tests: exhaustive widths 1–8, random widths 9–64, Tseitin exactness on
  every input of 200 random AIGs, and DRAT-certified miters at 8, 16 and
  32 bits.
- SMT-LIB `QF_BV` reaches these circuits through the existing stack:
  `constraint-engine`'s BV tactic, run by `constraint-vm`. This crate has
  no program interpreter of its own.
