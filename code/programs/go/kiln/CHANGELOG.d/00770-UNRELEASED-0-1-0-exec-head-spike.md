## [0.1.0] - UNRELEASED

### Added (KILN04 exec-head spike; no production change)

- `code/specs/KILN04-exec-head-spike.md`: a design note on recording an exec'd
  binary's head at exec time, so a binary the step made under the fresh tmpfs and
  deleted (Go's `pkg.test`) stops being refused as
  `trace-incomplete:exec-unreadable:under-tmp`. It compares fanotify
  `FAN_OPEN_EXEC` with seccomp user notification, recommends the latter, and
  sketches a recorder sidecar the loader-chain logic would read. Each claim is
  marked VERIFIED or INFERRED.
- `TestSandboxExecHeadSpike` (`sandbox_exechead_linux_amd64_test.go`): an
  experiment that tries both mechanisms in an unprivileged process, in a user
  namespace of its own, and in the real sandbox's step, and logs the kernel's
  answers as `PROBE: exechead` lines. It does not assert on either mechanism;
  it needs the sandbox only as every sandbox test does. Named in
  `ci/assert_sandbox_tests.py`.
