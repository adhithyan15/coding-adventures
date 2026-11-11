# HL122 — On-demand macOS human-language-data full-corpus validation

## Status

Proposed validation child of #18852 and #18947. The pinned direct-owner reader
and its Windows security cases have landed, but a macOS Bash-compatibility
check is not evidence that the full package suite meets its unchanged budgets.
This lane supplies exact-head macOS evidence without making every unrelated
pull request pay for a whole-corpus run.

## Contract

Add a manually dispatched, read-only GitHub Actions workflow on `macos-latest`.
It must record the source SHA, confirm the runner is Apple Silicon (`arm64`),
install Node 22 and the package's transitive local TypeScript dependencies in
leaf-to-root order, compile the package, then run its complete `test:coverage`
script. Use the committed default worker, hook, test, heap, fixture, and
coverage settings: no selected test file, timeout override, skipped security
case, or reduced corpus is a full-suite result. All steps fail closed.

The workflow needs only `contents: read`; checkout must not persist credentials.
It must neither publish a book nor mutate a release, Pages branch, or issue.
Its run URL and exact source SHA are the evidence for #18852 and #18947. If the
run fails, use its terminal test and job logs to name the actual remaining
blocker; do not close either issue from a green setup or build step alone.

## Acceptance

- The workflow is manually dispatchable on main and runs one macOS arm64 job.
- A clean install uses the same leaf-to-root local dependency chain as the
  Human Languages Books job, without executing package lifecycle scripts.
- The command runs all package tests with coverage and unchanged repository
  thresholds; setup, TypeScript build, and suite verdict remain distinct.
- A terminal run on the exact current source is recorded before closing the
  macOS-dependent issue or unblocking #18941.
