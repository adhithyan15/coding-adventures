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
- `smt`: `check_program` / `check_program_with(Limits)` decide
  `smt-lib-format` programs in `QF_BV`, including `#b`/`#x`/`(_ bvN w)`
  literals, n-ary left-associative operators, Booleans, push/pop/reset and
  shadowing. UNSAT answers carry the CNF and its DRAT proof. Unsupported
  constructs are refused with a reason; width and AIG-size limits and
  the predicate depth limit bound hostile input.
- Tests: exhaustive widths 1–8, random widths 9–64, Tseitin exactness,
  DRAT-certified miters, a 15-file QF_BV corpus, 400 random QF_BV problems
  against brute force, and refusal cases.
