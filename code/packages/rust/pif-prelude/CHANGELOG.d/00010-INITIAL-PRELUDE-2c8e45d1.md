## [0.1.0] - 2026-11-24

### Added

- PIF-004b: the generator of the PIF prelude (VER32 6.4, 9.2), built with `pif-writer`, and the committed output `code/specs/data/pif/prelude.pif` (`cargo run -q -p pif-prelude > code/specs/data/pif/prelude.pif`).
- The inductive types `Bool`, `Nat`, `Int`, `Eq`, `And`, `Or`, `False`, `True`, `Exists`, `Option`, `List` and `Prod` under Lean's names, with Lean's universe parameters (and their names), parameter counts, indices, constructor names and constructor types, binder names in the ASCII class. `And` and `Prod` are plain one-constructor inductives (no projections until kernel slice 3b).
- `Pif.*`: `Pif.Not`, `Pif.not`, `Pif.and`, `Pif.or`, `Pif.xor`, `Pif.Nat.add` (by `Bool.rec` and `Nat.rec`, no literals) and the generic `Pif.Result` (`ok`, `err`). The gate-specific `GateError` is deliberately not in the prelude (it belongs in the gate model file, VER33 ADL-002).
- `oracle/dump.lean`, `oracle/run.sh` and the committed `oracle/expected.sexp`: Lean 4.35.0 as an oracle (run by hand, never by the build) recording each type, constructor and derived recursor.
- Tests: the committed file equals the generator output (drift fails); it is read as a reviewed file with no features, imports or axioms; every declaration has its expected kind; all twelve inductives and their kernel-derived recursors equal the Lean oracle (recursor binder infos erased); `Pif.Not` equals Lean's `Not`; theorems (`not (not b) = b`, `xor b b = false`, `and b true = b`, `or b false = b`, `1 + 1 = 2`, `Pif.Not False`) check through the prelude, and a false law is refused.
