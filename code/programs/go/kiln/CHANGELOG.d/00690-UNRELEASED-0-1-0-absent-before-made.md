## [0.1.0] - UNRELEASED

### Changed (trace: absent before made, KILN04 D42)

- A call that failed with ENOENT on a final name the action created afterwards
  (the linker that stats its output, then creates it with a plain `O_CREAT`
  open; `mkdir -p _build` after a stat) no longer gives
  `trace-incomplete:unstable-path:final-changed`. The rule (`absentBeforeMade`):
  the name is untracked and never hard-linked, every change-log entry started
  after the lookup completed, the first entry is a creation (clean, or an
  `O_CREAT` open keyed by the -y path) and no removal precedes it. A verdict that
  was stable stays stable. `rename`, `link` and `execve` never feed the rule.
  New census sub-code `absent-before-made`.
- A successful `chmod`, `fchmodat`, `fchmodat2`, `chown`, `fchownat`, `utimensat`,
  `utimes`, `futimesat`, `utime` or `setxattr` by path now uses the open-only rule
  exactly as a successful stat does (the linker's post-link chmod).
- A shell `>/dev/null` (an `O_CREAT|O_TRUNC` open) logs no namespace change for a
  path under `/dev/` that is on the host's ephemeral list, so `/dev/null` no
  longer raises `final-changed` or `own-overlap`.
- `RecorderVersion` 9 to 10: the verdict for these actions changes, so every weak
  key changes.

### Added

- `internal/traced/absent_before_made_test.go`; four fuzzer bursts in
  `model_test.go` (and a model guard for a cwd text that now names a file).
- `TestOpenOnlyDoesNotRelaxOtherCalls`: the chmod case now expects a stable
  record (it encoded the old refusal); the failed-stat and directory negatives
  stay.
