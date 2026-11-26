---
category: Testing & coverage
---

# A native benchmark clock needs a real platform smoke run in addition to fake clock tests

The Go microbenchmark collector passed its fake-clock and library tests, but the
real array driver failed on Windows with a zero-duration measured batch. The
local Go 1.26 runtime read shared interrupt time for its monotonic source; its
observed resolution was coarser than the 100-microsecond calibration target.
Crossing a clock tick in calibration did not ensure later batches could be timed.
Increasing the target to one millisecond also failed on this host.

Use QueryPerformanceCounter/QueryPerformanceFrequency for the native Windows
clock, converting elapsed ticks with exact overflow-checked integer arithmetic.
Keep fake-clock tests for collection semantics, and also execute a real native
driver on every supported platform. Check that it produces valid positive pairs
and that the raw report replays; do not require one implementation to be faster.
The repaired driver produced 40 positive pairs at its default calibration target.
