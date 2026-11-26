## 0.1.1 - security review round 1

### Fixed
- TFV validator: double free when the garbage-list reallocation failed
  (`keep()` freed a block that `sb_finish`/`dup_charged` callers also freed).
  One ownership rule now: `keep()` never frees on failure, callers do.
  Regression: allocation-failure sweeps over TFV inputs whose map keys are lists
  and maps, under ASan; confirmed to fail with the double free restored.
- `pbt_corpus_evaluate` refuses more than 10,000 cases (pairwise duplicate-id check).
- `pbt_shrink_int` returns `PBT_E_ARG` for NULL arguments instead of dereferencing.
- `pbt_attempt_budget` clamps a negative step count.
- `pbt_check` has an explicit nesting counter: past 64 enclosing checks nothing is
  evaluated and the root is marked refused (never a pass).
