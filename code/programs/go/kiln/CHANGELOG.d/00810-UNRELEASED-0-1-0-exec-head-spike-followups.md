## [0.1.0] - UNRELEASED

### Changed (KILN04, exec-head spike; test and note only, no production code)

- The gated `TestExecHeadSpike` now names its copied `true` and `sleep` binaries
  by their real names (a multi-call coreutils, as on ubuntu-26.04, picks the
  applet from `argv[0]`; the old `bin-sleep2` copy most likely made the strace
  case exit 1 there), prints `prep ... multicall=`, and reports strace's stderr
  and the tracee's exit separately (`strace_stderr=`, `tracee_exit=`,
  `strace-trace first_execve= first_stderr_write=`).
- New probe: `prctl PR_SET_MM_MAP(valid,exe_fd)=` with the process's own
  `/proc/self/stat` addresses, `brk(0)` and `/proc/self/auxv`, after a
  `PR_SET_MM_MAP_SIZE` check, in a short-lived re-executed child of `host`,
  `userns` and `sandbox`. The earlier all-zero probe is kept and marked
  inconclusive.
- New probe: `fdpath strace=... inherited=... listener=... confirm=...` passes a
  socketpair end through `strace -f` to a child that sends its seccomp listener
  back, with the test process as supervisor; `fdpath-sandbox` records that the
  real sandbox root and init cannot pass the socket to the step without a
  production change.
- KILN04-exec-head-spike.md sections 6 and 7 list the new lines, the open
  hypothesis for the 26.04 exit, and what each outcome would mean.
