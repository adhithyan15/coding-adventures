## [0.1.0] - UNRELEASED

### Added (KILN04 D44, `kiln:recorder` 11 to 12)

- An exit line with no other record for its pid is accepted when D35 dropped at
  least as many dying spawns (fork, vfork, clone) in the trace; with more
  orphans than dropped spawns all keep refusing as `orphan-pid`. The orphan's
  exit stays in `Trace.Exits` (so `DaemonLeft` still sees it) but is no process.
- A dying `stat`, `lstat`, `access` or `readlink` with a `NULL` path is dropped
  like the ignored class. `newfstatat` and `statx` are not (AT_EMPTY_PATH).
- `mknod` and `mknodat` accept the short form strace prints for a regular file,
  FIFO or socket (no `dev`); a short `S_IFCHR` or `S_IFBLK` is still `arity`.
- Census: a refused dying call whose name is outside the table reports
  `unknown-number:yes` or `unknown-number:no` (strace's `syscall_%#lx`
  spelling) in the refusal-by-syscall tables, never the name. No drop rule yet.

### Changed

- `exitInsideCall` also returns whether it dropped a spawn; `RecorderVersion` is
  12 (the accept set grows, so every weak key changes).
