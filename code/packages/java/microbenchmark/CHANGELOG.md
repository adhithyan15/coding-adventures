# Changelog

## 0.1.0

- Add paired benchmark collection, exact analysis, JSON replay, and array CLI.
- Accept generic workload labels containing a non-NUL character outside ASCII whitespace; retain U+2003 and other Unicode whitespace during replay.
- Run the installed-jar consumer and 40-pair report replay through a POSIX helper from `BUILD`, while keeping the PowerShell helper for `BUILD_windows`.
- Declare the clock, replay-file, and standard-stream capabilities; enforce benchmark pair and replay-input bounds before analysis.
- Reject NUL in all metadata strings and invocation arguments while retaining empty argument strings.
