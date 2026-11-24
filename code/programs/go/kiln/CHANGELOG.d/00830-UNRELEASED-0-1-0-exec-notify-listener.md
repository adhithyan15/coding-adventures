## [0.1.0] - UNRELEASED

### Added (KILN04 D48 step 3, `kiln:recorder` 16 to 17)

- `kiln sandbox-exec --exec-notify-fd N` (optional, after `--stdout-fd` and
  `--stderr-fd`). The root checks `N` with its own rules (`CheckExecNotifySocket`:
  an open `S_IFSOCK` descriptor, `SO_DOMAIN` `AF_UNIX`, `SO_TYPE` `SOCK_STREAM`,
  `getpeername` succeeds, not an output descriptor; not `CheckOutputFD`, which
  refuses a socket by name), keeps it for the init through the existing
  close-on-exec and `Files` mapping (the init sees it as descriptor 4), and closes
  its own copy after the spawn. A missing or wrong-kind descriptor is refused
  before anything is spawned (status 125). With the option absent nothing changes
  and no second filter is installed (tested: the step has one seccomp filter and
  can install a listener of its own).
- The second seccomp filter (`BuildExecNotifyFilter`): `USER_NOTIF` on `execve`
  (59) and `execveat` (322), `ALLOW` otherwise, with its own architecture KILL
  and x32 KILL. The init installs it with `seccomp(SECCOMP_SET_MODE_FILTER,
  SECCOMP_FILTER_FLAG_NEW_LISTENER)` on its locked main thread, after the first
  filter, the way the first is installed. TSYNC finding: the first filter is
  loaded by `prctl(PR_SET_SECCOMP)`, which has no flags and is per thread, so
  there is no TSYNC to combine; and the kernel does not allow `NEW_LISTENER` with
  `TSYNC` unless `SECCOMP_FILTER_FLAG_TSYNC_ESRCH` (Linux 5.7) is also given,
  because a TSYNC failure returns a thread id that a listener descriptor cannot be
  told from. The step's exec is made by that same thread, so one thread is enough.
- The hand-over: the init sends the listener over the socket with `SCM_RIGHTS`
  (exactly one descriptor, one data byte `ExecNotifyByte` = `L`, `MSG_NOSIGNAL`)
  and closes both before `execve`; the step holds neither (tested by a descriptor
  scan inside the step, and by end of file on the supervisor's end afterwards).
  If the option was given and the listener cannot be installed or sent, the step is
  refused (status 125, message prefix `exec-notify:`); it never runs without it.
- Dead listener: a notified call whose listener is closed fails with `ENOSYS`.
  The init reports its exec failing with `ENOSYS` (only possible this way when a
  listener is configured) as status 125 and `kiln sandbox-init: exec-notify: the
  listener is gone (execve returned ENOSYS); the step did not run`. The trace then
  has a failed `execve` and no step exec: `helper-no-exec`, read with status 125 as
  `sandbox-refused`. A supervisor already gone before the send fails the send
  (`EPIPE`) with the same status and the `exec-notify:` prefix. The constants
  `ExecNotifyRefused` and `ExecNotifyListenerGone` are for PR 4 to match.
- A step that installs its own filter returning an error for `execve` only loses
  its execs (ERRNO beats USER_NOTIF, so the supervisor is not even asked), and its
  attempt to install a second listener is refused with `EBUSY`.
- Helper argv and trace parsing: the root's and the init's argv carry the new
  argument; `flagValue` (the `--config-sha256` and `--sha256` comparison in
  `record/classify_step.go`) and the trace parser's helper recognition accept it,
  with canned-trace tests of the new root and init calls (socket checks,
  `seccomp(NEW_LISTENER)`, `sendmsg(SCM_RIGHTS)`, the closes).

### Changed

- The first filter refuses `prctl(PR_SET_MM, ...)` (option 35, any sub-option)
  with `EPERM`, comparing only the low 32 bits of `args[0]` as the kernel's `int`
  option does; `35|1<<32` is refused, `35<<32|39` is not. A step cannot re-point
  `/proc/<pid>/exe`, which the exec-head confirm will rely on. Defence in depth: the
  sandbox's init has also dropped `CAP_SYS_RESOURCE`.
- `RecorderVersion` 16 to 17 (origin/main was still at 16): the refusal changes
  what a step sees, so every weak key changes. The classifier is untouched; step 5
  of D48 bumps again.
- `execStep` wraps its last error with `%w` (same text) so the init can see
  `ENOSYS`.

### Tests

- Pure, every OS: `ParseExecArgs` and `ParseInitArgs` with the option,
  `CheckExecNotifySocket`, the PR_SET_MM verdicts (including junk in the high
  bits and the neighbours 34, 36, 37, 39), the second filter's verdicts, and a
  model of the kernel's most-restrictive-wins rule over both filters for every
  number below 1024.
- Real kernel (`kiln-sandbox`, all named `TestSandbox...` and in
  `ci/assert_sandbox_tests.py`'s `REQUIRED`): `TestSandboxExecNotifyNotifies`,
  `...FirstFilterUnchanged`, `...StepHoldsNothing`, `...DeadListener`,
  `...RefusesBadDescriptor`, `...StepFilterOnlyLosesItsExecs` and `...RealStrace`
  (a real strace run with the option parses and cross-checks clean);
  `TestSandboxSeccompRefusals` gains the PR_SET_MM lines. Every test that passes
  the option runs a responder that answers CONTINUE, with a deadline and a kill of
  the process group.

### Security review follow-up (same PR, `kiln:recorder` stays 17)

- The equal-action USER_NOTIF tie is closed. A step's own filter answering
  USER_NOTIF for execve would win the tie on the kernel's "newest filter" rule, but
  the step cannot own a listener: the kernel allows one listener per task
  (`has_duplicate_listener()` gives `EBUSY`), and a USER_NOTIF filter without a
  listener fails the call with `ENOSYS` (the exec is lost, fail closed). The
  `BuildExecNotifyFilter` doc comment now says so.
- Defence in depth: the SECOND filter refuses `seccomp(2)` with `EPERM` when the
  low 32 bits of `args[1]` have `SECCOMP_FILTER_FLAG_NEW_LISTENER` set, checked
  before the execve compares. The refusal is ours, so the step sees `EPERM`, not
  the kernel's `EBUSY`. `seccomp(2)` without the flag and `prctl(PR_SET_SECCOMP)`
  stay allowed. The second filter's bytes change; the option absent still
  installs nothing.
- New real-kernel test `TestSandboxExecNotifyStepUserNotifFailsClosed` (in
  `REQUIRED`): the step's listener install is refused, and its listener-less
  USER_NOTIF filter makes its exec fail with `ENOSYS` with no notification
  reaching the responder. The two existing tests now expect `EPERM` for the
  second listener. Pure verdict tests cover the flag, junk in the high bits and
  other flags.
- `inspectExecNotifyFD` returns the `SO_DOMAIN` and `SO_TYPE` errors itself.
- `ExecNotifyListenerGone` documents that the ENOSYS inference holds only for the
  init's own exec of the step; later ENOSYS execs are PR 4's concern.
