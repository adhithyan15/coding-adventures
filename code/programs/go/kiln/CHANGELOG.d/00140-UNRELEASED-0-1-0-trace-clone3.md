## [0.1.0] - UNRELEASED

### Fixed

- `internal/trace`: `clone3` is decoded and tracked. Real strace 6.8 prints the
  Go runtime's thread creation as `clone3({flags=..., ...} => {parent_tid=[N]},
  88) = N`, inline or split across `<unfinished ...>` and `<... resumed>`; the
  parser did not record its return value as a spawn, so every thread was an
  `orphan-pid` and `TestSandboxRealStrace` was refused. The leading `flags=` is
  decoded as for `clone`; a struct it cannot decode exactly is `clone-flags`.
  Policy is unchanged: tolerated and tracked in the helper root and the init
  window (threads only), `refused:clone3` everywhere else.
- `futimens` is not an x86-64 system call; it is gone from the strace `-e trace=`
  list and the closed table (glibc's is `utimensat(fd, NULL, ...)`, handled).
- `TestSandboxRealStrace` now prints the lines around a refused trace line, so
  the next unknown shape needs no second CI run to find.
