# Changelog

## Unreleased

- **Fix: stopping a TCP service subject now stops everything it started.** A service
  command runs through a shell (`sh -c "go run server.go ..."`, `cmd /C ...`), and `Stop`
  killed only the shell, leaving the real server running as an orphan. On Windows the
  orphan kept `service.log` open, so the results directory could not be deleted
  (`TestRunManifestStartsTCPServiceSubject` failed its TempDir cleanup in the first
  scheduled cross-OS build). On Unix the service now starts in its own process group and
  the whole group is killed; on Windows it is put in a Job Object with
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and the job is terminated. If the job cannot be
  created, `Stop` falls back to killing the process, as before. On Unix a service in its own
  group no longer receives the terminal's Ctrl-C, so the first service started installs a
  SIGINT/SIGTERM/SIGHUP handler that kills the live services and re-raises the signal (a
  Windows job dies with the tool, so it needs none). Found by the security review of this change.
  Known limits: a command that deliberately leaves the group or job escapes cleanup, and the
  Windows job is assigned just after the shell starts.
- Add `TestStopKillsTheWholeProcessTree` (Unix: the grandchild must be dead; Windows: the log
  must be deletable) and a layout check for the Windows job structure. Both were exercised
  under Wine, and the Unix test fails when the fix is removed.
- Add `BUILD_windows` (`go build -o benchmark-tool.exe .`, `go test ./... -v -cover`).

## 0.5.0 - 2026-04-20

- Add `tcp-resp` `hold` mode for C10K-style capacity benchmarks.
- Add `hold_ms` workload support so the load generator can open all sockets,
  keep them alive simultaneously, and validate RESP responses after the hold.
- Report `connected_before_hold` on hold workload trials.

## 0.4.0 - 2026-04-20

- Add phase-four comparison verdict artifacts for benchmark result directories.
- Make `benchmark-tool compare` read per-trial measurement aggregates from
  `trials.jsonl` instead of comparing summary medians alone.
- Write `comparison.json` and `comparison.md` into the candidate result
  directory with relative difference confidence intervals, Cliff's delta,
  practical threshold verdicts, and metric direction metadata.
- Suppress performance verdicts when either side has correctness failures.

## 0.3.0 - 2026-04-20

- Add the phase-three TCP/RESP load generator inside `benchmark-tool`.
- Start service subjects on allocated local TCP ports and wait for
  `ready_check = "tcp-connect"` before loading them.
- Execute `driver = "tcp-resp"` workloads with RESP frame-aware reads,
  correctness validation, per-connection samples, and trial-level throughput
  metrics.
- Support one-shot, preconnect-then-fire, pipelined, and idle TCP workload
  modes.
- Add tests for RESP parsing, concurrent TCP load generation, pipelined
  request validation, and service lifecycle startup.

## 0.2.0 - 2026-04-20

- Add phase-two git-ref comparison groundwork for command benchmarks.
- Support repeated `--subject name=ref` CLI overrides for manifest subject
  checkouts.
- Support `--subjects` and `--workloads` filters for focused local and CI runs.
- Prepare checkout-backed subjects in detached temporary git worktrees, run
  builds and command workloads from those worktrees, and remove them after the
  run.
- Write per-subject metadata to `subjects/<name>/subject.json`, including the
  exact pinned commit SHA, working directory, checkout ref, dirty state, and
  temporary worktree path.

## 0.1.0 - 2026-04-20

- Add the phase-one benchmark-tool CLI.
- Drive CLI parsing from the repository's declarative Go `cli-builder` package
  with an embedded `benchmark-tool.json` spec for local use.
- Resolve manifest subject working directories from the manifest git root so
  local runs do not depend on launching the binary from the repository root.
- Support manifest validation, host diagnostics, command benchmark runs,
  summary/report generation, and simple result comparisons.
- Write raw samples, trial summaries, environment metadata, JSON summaries, and
  Markdown reports into self-contained result directories.
