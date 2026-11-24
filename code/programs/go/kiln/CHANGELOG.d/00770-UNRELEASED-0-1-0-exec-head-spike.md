## [0.1.0] - UNRELEASED

### Added (KILN04 exec-head spike; no production change)

- `code/specs/KILN04-exec-head-spike.md`: a design note on recording an exec'd
  binary's head at exec time, so a binary the step made under the fresh tmpfs and
  deleted (Go's `pkg.test`) stops being refused as
  `trace-incomplete:exec-unreadable:under-tmp`. It compares fanotify
  `FAN_OPEN_EXEC` with seccomp user notification, recommends the latter, and
  sketches a recorder sidecar the loader-chain logic would read. Each claim is
  marked VERIFIED or INFERRED.
- `TestExecHeadSpike` (`exechead_spike_linux_amd64_test.go`): an experiment that
  tries fanotify and seccomp user notification in an unprivileged process, in a
  user namespace of its own, with the listener handed to a supervisor outside that
  namespace, and in the real sandbox's step, and logs the kernel's answers as
  `PROBE: exechead` lines. It asserts nothing about either mechanism. It is gated
  (`KILN_EXECHEAD_SPIKE=1`), is not named `TestSandbox*`, so the kiln-sandbox job
  never runs it, and is not in `ci/assert_sandbox_tests.py`.
- `.github/workflows/kiln-exechead-spike.yml`: a manual `workflow_dispatch`
  workflow that runs it on ubuntu-24.04 and 26.04 and puts the PROBE lines in the
  step summary.
- Probes added to the spike: `exec-with-writer` (exec of a file with an existing
  writer or shared writable mapping), the `xns` child in a pid namespace with
  `strace -f` and the supervisor's view of the pid, and `PR_SET_MM_MAP(exe_fd)`.
