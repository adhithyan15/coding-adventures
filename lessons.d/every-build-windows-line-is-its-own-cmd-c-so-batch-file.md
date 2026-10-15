---
category: Cross-platform & Windows BUILD_windows
---

# Every `BUILD_windows` line is its own `cmd /C`, so batch-file syntax and `set` variables do not carry

`python/window-native`'s `BUILD_windows` failed the first time a PR touched
the package:

    %%i was unexpected at this time.

It was written as if it were a `.bat` file:

    for /f "delims=" %%i in ('python -c "...mkdtemp..."') do set BUILD_IMPORT_DIR=%%i
    mkdir "%BUILD_IMPORT_DIR%\window_native"

The build tool runs each line through `exec.Command("cmd", "/C", line)`
(`internal/executor/executor.go`, `shellCommand`). Two consequences:

1. **Batch-file syntax is wrong at the prompt.** `%%i` is how a `.bat` file
   escapes a `for` variable; typed at `cmd /C` it must be `%i`, and `%%i`
   is a syntax error.
2. **Nothing survives to the next line.** Even with `%i`, the `set` dies
   with its `cmd` process, so the next line's `%BUILD_IMPORT_DIR%` expands
   to the literal text.

It went unnoticed because Windows only builds a package when a PR touches it
(see `build-windows-drifts-from-build-silently-because-almost-no-pr-runs-the.md`).

**Fix used:** move the multi-step logic into one Python script
(`scripts/check_staged_extension.py`) that stages, checks and cleans up in a
single process, and make `BUILD_windows` call it. Same behaviour on every OS,
no shell dialect to get wrong.

**Rule:** a `BUILD_windows` line must stand alone. If two lines need to share
state, they are one program: put it in a script, or join them with `&&` on
one line.
