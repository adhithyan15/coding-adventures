---
category: Cross-platform & Windows BUILD_windows
---

# Portable kiln tests that use POSIX paths or strace fixtures must skip on Windows

D32 (#19641) added kiln tests with no build tag, so the `build (windows-2025)` job ran them and two failed. A test is "portable" only in the sense that it needs no strace binary; its data can still be Linux-shaped. `TestTraceRefusalPath` builds `/tmp/...` paths and expects `filepath.IsAbs` to accept them, which it does not on Windows. `TestParseHelperTraceReportsTheRefusedLine` feeds a strace fixture through cwd resolution, which refuses it at `initial-cwd` before reaching the intended line.

Fix: `if runtime.GOOS == "windows" { t.Skip(reason) }` in those tests only (not a `!windows` file tag, which would hide shared helpers). Next time: any kiln test that hardcodes a POSIX absolute path or parses a strace fixture through cwd resolution gets the guard when it is written; the Windows CI build runs kiln's `go test`, and `GOOS=windows go vet` cannot catch this.
