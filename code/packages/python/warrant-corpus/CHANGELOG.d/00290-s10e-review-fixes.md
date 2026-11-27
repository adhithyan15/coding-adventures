## Unreleased - CV2 S10e review: committed wasm-leb128 suites, open error types, a work budget

The kit run is unchanged (1,452 run, 0 skipped, 0 failed). No dependency added.

- **P1.** A `reference` suite's `expected` side is now checked against `result_type` with R51 unify, not flow: R42 makes
  `Nat` arithmetic an `Int`, and the body is `eq(subject, expected)`. The committed `encode_unsigned_length_formula` and
  `encode_signed_length_formula` suites (Nat `result_type`, Int `expected`) were refused as `corpus`. The subject side
  keeps its unify check. `tests/test_v2_corpora.py` now builds every suite and fault of all five committed v2 corpora at
  the lane's levels (a suite that needs `ir.data` is `level-missing`, never `corpus`).
- **P2/P7.** A `match` on a `Result` whose error type is undetermined (a bare `ok`, a case two error types declare, an `if`
  joining them) treats its cases as an open set (the named ones plus "all the rest"). An `ok`-only `match` is therefore
  refused as not exhaustive (it was accepted and crashed with a `KitError` at evaluation), while `err_any`, a wildcard, a
  `bind` and `ok(_)` with `_` are accepted, as is `err BAD` when some error type declares `BAD`.
- **P3.** The usefulness check has a work budget of 20,000 recursion steps per `match`; over it the `match` is refused
  `ill-typed` (the spec names no code). An 18-column case that took over a minute is refused in well under a second.
- **P4.** `MAX_ARMS` is 256 (Appendix A.2), `ill-typed` beyond it; a pattern nests at most 8 levels (the outermost is 1),
  `depth` beyond.
- **P5.** A `bind` named `result` is `ill-typed` in any body.
- **P6.** `from_be`/`from_le` check the runtime length is `w/8`; otherwise `ir-overflow` with the R71 class split.
