# Changelog — `bit-blaster`

## Unreleased

- Keep the multiply circuit's row index paired with the corresponding operand
  bit through iteration, satisfying the current Rust stable Clippy lint without
  changing the generated circuit.

## 0.2.0 — 2026-11-09

Faithfulness audit (VER10 §9). No bug was found in the existing encoding:
the AIG, the Tseitin clauses and every operator in 0.1.0 matched an
independent reference on every checked assignment, and 24 hand-made
mutants of the encoder and circuits were all killed.

### Added

- `bv`: `udiv`, `urem`, `sdiv`, `srem` (restoring division; division by
  zero follows SMT-LIB with no special case in the circuit), `extract`,
  `concat`, `zext`, `sext`. The spec §4/§9.1 table lists them.
- `tests/faithfulness.rs`: AIG, Tseitin and `cdcl-sat` checked together
  against a Rust reference for every operator (widths 1–4 and 5–6
  exhaustive, 7–8 corners and samples), with uniqueness of the output
  certified by `drat-check`; all depth-2 expression trees (ignored test:
  87 812 circuits, about 3.5 M assignments, about 5 minutes unoptimised;
  a 1-in-25 slice by default); certified identities and refuted
  non-identities; structural-hashing and Tseitin functional-exactness
  checks; incremental encoding.

### Changed

- The signed comparisons (and so `sle`, `sgt`, `sge`) on a width-0 vector
  now panic with "bit-vectors have width ≥ 1" instead of an opaque index
  underflow. Width 0 was never supported (spec §4).
- Spec VER10 gains §9: reference semantics, the faithfulness contract
  (F1–F4) and the spec-versus-code audit table.

### Measured

- Line coverage 100% (593/593 lines in `aig.rs`, `bv.rs`, `tseitin.rs`),
  `cargo llvm-cov` on a standalone copy with `cdcl-sat` and `drat-check`.

## 0.1.0 — 2026-11-02

Initial release: VER10 part 1, encodings and bit-blasting.

### Added

- `aig`: and-inverter graphs. Edges use the literal packing; `and`
  simplifies and structurally hashes; `or`, `xor`, `xnor`, `implies`,
  `mux`, `and_all` and `or_all` are derived from it; `simulate` evaluates
  64 patterns at once.
- `Aig::and_calls`: counts every gate construction, including ones that
  structural hashing or simplification absorb, so callers can bound the
  *work* of building a circuit separately from its size.
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
