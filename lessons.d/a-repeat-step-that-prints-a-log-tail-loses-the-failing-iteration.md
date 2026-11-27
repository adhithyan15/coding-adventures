---
category: CI & GitHub Actions
---

# A repeat step that prints a log tail loses the failing iteration's message, and -count over a real-strace test multiplies exposure to strace's own races

The `kiln-sandbox` job's "Repeat the restart and signal real-strace tests" step
runs `TestSandboxStraceDyingThreads` with `-count=5` and printed only a grep and
`tail -n 200` of a multi-thousand-line log.  It failed in about one CI run in six
(9 of 100 job runs, on unrelated branches and on main), and in 7 of the 9 the
failing iteration's `t.Fatalf` text was not in what the step printed, so the
cause could not be read.  The two that did show it were strace's own race on a
dying thread: `ptrace(PTRACE_LISTEN,...): Input/output error`, strace exiting 1.

Two lessons:

- **A repeat step must print every failed iteration in full**, not a tail.  Run
  it with `go test -json` and print each failed run's whole output (key by run,
  not by test name: the `-count` runs share a name).  `assert_sandbox_tests.py
  --failures` does this and `--text` turns the stream back into the `-v` text the
  awk counters read.
- **`-count=N` over a test that runs real strace multiplies its exposure to
  strace's own races**, so the helper must classify strace's own failure
  (`record.ClassifyStraceStderr`) before the test asserts anything about the
  trace.  A failure that is exactly the known race is re-run from a fresh
  fixture (three attempts, counted as `STRACE_LISTEN_RETRIES=N`); every other
  strace failure still fails.  Never turn it into a skip, and never widen a
  parser rule to absorb it.
