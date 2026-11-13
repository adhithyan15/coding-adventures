## [0.1.0] - UNRELEASED

### Added

- `kiln sandbox-exec --config <abs path> -- <argv...>` (KILN04 stage 3b), in
  `internal/sandbox`: runs one traced BUILD step in new user, mount and PID
  namespaces.
  - A root process (the one strace traces) spawns exactly one child, the init
    of the new PID namespace (a re-exec of `/proc/self/exe sandbox-init`), and
    waits. The init maps the caller to uid/gid 1000, builds a read-only view of
    `/` with the checkout bound writable at its own path, fresh tmpfs on `$HOME`
    and `TMPDIR`, writable state roots, empty read-only masks over `.git` and the
    recorder's directories, and a `hidepid=2` `/proc`, then `pivot_root`s,
    drops every capability, sets `no_new_privs`, installs a hand-built seccomp
    filter, marks descriptors 3 and up close-on-exec, and `execve`s the step with
    exactly the configured environment for `KILN_ACTION`.
  - Pure and portable (tested on every OS): the strict configuration and its
    validation, `Plan` (the ordered mount list), `BuildFilter` (the BPF program,
    tested by running it in a small interpreter), `PrivSteps`, argument parsing
    and the host checks (canonical paths, mask targets, binary location).
  - linux/amd64 only; other platforms get a stub that exits 125. The Linux
    integration tests skip where unprivileged user namespaces are unavailable.
- KILN04 section 4.7 records the decisions: the exact mount order, the id map,
  the filter layout and the privilege-drop sequence.

### Changed

- `internal/trace`: in the sandbox init's window, a successful
  `unshare(CLONE_FS)` gives the calling process a private working directory, so
  the Go helper's setup `chdir`s are not a `cwd-race` with its runtime threads.
  This supersedes KILN04 section 4.5's advice to avoid `unshare(CLONE_FS)`.
- Two deliberate departures from the helper sketch in KILN04 section 4.3, both
  forced by the parser's model of the root: the init writes its own id maps
  (Go's `UidMappings` write them from the root after the clone, an `openat` the
  parser refuses), and the init sets its own parent-death signal (Go's
  `Pdeathsig` check kills a child of a new PID namespace).
