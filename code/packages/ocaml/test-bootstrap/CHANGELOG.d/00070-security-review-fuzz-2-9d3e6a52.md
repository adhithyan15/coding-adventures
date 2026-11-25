# 0.2.2 - fuzz engine security review, round 2 (VER06r)

- The allocation sampler is net of its own cost: `arm` calibrates what one `Gc.counters` call allocates and
  each reading subtracts it, so an honest target that allocates nothing can no longer reach Oom through
  accumulated sampler allocation (5M `tick`s or `reserve 0`s pass under the 64 KiB minimum budget).
- The wait does not trust the non-monotonic wall clock alone: it accumulates the sleeps too (clamped
  observed deltas), so a backward step cannot hang an execution and a forward jump cannot end it early.
  `Fuzz_executor.wait_with` exposes the clock and the sleep as seams for a test.
- The leak counters (per campaign and per process) are raised before the abandon race and lowered if it is
  lost or an exception interrupts the window, so neither is left lost or drifted.
- Tests: blocked workers sleep instead of spinning (no CPU stolen from the timed tests, no stop-the-world
  crawl at the domain ceiling); the global-cap test restores by offset and tolerates concurrently returning
  workers.
