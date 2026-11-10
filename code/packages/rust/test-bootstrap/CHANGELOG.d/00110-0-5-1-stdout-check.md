## 0.5.1 — a closed or read-only stdout is a failure (LH-003c)

### Fixed: false-pass class

- Rust std treats `EBADF` on stdout as a successful write, so a harness whose
  fd 1 was closed or read-only printed nothing and exited 0. `run::main_recorded`
  now verifies, before running anything, that stdout is open and writable
  (`fcntl(1, F_GETFL)` access mode on Unix, rejecting `O_RDONLY` and Linux
  `O_PATH`; `GetStdHandle`/`GetFileType` on Windows) and otherwise exits 1
  with a reason on stderr (`STDOUT_UNUSABLE_EXIT`).
- A closed fd 1 is re-opened as `/dev/null` by std before `main` (Linux and
  macOS alike), which hides it from any in-`main` check. A loader constructor
  samples fd 1 before std's start-up so the closed case is still caught: an ELF
  `.init_array` entry on Linux and Android, a Mach-O `__DATA,__mod_init_func`
  entry on macOS. The first CI run proved macOS needs its own entry
  (the closed-stdout test exited 0 there). Other Unix systems remain a
  documented known limit and the test does not run on them.
- The TAP plan line is written with an explicit `write_all` + `flush` and its
  error is propagated, so a failing stdout fails the run at the plan, and the
  plan is on the wire before the first test runs.

### Added

- `run::stdout_is_usable`, `run::fd_is_writable` (Unix), `STDOUT_UNUSABLE_EXIT`.
- Tests: child with fd 1 closed via `pre_exec` exits 1; child with read-only
  fd 1 exits 1; a discarding `/dev/null` stdout is (documented) accepted;
  unit tests for the fd probe, O_PATH and plan write/flush errors. The
  existing Linux-only `/dev/full` test is kept.

### Spec

- TF01 §8.3 item 9: what is and is not detectable (table of known limits).
