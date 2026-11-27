## Unreleased - second review fixes to the oracle's share of VER04r 16.23 (CV2-F17)

- **F1.** `reached_levels` levels trees with `known_levels` (it called `ir_levels`, which raises on the tag `list`), so a stand-in holding `list`, or
  calling a model that holds it, is the R100 `KitError` and `requires_of` no longer raises a bare `OracleError`.
- **F2.** The `v2-chunk` reader compiles a corpus model in full (tier 1) only when its types and tags lie inside the suite row's `requires` plus
  `ir.core` (`check_corpus_ops_models(corpus, row)`); any other model gets the tier-2 checks. A model beyond `requires` that the oracle can compile and
  that fails typing or cost is a `KitError` (R101: the outcome would depend on the lane's levels), not `corpus`. A model beyond the oracle's own levels
  (`ir.list`, `ir.seq`) is never compiled.
- **F3.** The reader calls `undeclared_err_case` for the models compiled in full (an `err` of a case no error type declares is `corpus`).
- **F4.** `derive_fault`'s level check always includes `ir.core` (the op's own types need it): `requires []` with `levels []` is `level-missing`
  whatever the tree holds, and `["max_value","U8"]` at `levels ["ir.fixed"]` is too.
- **F5.** `check_op_row_shape` refuses a non-text type (`params [["a", 3]]`, `ret 3`, `ret null`): a `KitError` in `v2-fault`, `ill-typed` in `v2-ir`,
  `corpus` in a chunk corpus part.
- **F6.** Tests: the R125 `subject_op` and 65-deep cases now bind their variables and use the right arity, an in-level 65-deep body is refused and a
  64-deep one accepted, and each of F1 to F5 has tests (`tests/test_cv2_16_23_checks.py`). No committed byte moves (958 identical).
