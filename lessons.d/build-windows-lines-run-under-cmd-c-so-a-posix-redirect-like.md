---
category: Cross-platform & Windows BUILD_windows
---

# BUILD_windows lines run under cmd /C, so a POSIX redirect like >/dev/null silently turns a guarded test into a skip

Forty-nine Swift `BUILD_windows` files (47 packages, 2 programs) read

    where swift >/dev/null 2>/dev/null && swift test || echo Swift not available on this runner - skipping

The build tool runs each `BUILD_windows` line as its own `cmd /C`
(`executor.shellCommandForOS`). `cmd` cannot open `/dev/null` (there is no
`\dev` directory), so the redirect fails and `where` never runs. The `||`
then takes the `echo` branch. The line exits 0 and the job is green, but
`swift test` never ran on Windows, including on runners where CI had just
installed Swift with winget.

Loud POSIX syntax (`command -v`, `if [ ... ]; then`, which fail with "-v was
unexpected at this time") at least breaks when a PR rebuilds the package. A
redirect inside a `guard && run || echo skip` line never breaks, so nobody
notices.

**Fix:** use `cmd` syntax, `where swift >nul 2>nul && swift test || echo ...`,
and let the build tool check it. `-validate-build-files` now has a
`windows-posix-syntax` rule
(`internal/validator/windows_posix_syntax.go`). It rejects `/dev/null`,
`command -v`, `$(...)`, `${...}`, `[ ... ]` tests and `; then|fi|do|done|esac`
anywhere outside double quotes in a `BUILD_windows` line. A `bash -c "..."`
payload is double-quoted, so it stays allowed. On main its first run found 56
broken lines: 50 Swift, 3 Perl and 3 Python. It also flagged six lines in
Starlark Go `BUILD_windows` files, where a `[` is a list, so the rule now
skips Starlark files.

**Rule:** any `guard && run || echo skip` line is a test that can pass without
running. When you write one for Windows, check the guard under `cmd` itself.
If a step needs POSIX logic, run it as `bash -c "..."`, or move it into a
script run by the package's own interpreter (see
`python/audio-device-sink/scripts/check_staged_extension.py`).
