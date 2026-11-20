## [0.1.0] - UNRELEASED

### Changed (tolerate a task that dies inside a call when its thread group exits)

- Real-build census runs showed three strace shapes from a thread killed
  mid-syscall by its group's `exit_group` (every threaded runtime, Go first, makes
  them) and both readers refused all of them, leaving Go actions unparsed:
  `<... tgkill resumed>) = ? <unavailable>`, `<... clone resumed> <unfinished
  ...>) = ?`, and an unfinished `openat(AT_FDCWD, 0x7f.., O_RDONLY <unfinished
  ...>` followed only by the pid's exit.
- Syntax, accepted identically by both readers: `UNAVAILABLE` token (before
  `ERR_LT`) and `tail` in `code/grammars/strace`, and `resumed_line = RESUMED
  inner UNFINISHED RPAREN tail`; the shared result parser accepts exactly
  `= ? <unavailable>` and `dyingResumeTail` exactly ` <unfinished ...>)` then
  `= ?`. The embedded grammar data of `strace-lexer` and `strace-parser` was
  regenerated and the parser's goldens refreshed.
- Policy, in shared code (`dyingPolicy`, `exitInsideCall`): the pid's next line
  must be its own exit; an ignored-class call is dropped; a spawn is dropped and
  no spawn recorded (a child that did start is an `orphan-pid`); a read-only call
  (metadata, listing, an open whose visible flags include `O_RDONLY` and none of
  `O_CREAT`, `O_TRUNC`, `O_WRONLY`, `O_RDWR`, `O_APPEND`, `O_TMPFILE`) is dropped
  only if its thread group made an `exit_group` or its leader was killed, and the
  group's leader was not spawned with `CLONE_VM` (or `vfork`) without an exec
  since (pass 2, `dropDying`/`checkDying`). Writes, network calls, `chdir`,
  `fchdir`, `execve`, `creat`, `openat2` and an open whose flags cannot be read
  stay `unresumed-call`.
- The stricter-only row "ERR token in the text after the close" is not widened.
- `RecorderVersion` 4 -> 5: traces that were refused now parse, so every weak key
  changes.
- KILN04 D35 (section 4.3), the stricter-only table and section 14 say so.

### Added

- `dying_call_test.go`: the three shapes accepted and every near miss refused
  (both readers, via the cross-check hook), the policy table, the CLONE_VM guard,
  the helper layout, and fuzz seeds for `FuzzLineDiff` and `FuzzParseDiff`.
- `TestSandboxStraceDyingThreads` (linux/amd64): a Go fixture whose main goroutine
  calls `os.Exit` while workers are mid-syscall, run under real strace.
