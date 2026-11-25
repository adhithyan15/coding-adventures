# 0.2.1 - fuzz engine security review fixes (VER06r)

- The handle's per-call path is allocation-free (no closure, option or tuple), and `Gc.counters` is read
  on every 64th `tick`/`reserve` only: an honest target was charged about 150 bytes per `tick` and got a
  false `Oom` at about 1.77M ticks. 5M `tick`s, `reserve`s and `edge`s now pass under 256 MiB.
- The deadline is a wall clock (`Unix.gettimeofday`) and the waiter sleeps (`Unix.sleepf`, 50 us to 1 ms)
  instead of reading process CPU time: leaked spinners and the waiter itself no longer cause false timeouts.
  The library now depends on `unix` (ships with the compiler); the manifest adds `time:sleep`.
- A process-wide cap of 16 leaked worker domains (a returning worker subtracts itself) next to the per-campaign
  cap of 4; at the cap, and when `Domain.spawn` fails, the executor raises the typed `Error.Engine_error`.
- A `Sys.Break` while waiting retires the handle first. `mem_budget` below 64 KiB is rejected by validation.
- Tests: an honest sleeper and a busy worker pass with leaked spinners present; the global cap, the
  domain ceiling and the subtraction on return; the hot-path test fails if the sampling is made per call.
