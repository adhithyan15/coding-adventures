## [0.1.0] - UNRELEASED

### Changed

- `record.VerifyTree(root, expectedCommit)` now returns `Result{Commit}`
  instead of the gitlink list. `HEAD^{commit}` is resolved once to a full id and
  `ls-tree -r -z --full-tree <id>` is checked against it; the backstop after the
  build passes the first call's id, so a BUILD that ran `git commit -a` fails.
  The root must be the repository's top level (`rev-parse --show-prefix` empty),
  for `Inventory` too.
- A gitlink is an error (`submodule-unsupported`) in `VerifyTree` and
  `Inventory`, until submodules are designed.
- `lsTree` refuses a path with an empty, `.` or `..` component, a `.git`
  component (any case), a leading `/`, a backslash or a NUL.
- `Inventory` fails when a tracked path is a directory on disk, and stops at the
  record format's maximum instead of collecting everything first.
- `source`: every Git call now runs with `--no-replace-objects`,
  `GIT_NO_REPLACE_OBJECTS=1` and `GIT_NO_LAZY_FETCH=1`.
- `legacy.DecodeSteps` also refuses invalid UTF-8, U+FFFD, empty lines and null
  anywhere, and a document with more than 2^20 values; `MaxStepsBytes` drops to
  16 MiB (the real export is 1.4 MiB). `ReadStepsFile` lstat-s and opens with
  `O_NOFOLLOW|O_NONBLOCK`, refusing a FIFO or a symlink.
- `legacy.DecodeSteps` counts JSON nesting in its token pre-pass and refuses
  anything deeper than `MaxStepsDepth` (8; a valid export is 4), so a run of
  `[` no longer costs memory per level.
- KILN04 section 4.8 updated, with a "Pending for PR 2b" list.

### Tests

- Regression tests from the review: `TestPoCReplaceRef`,
  `TestPostBuildCommitMovesHead`, `TestPoCSubdirRoot`, `TestPoCGitlinkFile`,
  `TestPoCInventoryTrackedReplacedByDir`, `TestForgedTreeNamesAreRefused`,
  `TestPoCDecodeLossy`, plus parser unit tests. `internal/record` coverage is
  95.8%.
