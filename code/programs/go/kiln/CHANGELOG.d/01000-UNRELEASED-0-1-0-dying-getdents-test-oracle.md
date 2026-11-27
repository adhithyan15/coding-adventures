## [0.1.0] - UNRELEASED

### Fixed (the real-strace dying-thread test recognizes the unavailable directory-read shape without weakening parser policy)

- `TestSandboxStraceDyingThreads` now recognizes the exact `getdents64(...)= ? <unavailable>` refusal only when a parsed terminal event belongs to the same, unambiguous PID generation. Missing, malformed, spoofed, or PID-reused terminal evidence still fails the test.
- The production trace parser continues to refuse unavailable directory reads. The failure diagnostic now prints a bounded sequence of events for the refused PID, including direct spawn evidence that could signal PID reuse.
