## [0.1.0] - UNRELEASED

### Fixed (D33: a phantom execve under a thread that an exec killed)

- `kiln record prepare`'s "signals" probe trace was refused `unresumed-call` by
  both trace readers. strace (6.8) printed a second, byte-identical
  `execve(... <unfinished ...>` under a Go runtime thread that `de_thread` had
  killed at a syscall-entry stop, then that thread's `+++ exited with 0 +++`.
  Its `PTRACE_GET_SYSCALL_INFO` returned `op=NONE` and it only checks for
  `EXIT`, so it reprinted the static entry buffer (the exec leader's call).
- The line is now accepted, and produces no event, only when the pid's next line
  is its exit with code 0, an absolute-path `execve` of another pid with an
  earlier line and identical text is pending, that exec resumes `= 0`, and
  (pass 2) both pids are in one thread group. Any other case stays
  `unresumed-call`. Both readers call one helper (`exitInsideCall`).
- KILN04 section 4.5 (rule, mechanism, soundness), section 4.9 (reader
  agreement row, D33 entry). Tests: `phantom_exec_test.go`, an
  `internal/traced` case, and fuzz seeds.
