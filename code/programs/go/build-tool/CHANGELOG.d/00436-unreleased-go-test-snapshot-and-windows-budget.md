## Unreleased — Go repository-backed test runtime

- Reuse one immutable, checkout-root-scoped package and test-trigger snapshot
  across repository-backed resolver tests. Each caller receives cloned package,
  graph, and trigger data, while temporary fixtures remain isolated.
- Keep the oversized Cargo-entry test's one-second limit on the resolver call
  alone. The temporary repository and 1 MiB fixture are prepared before timing;
  setup and parser durations are reported separately.
- Give the Go build-tool package's full test suite a 15-minute Go test budget.
  The Windows CI run used 363 seconds in the repository-wide DSP test before
  entering the six-input CLI emit/replay regression and timed out at Go's
  default 10-minute package limit. The active test had completed three input
  iterations; the additional budget is intended to let the existing full suite
  finish without weakening its assertions. Exact-head Windows CI must confirm
  that it is sufficient. This does not establish that a future timeout is a
  deadlock or that increasing the budget cures one.
