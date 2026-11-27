---
category: Testing & coverage
---

# A Go package test alarm should match the measured whole-package runtime

**What happened:** Kiln's macOS package job failed when Go's default test-process
alarm expired during `go test ./... -cover`. A separate run of the same package
completed in about 438 seconds; the test that was visible when the alarm fired
was short by itself. The evidence identifies a package-level budget problem, but
does not establish why the macOS run took longer than other hosts.

**Fix:** Give the macOS BUILD front a 20-minute Go test-process timeout and
verbose per-test timings. Keep the full package selection, coverage, assertions,
stress rounds, and operation-specific timeouts. The larger outer budget does not
prove a performance cause and is not a benchmark threshold.

**Next time:** Compare the complete package runtime with Go's default test
alarm before changing an individual test. If one test has its own deadline, keep
that bound; changing the package alarm should only accommodate the measured
whole-suite runtime and remain below the CI job limit.
