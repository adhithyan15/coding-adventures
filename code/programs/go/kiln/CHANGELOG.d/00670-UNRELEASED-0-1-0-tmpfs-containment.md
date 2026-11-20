## [0.1.0] - UNRELEASED

### Changed (classifier: tmpfs containment, KILN04 D40)

- While the action has made no symbolic link inside the fresh `Home` or
  `Tmpdir`, the stability rules do not apply to paths there: `stable` accepts an
  `underTmp` path, and every loop of `judge` skips a `contained` one. Go's build
  cache, `$TMPDIR/go-buildN`, gcc's `ccXXXXXX.o` and clang's temporaries (which
  are re-created without `O_EXCL`, unlinked, or renamed into place) no longer
  give `trace-incomplete:unstable-path` or void the write set.
- `classifier.tmpLinked` is set by `noteTmpLink` where a link is recorded, and in
  the `rename` (plain and `RENAME_EXCHANGE`) and `link` branches that carry a
  link, whenever the link's location is under the tmpfs or a tmp root. With one
  link anywhere under the tmpfs every verdict is what it was.
- `exec-unreadable:under-tmp` is unchanged.
- `RecorderVersion` 7 to 8: the verdict for these actions changes, so every weak
  key changes.
- Census only: an unstable path now carries the op class of the look that ENTERED
  the lookup set that failed the rule (`noteLookupOp(set, abs)`), not the first
  look of the path overall. The record bytes are unchanged.
- Review fix: D39 rule (a) (`observeHost`) now covers looks only. A write-class
  call (rename, rmdir, unlink, chmod, utimensat, ...) on an ancestor is
  `host-read` and writes-incomplete again. Without it, with the checkout under
  `Home`, `mv ~/work ~/y` then a listing of `~/y/x` was an unrecorded read of the
  checkout root once D40 skipped the tmpfs.
- Review fix (security, HIGH): a directory rename in the tmpfs leaves the trace's
  cwd TEXT stale while the kernel resolves `..` physically, so a path could reach
  the bound checkout with no symlink. `classifier.tmpMoved` (`noteTmpMove`) is set
  by a successful rename or exchange under the tmpfs whose source is a directory
  or an object the action did not create; `contained` is false when it is set.
  Renaming a file the action created (Go cache, gcc) stays contained. Regression
  tests in `tmpmoved_test.go` (the PoC sequence), a fuzzer burst for the stale cwd.
- Review fix (LOW): the recorder's derived state roots (`~/.cargo/registry`,
  `~/.cargo/git`) are writable host binds, not fresh tmpfs. `traced.Config.HostBinds`
  (from `record.DerivedStateRoots`) makes `fresh()` refuse them and their
  ancestors; an absent probe is still allowed.
- `stable` is back to its original body: the `!c.tmpLinked ||` test was dead code
  (its only caller, the `seen` loop, tests `contained` first).
- Spec: KILN04 D40 (premises 2 and 3 rewritten, premise 4 names the checkout's ancestor chain) and a D39 line.

### Added

- `internal/traced/tmpcontain_test.go`; the differential fuzzer's model
  (`model_test.go`) can name and write the checkout's parent, and treats a tmp lookup as own-or-absent while no link was made
  there, and makes a rename or link between the checkout, `Home` and `Tmpdir`
  fail with `EXDEV`, as the separate mounts do.
