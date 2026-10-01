---
category: Cross-platform & Windows BUILD_windows
---

# Unix-only syscalls in Go tests need a platform build constraint

PR #16232's Windows CI reached the Go build-tool test package and failed to
compile `internal/executor/b07probe_test.go`: `syscall.Mkfifo` is undefined on
Windows. A runtime skip cannot help because Go compiles the test before it can
run. Keep the FIFO probe in a `//go:build !windows` test file and leave the
symlink and package-walk probes in the common file. Verify the executor suite
on Windows and cross-compile its test binary for Linux to retain the FIFO case.
