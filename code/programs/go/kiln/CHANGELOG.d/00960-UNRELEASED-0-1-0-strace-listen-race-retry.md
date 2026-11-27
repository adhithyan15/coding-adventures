## [0.1.0] - UNRELEASED

### Fixed (the real-strace sandbox tests no longer fail on strace's own PTRACE_LISTEN race)

- `runStracedImpl` (the helper behind `TestSandboxStraceDyingThreads`, `TestSandboxStraceSignals`,
  `TestSandboxStraceRestartedSpawn` and the other real-strace tests) now classifies strace's own
  failure before the test asserts anything. When strace exits 1 and every line of its stderr is
  `ptrace(PTRACE_LISTEN...): Input/output error` (`record.ClassifyStraceStderr` ==
  `ptrace-listen-eio`, plus a per-line check), the whole step is re-run from a fresh fixture, at
  most 3 attempts in all, and each retry is `t.Logf`'d with strace's stderr. Any other strace
  failure, any other stderr code, a signal-killed strace, and 3 race hits in a row still fail the
  test. Nothing becomes a skip; no trace-parser rule and no dying-shape list is widened.
  Each test logs `STRACE_LISTEN_RETRIES=N`.
- The decision is a pure function (`stracePtraceListenRace`, `straceShouldRetry` in
  `strace_retry_test.go`) with a table test that runs on every OS.
- Why: since 2026-11-25 this race failed the job in 9 of 100 runs (about 1 in 6 CI runs), always in
  the `-count=5` repeat step. KILN04's recorder already fails closed on the same stderr in
  production; only the test treated it as a defect.

### Changed (the repeat step prints every failed iteration in full)

- `ci.yml`, "Repeat the restart and signal real-strace tests": runs with `go test -json`, rebuilds
  the `-v` text for the existing counters (`RESTARTED_SPAWNS=`, `REFUSED=`, `PROBE: trace`) and,
  on failure, prints every failed run's complete output instead of `tail -n 200`. It sums
  `STRACE_LISTEN_RETRIES` and prints a `::notice::` when non-zero.
- `ci/assert_sandbox_tests.py` gains `--text FILE` and `--failures FILE` (`failed_runs`,
  `plain_text`); the single-argument assertion mode is unchanged. Tests in
  `code/scripts/tests/test_assert_kiln_sandbox_tests.py`.
