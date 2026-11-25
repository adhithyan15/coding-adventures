# Changelog

## 0.1.0

- Add paired collection, portable analysis, report replay, and indexed-array CLI.
- Accept generic workload labels containing a non-NUL character outside ASCII whitespace; retain U+2003 and other Unicode whitespace during replay.
- Run the installed-jar consumer and 40-pair report replay through a POSIX helper from `BUILD`, while keeping the PowerShell helper for `BUILD_windows`.
