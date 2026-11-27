## Unreleased - CV2 S10e (1/2): the lane implements `ir.fixed`

`IMPLEMENTED_LEVELS` gains `ir.fixed`: the lane now evaluates the 35 built-ins of VER04r Appendix A.1 (`bit_and bit_or
bit_xor bit_not shl shr sar rotl rotr popcount clz ctz bit widen wrap try to_int to_nat max_value min_value checked_add
checked_sub checked_mul div rem div_trunc rem_trunc imin imax be_bytes le_bytes from_be from_le at unwrap_or`) and the
conformance kit cases that need the level run instead of being skipped. No dependency added.

- New `v2/fixed.py`: the value rules, as plain integer arithmetic on the `w`-bit pattern. A shift count is an exact
  `Nat` compared with `w` **before** any host shift (`shl(1u8, 2^64-1)` is `0` and builds no number); `shr` is logical on
  the pattern, `sar` fills the sign, rotations reduce the count modulo `w`; `clz`/`ctz` of `0` are `w`; `checked_*` are
  `None` exactly when the exact result leaves the type; `div`/`rem` floor (`rem` has the divisor's sign), `div_trunc`/
  `rem_trunc` truncate toward zero and are `None` for a zero divisor and for `MIN / -1`.
- `Program._fixed` and friends (`v2/ir.py`): typing is inferred, as for `add`. Mixed widths, `sar` on an unsigned type,
  a count that is not a `Nat`, `widen` that narrows or changes sign, a signed `div`/`rem`, `to_nat` of a signed type
  and a plain-`Bytes` operand of `from_be`/`from_le` (the static width is part of the type, R51) are `ill-typed`. Every
  tag costs `1 +` the sum of its operands; `unwrap_or` evaluates both operands (no short circuit).
- The `Option` that nine nodes produce is an *intermediate*: only `unwrap_or` may consume it directly. An `Option` that
  reaches anywhere else (an operand of `eq`, a branch of an `if`, a tuple component, a result) needs `ir.data` and is
  `level-missing`, as is an `Option` in a signature, an environment or a literal type.
- `Semantics` gains the seams of the new rules (`shift`, `rotate`, `count_zeros`, `checked`, `divide`); the `wrap` node
  reads the existing `wrap` seam, so the no-wrap mutant breaks it too.
- Tests: `tests/test_v2_fixed.py` (every tag, a naive bitwise reference over every 8-bit value, every refusal, the
  level rules). Seven tests of the `ir.core` era that wanted "a level the runner lacks" and borrowed `ir.fixed` now
  name the levels (`levels=`) or use an `ir.data` tag (lesson added).
