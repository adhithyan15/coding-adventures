## [0.1.0] - UNRELEASED

### Fixed (KILN04 D46a, recorder 17 to 18; flaky kiln-sandbox CI)

- A split `unshare(CLONE_FS)` in the sandbox helper's init window is no longer
  refused as `cwd-race` when other Go threads share the cwd. strace splits the
  call whenever another tracee's stop lands between its halves, so the
  `pipes` and `signals` structure probes of `kiln record prepare` failed on
  some runs only. The resumed record now carries the call's start line
  (`record.startLine`, both readers), and the unshare is checked over its own
  interval like any other cwd-dependent call; a sharer chdir completing inside
  it is still a race.
- `legacy.RecorderVersion` 16 to 18 (the accept set grows; 17 is held by
  another open change).
- Tests: `TestD46SplitUnshareWithSharerIsAccepted` and
  `TestD46SplitUnshareOverlappingSharerChdirRefuses` replace the test that
  pinned the old refusal.
