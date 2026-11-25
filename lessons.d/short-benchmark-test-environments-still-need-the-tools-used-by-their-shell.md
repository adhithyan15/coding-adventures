---
category: Cross-platform & Windows BUILD_windows
---

# Short benchmark test environments still need the tools used by their shell fixtures

The runner's full Windows tests initially failed in command-manifest summaries
and build logs after the test environment was shortened to Go, Windows and Git's
command directory. Both existing fixtures execute `printf`, supplied by Git's
`usr/bin`, which the new environment had omitted. Adding that directory made
the complete suite pass without changing production or fixture behavior.

Keep child PATH short enough for cmd.exe, but derive its contents from every
tool a test actually invokes. A short environment is not automatically a complete
one. Invoke native Go arguments through a structured argument list when collecting
coverage, rather than relying on PowerShell's parsing of dotted option values.
