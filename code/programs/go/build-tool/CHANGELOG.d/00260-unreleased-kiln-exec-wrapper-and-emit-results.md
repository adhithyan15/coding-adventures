## Unreleased — Kiln tracing hooks (KILN04 stage 3a)

- **`-exec-wrapper <json-array>`** prefixes every BUILD step with a wrapper
  argv (`wrapper... sh -c <line>`; `cmd /C` on Windows). The wrapper process
  gets `KILN_ACTION` (repo-relative package path), `KILN_STEP` (0-based) and
  `KILN_STEP_COUNT`, replacing any caller values; they are for the wrapper,
  not the BUILD. Invalid JSON, empty or NUL elements, and combination with
  `-clippy` or `-prune-leaf-targets` are usage errors. Absent, behaviour is
  unchanged.
- **`-emit-results <path>`** writes `kiln-legacy-results/1` (package, executor
  status, `steps_run`, `steps_total`; sorted) atomically after the run, and
  refuses a symlink path before building. `BuildResult` gains `StepsRun` and
  `StepsTotal`.
- **Security-review fixes.** `-emit-results` removes any older file before
  doing any work, so a missing file means the run did not complete (consumers
  must require exit 0 and a file written by this run). `wrapper[0]` is
  resolved to an absolute path at startup (a relative path with a separator is
  rejected; a bare name goes through `exec.LookPath`), so a BUILD can never
  supply the wrapper from the package directory. A wrapper that cannot start
  reports `failed` with the step not counted as run. New
  `executor.EffectiveStepLines(pkg)` returns the lines the executor really
  runs (after the Swift and Windows rewrites) for the stage 3b recipe check.
  The atomic writer chmods the open handle, and `steps_total` falls back to the
  BUILD's step count when the executor recorded none (cycle failure).
- Tests: fake wrapper recording argv and env, mid-package failure, skipped and
  would-build results, usage errors, symlink refusal, stale-file removal,
  planted-wrapper rejection, unstartable wrapper, effective step lines.
- Spec: KILN04 section 4.6 records the wiring decisions.
