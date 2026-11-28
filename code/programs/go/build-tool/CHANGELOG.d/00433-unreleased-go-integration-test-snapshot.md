## Unreleased — Go integration test snapshot

- Reuse one immutable, checkout-root-scoped package and test-trigger snapshot
  across repository-backed resolver tests; each caller gets cloned package,
  graph, and trigger data, while temporary fixtures remain isolated.
- Keep the oversized Cargo-entry test's one-second limit on the resolver call
  alone. The temporary repository and 1 MiB fixture are prepared before the
  timer starts, and the test log reports setup and parser durations separately.
