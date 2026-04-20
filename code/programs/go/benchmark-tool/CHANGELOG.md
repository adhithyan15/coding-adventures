# Changelog

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
