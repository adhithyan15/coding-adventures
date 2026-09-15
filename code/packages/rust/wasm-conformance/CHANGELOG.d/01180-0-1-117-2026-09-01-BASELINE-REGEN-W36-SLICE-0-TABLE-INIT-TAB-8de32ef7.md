## 0.1.117 — 2026-09-01 — baseline regen: W36 slice 0, `table.init`/`table.copy` arity fix

Regenerated `tests/fixtures/testsuite-status.json`
(`--write-baseline`) after `wasm-wast-parser` 0.1.96 fixed
`encode_table_init_flat`/`encode_table_copy_flat`'s missing arity
detection for `table.init`'s/`table.copy`'s optional leading index
abbreviations (see that crate's own CHANGELOG for the full root-cause
writeup; no code changes in this crate itself). This was originally
misdiagnosed by `code/specs/W07-wasm-post-mvp-epics.md`'s Addendum 2 as a
~2130-directive element-segment "exprs-list" gap; `code/specs/
W36-wasm-element-segment-exprs-list.md`'s own re-verification found the
real cause was this much smaller, unrelated parser bug instead, and
specced it as "slice 0" — do first, independently of that spec's own
(still-unimplemented) exprs-list subject.

Programmatic per-file diff against the pre-fix baseline, across all 257
corpus files (every file not listed below is byte-for-byte unchanged):

| File | Before | After | Notes |
|---|---|---|---|
| `bulk.wast` | 42 NYS | 0 NYS, 0 fail | all 42 → `Pass` |
| `table_copy.wast` | 566 NYS | 0 NYS, 0 fail | all 566 → `Pass` |
| `table_copy64.wast` | 566 NYS | 0 NYS, 0 fail | all 566 → `Pass` |
| `table_init.wast` | 499 NYS | 2 NYS, 1 fail | 496 → `Pass`; 2 NYS remain (the pre-existing, explicitly-out-of-scope `arrayref`/`array.new_default` exprs-list case, split across `module`/`assert_return`); 1 → genuinely NEW real `Fail` (see below) |
| `table_init64.wast` | 499 NYS | 2 NYS, 1 fail | same shape as `table_init.wast` |
| `elem.wast` | 4 of its NYS entries were the `"unknown table identifier \"$e\""` case Addendum 2 left "undetermined, re-probe after fix" | 2 of those 4 → `Pass` (the modules now build); the other 2 (their own follow-on `assert_trap` directives) → genuinely NEW real `Fail` (same cause as `table_init.wast`'s, below) | all other `elem.wast` tallies unchanged |

Aggregate: `module` 1994→2055 pass (198 NYS remain, was 259), `action`
357→382 pass (33 NYS remain, was 58), `assert_return` 51784→52015 pass
(749 NYS remain, was 980), `assert_trap` 2973→4824 pass **+4 new fail**
(140 NYS remain, was 1995). Every other directive kind (`assert_invalid`,
`assert_malformed`, `assert_unlinkable`, `assert_exception`,
`assert_exhaustion`, `register`) is untouched by this baseline regen.

**4 genuinely new real failures, honestly reported, NOT fixed here**: all
four trace to the exact same pre-existing `wasm-runtime` gap, only
reachable now that these modules parse for the first time —
`instantiate()` never marks an active (or `declare`d) element segment as
"dropped" once it's applied, so a later `table.init` reading from that
segment wrongly succeeds instead of trapping `"out of bounds table
access"` per the real spec's own implicit-drop rule for active/
declarative segments. Confirmed via a throwaway `run_wast_source` probe
correlating each `Fail`'s directive index against the file's own parsed
top-level forms:
- `table_init.wast` byte offset 21455 / `table_init64.wast` byte offset
  31003 (i64 analog): `(table.init 2 (i32.const 12) (i32.const 1)
  (i32.const 1))` reads from elem index 2, an ACTIVE segment already
  consumed during this same module's own instantiation — should trap,
  doesn't.
- `elem.wast` byte offsets 20594 and 20815: one case reads from an active
  segment already consumed at instantiation, the other from a `declare`d
  segment (which the real spec treats as always-already-dropped); both
  should trap, neither does.

This is a `wasm-runtime` bug, unrelated to text-format parsing or this
crate — tracked separately, out of scope for the `wasm-wast-parser` fix
that unmasked it.

