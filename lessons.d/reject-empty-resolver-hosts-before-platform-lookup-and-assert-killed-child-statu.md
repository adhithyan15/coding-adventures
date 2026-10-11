---
category: Cross-platform & Windows BUILD_windows
---

# Reject empty resolver hosts before platform lookup and assert killed child status per OS

Closure CV04a run 38169475256 completed Windows Closurec but failed unchanged
Chief tests. Windows resolves an empty host instead of rejecting it, so enforce
`InvalidInput` at the resolver boundary before the OS call. A killed Windows
child has termination code 1; waiting for Unix's signal status `None` times out
after the child was already reaped. Preserve exact per-OS status, timeout error,
and no-process-ID assertions without increasing any production or test bound.

The same run's Linux failure was hosted-runner communication loss with an
incomplete package step. Classify that separately from package assertions and
require a complete new native CI result; partial output is not acceptance.

