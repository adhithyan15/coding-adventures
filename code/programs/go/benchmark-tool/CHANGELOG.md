# Changelog

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
