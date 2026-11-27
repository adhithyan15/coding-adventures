## Unreleased - CV2 S10e (2/2): the lane implements `ir.tuple`; every S10d kit case runs

`IMPLEMENTED_LEVELS` gains `ir.tuple`. The kit run is now **1,452 run, 0 skipped, 0 failed** over the six categories
(839 run and 613 skipped before): `v2-value` 177, `v2-arith` 129, `v2-ir` 717, `v2-domain` 219, `v2-chunk` 97,
`v2-fault` 113; the 176 `v2-binding` cases stay skipped whole until S6b. `ir.fixed` and `ir.tuple` are siblings (each
depends on `ir.core` alone). No dependency added; the kit is unchanged.

- `["tuple", [e1..en]]` (2 to 8 operands) and `["project", e, i]` (`i` a JSON integer, not a bool): component types are
  the operands' static types and nothing lifts under a tuple (R51); `eq`/`ne` are structural and a tuple is never
  ordered. Values, JSON and stream tag `64` were already in the codec; the tuple descriptor now enumerates through a
  seam (`Semantics.component_digits`, last component fastest).
- New `v2/patterns.py` and `Program._match`: `match` on a `Result` with the patterns `ok err err_any tuple bind wild`.
  The scrutinee is typed first and **the level is type-directed** (R76): a scrutinee that is not a `Result`, or a data
  pattern (`some none ctor bool`) at any depth, is `level-missing`. Patterns are typed against the type they test
  (arity, declared error cases, no repeated or shadowing `bind`), then a Maranget usefulness check refuses an arm no
  value reaches and a `match` that can fall through. The kit's `v2-ir` code set has no `corpus`, so this lane reports
  these corpus errors of Appendix A.1 as `ill-typed`; a lane bound of 1,024 arms per `match` is added (the spec states none; the usefulness check is quadratic in the arms). Cost is `1 +`
  scrutinee `+` the largest arm; calls are the taken arm's, decided by the pure scrutinee; an arm is under a guard whose
  condition is the scrutinee (`impure-guard`, overflow class `case`).
- Fault rows (`faults.py`, `wtypes.fault_unify`): a fault expression need only have a **common type** with what it
  replaces (R94). Tuples relate component by component under a narrower relation (R96: `(Bytes,Int)` against
  `(Bytes,Nat)` is fine, a one-byte literal in a `Bytes<2>` component is not); the declared type is handed to every
  branch, arm, `ok` payload and tuple component (R98), and a bytes literal where a plain `Bytes` is expected has the type
  `Bytes` (R67). `off-by-one-consumed` and the tuple `missing-error` templates are accepted.
- `Semantics` gains `choose_arm` (first matching arm) and `component_digits`.
- Tests: `tests/test_v2_tuple.py`; `tests/test_v2_conformance.py` pins the new counts; ten new mutants in
  `tests/v2_mutants.py` (shift count masked, `shr` that fills the sign, rotation by the width, `clz` of zero, wrapping
  `checked_add`, truncating `div`, flooring `div_trunc`, `MIN / -1` that wraps, last-arm-wins `match`, a tuple
  descriptor with the first component fastest). Coverage 99.4%.
