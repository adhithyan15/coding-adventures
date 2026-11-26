## Unreleased - the Python-lane checks VER04r section 16.22 lists as not built (D74, D75, R110, R111, R115 to R117)

The kit is unchanged: 782 cases run, 0 failed, and no committed kit case changes outcome. The `ct-compare` v1 manifest
is regenerated, because the runner digest moves with the source. No dependency added.

- **[D74], R110 `ill-typed` in `v2-fault`.** `faults.compile_fault` refuses a `types` table that breaks section 2.2
  as `ill-typed` (it was `corpus`, a code outside the closed set of `v2-fault`). The order is the row's shape, the
  table, `level-missing`, `subject-forbidden`, then the typing of the row; a table is never hidden by a missing level,
  a subject call or an ill-typed tree. A fault expression that is ill-typed as a tree was already `ill-typed`.
- **[D75], R111 `Int` as a model parameter.** `Program._compile_model` reads a model parameter with `allow_int`, so
  `m(a: Int)` is accepted in `v2-ir` `models`, in `v2-chunk` `corpus.models` and in a corpus part, also under `Option`,
  `Result`, a tuple or `List` (a model of such a type at a level the lane lacks is `level-missing` and not-live, no
  longer `ill-typed`). An op row, a binder, a field and every value still refuse `Int`; model results are unchanged
  (R112). An `env` entry of type `Int` stays a `KitError` (R111 (c) calls it no vector).
- **R115 order inside `v2-ir`.** Pinned by tests: the bound on `max_ir_cost`, then the table, then every other check.
  No code moves.
- **R116 `list` descriptor no longer hides the models.** `chunks._assemble_suite` builds the `Program` before the
  binders' descriptors, so a model's unknown tag (or any other whole-corpus defect) is `corpus` with a `List<U8>` binder
  as it is without one.
- **R117 an error type is not a value type.** `wtypes.check_types_table` refuses a field or constructor component that
  names an error type anywhere but the second argument of `Result` (alone, or under `Option`, a tuple or `List`), with
  the code of the context. The type readers (`parse_type`) already refused a bare error name; tests pin it for an op,
  a model, a binder and a `v2-value` type.
- Tests: `tests/test_v2_erratum_16_22.py`; `tests/test_v2_lane_checks.py` now expects `ill-typed` for a fault case's bad
  table and holds the R117 tables.
