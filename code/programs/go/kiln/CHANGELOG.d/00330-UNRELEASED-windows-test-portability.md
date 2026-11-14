## Unreleased - Windows test portability

- `recordcmd_test.go` spelled its paths as `/r`, `/d`, `/b`, which `filepath.IsAbs` rejects on
  Windows (no drive letter), so seven `TestRecord*` tests failed there. They now build paths
  through `absPath`, and pass the Linux `--strace`/`--git` defaults explicitly on Windows
  (`record prepare` runs only on linux/amd64, so those defaults never apply there). Linux
  behaviour is unchanged. Reproduced under Wine before the fix (the same three
  `TestRecordPrepare*` failures as CI) and clean after.
- `TestRealBuildToolExportsAreWhatPrepareReads` is skipped on Windows with a reason: it replays
  the minimal Unix environment `record prepare` gives the build tool, a `:`-separated PATH of
  `/usr/bin:/bin`. On Windows that PATH made `git` unfindable, and there is no such environment
  to verify there.
- `BUILD_windows` gains a comment so a Windows leg runs on the pull request that changes these
  tests.
