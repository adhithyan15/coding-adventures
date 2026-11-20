## [0.1.0] - UNRELEASED

### Changed (a failed action's step traces may be a prefix of the recipe)

- `traced.CheckRecipe` demanded one trace per recipe line, so a multi-step BUILD
  that failed before its last step was `trace-incomplete:recipe-mismatch`, an
  unknown write set that voids the run (about 33 actions per census shard, about
  half of all unknown write sets). Fewer traces are now accepted when every trace
  matches its line byte for byte, the earlier steps exited 0, and the last step
  exited non-zero on its own (exit line present, not killed). The record then
  carries only `failed` and its `writes_complete` is left to the traces.
- Unchanged, still a mismatch: more traces than lines, a prefix whose last step
  succeeded, a differing or reordered step, an earlier step that failed, a killed
  last step, and an empty trace set beside a non-empty recipe.
- Verdict class: these actions were a HARD unknown (`recipe-mismatch`, a run-void
  in both the strict and the relaxed merge). They are now `failed`, which the merge
  classes as truncated: hard-void -> strict-only (the relaxed report loses only
  that action). D36 does NOT remove the strict void, which is correct by M-1.
- Why it is safe: the build tool stops at the first failing step
  (`runCommands` in `build-tool/internal/executor/executor.go`, currently near
  line 474, returns near 531), a failed
  action emits no manifest, and a BUILD rewritten to drop trailing steps ends on
  a step that succeeded, which still mismatches. KILN04 D36 (section 4.5) says so.
- `RecorderVersion` 5 -> 6: the verdict for these actions changes, so every weak
  key changes.

### Changed (census strict flag)

- The census counted `writes_unknown` as `writes_complete` false plus
  `record.failed`, so a failed action with complete writes dropped out of
  `strict_merge_voids_shard` although the merge still strict-voids it. New
  additive keys, overall and per language: `truncated` (a believed record with
  `failed` or `daemon-left`, or fewer steps than planned; mirrors recmerge
  `unknown()`), `relaxed_would_lose`, and `relaxed_merge_voids_shard`.
  `strict_merge_voids_shard` is now `writes_unknown > 0 || truncated > 0 || shard_void`.

### Added

- `TestCensusTruncatedActionsAndRelaxedLoss`.

- `TestCheckRecipeFailedPrefix` (table) and `TestClassifyFailedPrefixRaisesOnlyFailed`.
