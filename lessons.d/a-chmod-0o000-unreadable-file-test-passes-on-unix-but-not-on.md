---
category: Cross-platform & Windows BUILD_windows
---

# A chmod 0o000 unreadable-file test passes on Unix but not on Windows, where os.Getuid() is -1

What happened: PR #19222's Windows job (`build (windows-2025)`) failed in
`TestReadVectorFile` with `key_test.go:392: unreadable file accepted`. The test
wrote a file with `os.WriteFile(q, ..., 0o000)` and expected `ReadVectorFile`
to fail to open it. On Linux and macOS the mode bits make the file unreadable
to a non-root user, so the test passed there.

Why: Windows has no Unix permission bits. The only mode bit it honours is the
read-only attribute, so `0o000` leaves the file readable and the open succeeds.
The guard `os.Getuid() != 0` did not skip the case either, because
`os.Getuid()` returns -1 on Windows, which is not 0. The test therefore ran
and asserted behaviour the platform cannot provide.

What to do differently: guard Unix-only permission tests with
`runtime.GOOS != "windows"` as well as the root check, and add a one-line
comment saying why. Before relying on mode bits in a test, check what the
platform actually enforces. Run the test on every OS in the CI matrix, or
cross-check with `GOOS=windows go vet`, since a green Linux run says nothing
about Windows permissions.
