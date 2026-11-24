
## [0.1.0] - UNRELEASED

### Changed (KILN04 D46, `kiln:recorder` 14 to 15)

- `cwd-race` is now raised by overlap, not by the existence of a sharer. A
  successful `chdir` or `fchdir` on a cwd shared with another process
  (threads, `CLONE_FS`) is accepted unless the line interval of a sharer's
  cwd-dependent call (relative path with no dirfd or `AT_FDCWD`, a copying
  `fork`/`clone`, helper `unshare(CLONE_FS)`) or of another successful
  chdir/fchdir intersects its own. A split (`<unfinished ...>`) chdir spans its
  start line to its completion line. Later relative paths resolve against the
  new cwd. Failed chdir/fchdir change nothing; fork children are not sharers.
- Also cwd-dependent: `bind`/`connect`/`sendto`/`sendmsg`/`sendmmsg` with a
  relative non-abstract `AF_UNIX` pathname address, and every `exec` (a `#!`
  line may name a relative interpreter). The census may now attribute
  `cwd-race` to the sharer's call (`openat`, `connect`, ...) instead of
  `chdir`.
- Also cwd-dependent: `AT_FDCWD` with an empty path (`statx`/`newfstatat`/
  `fchownat`/`linkat`/`execveat` on the cwd directory itself). A split helper
  `unshare(CLONE_FS)` with another sharer is refused as `cwd-race`.
- Both trace readers get the rule (it is in the shared model), checked by the
  cross-check hook in new `internal/trace/d46_test.go`; four older tests that
  expected a refusal without an overlap now build an overlap.
- `RecorderVersion` bumped 14 to 15 in `internal/legacy/traced.go` (the verdict
  changes both ways; 14 is D44a, #19873).
- Added Decision D46 to `code/specs/KILN04-traced-read-sets.md` and updated
  the cwd-race lines of its parser section and of the README.
