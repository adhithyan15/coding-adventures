---
category: Cross-platform & Windows BUILD_windows
---

# PowerShell passes native file wildcards literally to gofmt

Running `gofmt -w *.go` from PowerShell passed the literal string `*.go` to
gofmt, which failed with an invalid filename while subsequent tests still ran.
The tests passing did not prove formatting had succeeded. Pass explicit filenames
or obtain paths with a native PowerShell enumeration and pass the resulting array.
Verify formatting separately before committing. The same literal-wildcard issue
applies to native search-tool path arguments; use their own glob filters instead.
