## [0.1.0] - UNRELEASED

### Changed (KILN04 stage 4, PR 2b-2c-1: signals in the trace and `daemon-left`)

- strace runs with `-e signal=all` (was `signal=none`). `print_signalled` prints
  `+++ killed by SIG +++` only for signals in the set, so every process killed by a
  signal (SIGPIPE in `yes | head`, a harness's child) had no exit line and voided the
  run as `missing-exit` (D19). `--- SIG ---` lines are skipped by the parser.
- `RecorderVersion` 2 -> 3: argv and the daemon rule change what a trace means, so
  every weak key changes.
- `trace.Trace` gains `ExitLines` (parallel to `Exits`) and `InitExitLine` (the start
  line of the first `exit_group` by the init child or an init `CLONE_THREAD` thread
  after the step exec).
- `daemon-left` is now `traced.DaemonLeft`: a SIGKILL death of a descendant of the init
  child, outside its thread group, after the init's `exit_group` line. An unseen
  `exit_group` fails closed (every such death counts). The old rule assumed the
  survivor's line followed init's exit line, which the kernel's PID-namespace teardown
  probably does not guarantee.

### Added

- `TestSandboxStraceSignals` (Linux CI): sigpipe, term-waited, kill9-waited,
  daemon-left and setsid against real strace; the strace-plus-sandbox harness is now
  `runStraced`, shared with `TestSandboxRealStrace`.
