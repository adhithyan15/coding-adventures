## 2026-11-02 — `windows-posix-syntax`: BUILD_windows lines must be cmd syntax

- **New `-validate-build-files` rule, `windows-posix-syntax`**
  (`internal/validator/windows_posix_syntax.go`). Every `BUILD_windows` line
  runs as its own `cmd /C`, so the rule rejects POSIX-only constructs outside
  double quotes: `/dev/null`, `command -v`, `$(...)`, `${...}`, `[ ... ]` /
  `[[ ... ]]` tests, and `then`/`fi`/`do`/`done`/`esac` after a `;`. Each
  diagnostic names the file, line, construct and the cmd alternative.
- **Why.** `where swift >/dev/null 2>/dev/null && swift test || echo skipping`
  never fails under cmd: the redirect cannot open `/dev/null`, so the line
  always takes the echo. 49 Swift `BUILD_windows` files skipped their tests on
  Windows that way. Three Perl files and one Python file used `command -v`,
  `$(...)` and `if [ ... ]; then`, which cmd rejects outright, but only when a
  PR happened to rebuild them there. On main the rule's first run found 56
  such lines.
- **False positives.** Double-quoted text belongs to the program it is passed
  to, so `python -c "...; ..."`, `perl -e "... ${x} ..."` and a
  `bash -c "..."` hand-off payload are never read. cmd's own
  `for %i in (...) do ...` has no `;` before `do`, and `echo done` is not a
  keyword after `;`. `#` and `rem` lines are skipped, and so are Starlark
  `BUILD_windows` files (six Go packages, where `[` is a list). Single quotes
  are not quotes to cmd, so single-quoted text is checked.
- Lesson: `lessons.d/build-windows-lines-run-under-cmd-c-so-a-posix-redirect-like.md`.
