## [0.1.0] - UNRELEASED

### Fixed (KILN04 D46a, recorder 19 to 20; flaky kiln-sandbox CI)

- A split `unshare(CLONE_FS)` in the sandbox helper's init window is no longer
  refused as `cwd-race` when other Go threads share the cwd. strace splits the
  call whenever another tracee's stop lands between its halves, so the
  `pipes` and `signals` structure probes of `kiln record prepare` failed on
  some runs only. The resumed record now carries the call's start line
  (`record.startLine`, both readers), and the unshare is checked over its own
  interval like any other cwd-dependent call; a sharer chdir completing inside
  it is still a race.
- `legacy.RecorderVersion` 19 to 20 (the accept set grows). Versions 17 and 19
  were taken by KILN04 D48 steps 3 and 5 (#19906, #19913), which merged first;
  18 is unused.
- Tests: `TestD46SplitUnshareWithSharerIsAccepted` and
  `TestD46SplitUnshareOverlappingSharerChdirRefuses` replace the test that
  pinned the old refusal.
- A probe refusal in `kiln record prepare` now names the refusing call
  (`(line N) (call unshare)`, from `trace.Error.Call`); the split-unshare
  refusals set it. New regression tests: a sharer chdir that starts before the
  unshare's window and completes inside or after it is refused; the refusal
  names `unshare`.
- CI: the kiln-sandbox job repeats `TestSandboxRecordProbe` and
  `TestSandboxRecordEmit` 8 times (`-count=8`) so a nondeterministic refusal
  shows up in one run.
