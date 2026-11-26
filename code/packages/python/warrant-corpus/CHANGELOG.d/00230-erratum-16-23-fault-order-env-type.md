## Unreleased - VER04r section 16.23: the static codes of `v2-fault`, `env` types, fault-row schema (R119 to R121, R126)

The kit is unchanged: 782 cases run, 0 failed, the mutants pass, and no committed kit case changes outcome. The
`ct-compare` v1 manifest is regenerated, because the runner digest moves with the source. No dependency added.

- **R120: an `env` type the reader refuses is `ill-typed` in `v2-ir`.** `outcome_ir` raised a `KitError` for an entry of
  type `Int`, an undeclared name, `Result<U8,Q>` with `Q` a record, or any other refused text. Any refusal of the type
  text is now the outcome `ill-typed` (R111 (c)'s "no vector" for `Int` is amended). An entry that is not
  `{"type": text, "value": V}` (a missing `type` or `value`, a non-text type) and a value that does not decode stay kit
  errors.
- **R121: the op row is read with the table (order 1).** `compile_fault` checked `requires` against `levels` before it
  built the `Program` that reads the op row, so an op parameter or result naming `Int`, an error type or an undeclared
  type, beside a missing level, was `level-missing`. It builds the `Program` and reads the op row first; an op type that
  merely needs a missing level is still `level-missing`.
- **R119 and [D77]: `unknown-tag`, `depth` and `model-order` before `subject-forbidden`.** `Program.scan` and
  `scan_tags` raised `subject-forbidden` or `model-order` in one positional loop, so a `model_call` after a
  `subject_op` gave `subject-forbidden`. `model-order` now has its own pass before `subject-forbidden`, wherever the two
  stand. A `pre` fault scans every rewrite before it compiles any, so the typing of one rewrite never hides a static
  code in another. The order is the table and op row, `level-missing`, the three codes, `subject-forbidden`, then the
  typing of the row.
- **R126: a malformed fault row is a kit error, read before the table.** An extra or missing key, a `kind` that is not
  `pre`, `post` or `replace`, a non-list `requires`, an undefined level name in `fault.requires` and a malformed
  `rewrites` list (empty, a row that is not `{param, expr}`, a parameter named twice or not in the op) raised `ill-typed`
  or `corpus`; they now raise `KitError`.
- **R127 is not done here.** The row's fields before the descriptors in a not-live `v2-chunk` suite belongs to the
  corpus reader (S6b); this lane has none yet, and the kit never gives a lane a not-live suite.
- Tests: `tests/test_v2_erratum_16_23.py`; the tests that pinned the old codes (`test_v2_erratum_16_22.py`,
  `test_v2_faults.py`, `test_v2_lane_checks.py`, `test_v2_review.py`) now pin the new ones.
