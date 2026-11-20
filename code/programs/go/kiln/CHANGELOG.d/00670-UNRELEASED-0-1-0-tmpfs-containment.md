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
- Spec: KILN04 D40.

### Added

- `internal/traced/tmpcontain_test.go`; the differential fuzzer's model
  (`model_test.go`) treats a tmp lookup as own-or-absent while no link was made
  there, and makes a rename or link between the checkout, `Home` and `Tmpdir`
  fail with `EXDEV`, as the separate mounts do.
