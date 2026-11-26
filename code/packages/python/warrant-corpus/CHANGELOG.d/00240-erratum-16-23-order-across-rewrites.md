## Unreleased - VER04r section 16.23: the order of R119 (c) holds across the rows of a fault, and level-missing leads

The kit is unchanged: 782 cases run, 0 failed, the mutants pass, and no committed kit case changes outcome. The
`ct-compare` v1 manifest is regenerated (the runner digest moves). No dependency added.

- **All rewrites of a `pre` fault are read step by step.** `compile_fault` scanned one rewrite completely (the three
  static codes, then `subject-forbidden`) before the next, so a `subject_op` in rewrite 1 hid an unknown tag, a nesting
  over 64 or a `model_call` in rewrite 2. It now reads every expression of the row for `level-missing`, then for
  `depth`, `unknown-tag` and `model-order`, then for `subject-forbidden`, then compiles and types (`_static_order`).
- **`level-missing` by tag precedes the three codes in a fault row.** With `requires: []` and `levels: []` an unknown
  tag or a deep nesting beat `level-missing`, because `Program.scan` checks levels after depth and tags. The fault row's
  level check now runs first. The order inside `v2-ir` is unchanged (section 11 pins none there).
- `scan_tags` gains `subjects=False` so a caller can run the `subject-forbidden` pass after all its expressions.
- **An `env` entry that is not a `[name, {type, value}]` pair is a `KitError`**; it was a bare `ValueError` from the
  unpack, before the schema check.
- Tests: `tests/test_v2_erratum_16_23.py`.
