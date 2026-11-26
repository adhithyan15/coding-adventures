## Unreleased - review fixes to the oracle's share of VER04r 16.23 (CV2-F17)

- **The tag `list` is the level `ir.list`, never an unknown tag (section 16.16 R57; blocking).** New `tag_level_of` (`tag_level` plus `list` as
  `ir.list`) is used by `static_refusal`, `_first_unknown_tag` and `known_levels`. `v2-ir` and `v2-fault` now give `level-missing` for a `list` tree
  where `ir.list` is not in `levels` (they gave `unknown-tag`), and a `v2-chunk` corpus reader accepts an uncalled model whose body holds `list` (it
  needs a level the oracle lacks, so tier 2 only). A fault row may state `ir.list` in `requires`.
- **R100 stand-ins.** `stand_in_reach` adds the stand-in of every op `ir` reaches (and what it calls) for `requires` (`requires_of`); a stand-in that
  needs a level outside `levels` is a `KitError`, never `level-missing`; a stand-in of an unreached op counts for nothing (not compiled-for-signature,
  skippable by R124). No committed `requires` moves.
- **R125 in full.** `check_corpus_ops_models` reads the corpus part as a `v2-ir` case at the oracle's levels (`ORACLE_LEVELS`) with the tree
  `["lit", true]`, so a model gets what R69 gives it: every model needs a place in `model_order` (once); a `subject_op` in a body, a `model_call` to a
  missing or later model, a body nested over 64 deep, a chain over 64 frames and an unknown tag are `corpus`; a model that needs no missing level is
  compiled in full (an ill-typed body, the cost bound); one that needs a level the oracle lacks gets the tier-2 checks only.
- **Implicit `ir.core` in `v2-fault` (section 11, order 2).** `derive_fault` decides `level-missing` from `requires` plus the `ir.core` (and `ir.list`)
  the tree's tags need, so `requires: []` with `levels: []` is `level-missing` (it gave the order 3, 4 or 5 code, or a value). No committed case moves.
- **R120 (b).** An `env` value that does not decode as its type is a `KitError` (it was the out-of-set code `value`); the test that pinned `value` and
  `test_a_stand_in_of_subjects_is_compiled` (now: a stand-in of an unreached op counts for nothing) changed for these reasons.
- **Malformed op rows.** `check_op_row_shape` / `OpRowShape`: `op_sig` no longer ends in a bare `ValueError` for `params: [["a"]]` and the like. It is
  `ill-typed` in `v2-ir`, a `KitError` in `v2-fault` (the `op` is a field of the case) and `corpus` in a `v2-chunk` corpus part.
- Tests: `tests/test_cv2_16_23_checks.py`. No committed fixture, dump, corpus or kit byte moves (`make_conformance.py --check`: 958 identical).
