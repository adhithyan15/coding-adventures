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
- Why it is safe: the build tool stops at the first failing step
  (`build-tool/internal/executor/executor.go:474`, returns at `:531`), a failed
  action emits no manifest, and a BUILD rewritten to drop trailing steps ends on
  a step that succeeded, which still mismatches. KILN04 D36 (section 4.5) says so.
- `RecorderVersion` 5 -> 6: the verdict for these actions changes, so every weak
  key changes.

### Added

- `TestCheckRecipeFailedPrefix` (table) and `TestClassifyFailedPrefixRaisesOnlyFailed`.
