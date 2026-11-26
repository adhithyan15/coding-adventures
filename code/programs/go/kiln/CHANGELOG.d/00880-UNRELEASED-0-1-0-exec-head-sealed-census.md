## [0.1.0] - UNRELEASED

### Added (KILN04 D48 step 6, items 1-3: the choice is sealed, the census reports it, the workflow can ask for it)

- **`kiln record prepare --exec-head`** seals the exec-head choice in the binding
  (`exec_head`, omitted when off so a default binding keeps its bytes) and in
  `pre/exec-wrapper.json`. `kiln record step` takes the value from the seal; a
  `--exec-head` / `--exec-head=false` of its own that disagrees is refused before the
  step runs (`StepOptions.ExecHead` is now `ExecHeadFlag *bool`, nil when absent).
- **`collect` voids a half-supervised run**: every started step's sidecar must carry
  an exec-head nonce exactly when the seal says on; either direction marks every record's
  write set unknown with `exechead-seal:` (census code `exechead-seal`, the fifth key of
  `unknown_reasons`). No `RecorderVersion` bump: the accept set is unchanged.
- **Census**: a new `exec_head` block in `census.json` and an "Exec heads" section in
  `census.md` (sealed, kernel on the trusted table, per-sub-code occurrences, unclean
  reasons per step, exec-head parse reasons, supervisor kills and notifications, Go
  emittable). Counts only. The step sidecar gains `exechead_notified` and `exechead_kills`
  (from `exechead.Result.Notified` / `.Kills`).
- **`ci/census_not_clean.py`** applies four hard criteria to a shard that ran with exec
  heads (no `exechead-*` unclean reason, no supervisor kill, no `exechead-*` parse reason or
  `exechead-seal`, no `exechead-kernel` on kernel series 7.0) and gains `--strict-exec-head`.
- **Workflow**: `kiln-census.yml` `workflow_dispatch` input `exec_head` (default false)
  passes `--exec-head` to `prepare` and puts a "NOT PR8 evidence" line first in every shard's
  summary. The scheduled run is untouched.

### Changed

- `TestSandboxRecordExecHead` (Linux only) now prepares with `--exec-head` and runs one
  supervised action; it asserts that `--exec-head=false` is refused against the on seal, and
  keeps its control as a second shard prepared without the flag (same action, own record
  directory, refused as exec-unreadable). Not run by the author (no Linux machine); it
  compiles for linux/amd64.
- The census `exec_head` block gains `seal_mismatch` (started steps disagreeing with the
  seal, counted directly); sidecar counts are capped at `MaxExecHeadCount` (1<<31) and the
  census sums saturate; the census sub-code list is held equal to the classifier's;
  `--strict-exec-head` requires a sealed summary, nonzero supervised steps and
  notifications, and real int counters.
