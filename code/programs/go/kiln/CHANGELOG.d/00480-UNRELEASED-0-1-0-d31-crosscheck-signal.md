## [0.1.0] - UNRELEASED

### Fixed (trace cross-check, D31)

- The cross-check's "too many arguments and a later error" row (KILN04 4.9)
  was defeated when a same-pid signal line (`--- SIGCHLD ... ---`) sat between
  the halves of a split call: the row rebuilt the glued call from earlier lines
  and forgot the pending prefix at the signal line, so a legitimate
  stricter-only divergence (old `arity`, grammar `unparsable-line`) was reported
  as outside the list. Signal lines now leave the rebuilt pending prefix alone,
  exactly as the old scanner does; exit lines still end the pid's pending call.
  The change only widens recognition of an already-listed shape; fail-open
  detection is untouched. Regression tests in `signal_glue_test.go`.
