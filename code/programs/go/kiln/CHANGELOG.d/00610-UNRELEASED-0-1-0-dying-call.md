## [0.1.0] - UNRELEASED

### Changed (tolerate a task that dies inside a call when its thread group exits)

- Real-build census runs showed three strace shapes from a thread killed
  mid-syscall by its group's `exit_group` (every threaded runtime, Go first, makes
  them) and both readers refused all of them, leaving Go actions unparsed:
  `<... tgkill resumed>) = ? <unavailable>`, `<... clone resumed> <unfinished
  ...>) = ?`, and an unfinished call followed only by the pid's exit.
- Syntax, accepted identically by both readers: `UNAVAILABLE` token (before
  `ERR_LT`) and `tail` in `code/grammars/strace`, and `resumed_line = RESUMED
  inner UNFINISHED RPAREN tail`; the shared result parser accepts exactly
  `= ? <unavailable>` and `dyingResumeTail` exactly ` <unfinished ...>)` then
  `= ?`. The embedded grammar data of `strace-lexer` and `strace-parser` was
  regenerated and the parser's goldens refreshed.
- Policy, in shared code (`dyingPolicy`, `exitInsideCall`; pass 2 has no part):
  the pid's next line must be its own exit; an ignored-class call is dropped;
  `fork` and `vfork` are dropped with no spawn recorded (a child that did start is
  an `orphan-pid`); a `clone` is dropped only when `flags=` is present, every
  token is a known name and none is `CLONE_UNTRACED` or `CLONE_NEW*`. EVERYTHING
  else stays `unresumed-call`, in particular every call that READS (stat, listing,
  open): the model infers reads only from those records, so dropping one removes
  an input, and a live sibling thread, fork or exec can still use the result. A
  Go worker dying inside `openat` or `newfstatat` is therefore still refused; the
  follow-up is to record it as an observed read with an unknown result.
- The stricter-only row "ERR token in the text after the close" is not widened.
- The census shape lists `UNAVAILABLE` (`shapeTokenNames`).
- `RecorderVersion` 4 -> 5: traces that were refused now parse, so every weak key
  changes.
- KILN04 D35 (section 4.3), the stricter-only table and section 14 say so.

### Added

- `dying_call_test.go`: the accepted shapes and every near miss refused (both
  readers, via the cross-check hook), the policy table, dying clones with
  forbidden, hex, unknown or missing flags in and outside the helper layout, and
  fuzz seeds for `FuzzLineDiff` and `FuzzParseDiff`; `TestShapeUnavailable`.
- `TestSandboxStraceDyingThreads` (linux/amd64, required): a Go fixture whose main
  goroutine calls `os.Exit` while workers are mid-syscall, run under real strace.
  It asserts the loop ran, that the readers agree, and that any refusal is
  `unresumed-call` on a handled call; it does not assert that the trace parses.
