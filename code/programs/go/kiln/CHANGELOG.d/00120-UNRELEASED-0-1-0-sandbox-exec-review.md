## [0.1.0] - UNRELEASED

### Fixed

Adversarial-review fixes to `kiln sandbox-exec` (KILN04 section 4.7, "Review
decisions"):

- The step is a session and process group of its own (`Setsid`), so
  `kill(0, sig)` can no longer reach strace; the recorder runs strace with
  `--kill-on-exit`, and a strace that exits by signal or non-zero voids its
  whole shard (sections 4.3 and 7).
- No host IPC: the sandbox adds IPC, UTS and network namespaces (loopback only,
  brought up with `SIOCSIFFLAGS`), always masks `/run` and `/var/run`, verifies
  that `docker.sock`, `/run/user` and `/run/dbus` are absent, replaces `/dev`
  with a minimal tmpfs (`null`, `zero`, `full`, `random`, `urandom`, `tty`, the
  `fd`/`stdin`/`stdout`/`stderr` links) and sets `RLIMIT_CORE` to 0. Network is
  isolated for every action in v1; stage 5 revisits it for fetchers.
- The root spawns with `syscall.ForkExec` and `syscall.Wait4`, not
  `os.StartProcess`, whose pidfd probe is a second non-thread spawn that the
  parser refuses.
- Masks: an in-repo mask must be a direct child of the checkout, none may lie in
  a state root, a missing mask is refused, and each is mounted through an
  `O_PATH|O_NOFOLLOW` descriptor (no look-then-mount race).
- `kiln_exe` and the configuration path must be canonical and outside the
  checkout, state roots and `tmpdir`.
- `close_range` falls back only on `ENOSYS`/`EINVAL`, then walks
  `/proc/self/fd`; any other error fails. The recorder must make descriptors 1
  and 2 pipes.
- The seccomp filter also refuses `io_uring_enter`, `io_uring_register`,
  `process_vm_writev`, `userfaultfd`, `kexec_load`, `kexec_file_load`,
  `perf_event_open`, `add_key`, `request_key` and `keyctl`; the parser's refused
  class lists them.
- A liveness pipe: the init exits 125 if the root is gone, rather than relying
  on the parent-death signal. Both helpers disable the garbage collector so no
  runtime thread starts late.

### Added

- Tests: `KILN_REQUIRE_SANDBOX=1` turns every skip in the Linux integration
  tests into a failure; the user-namespace probe now runs the real sandbox; new
  tests for signalling, host IPC and a real-strace run whose trace is parsed in
  helper mode.
- A required `kiln-sandbox` CI job (gate in `ci-gates.json`, in `ci-gate`):
  lifts the AppArmor restriction, installs strace through `apt-install.sh`, and
  runs the sandbox tests, then `ci/assert_sandbox_tests.py` asserts from the
  `go test -json` stream that every named test passed and none skipped.
