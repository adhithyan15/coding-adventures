---
category: Testing & coverage
---

# A test that stops a loop from another thread after a fixed sleep hangs when run() starts late and resets the stop flag

A scheduled full build shard ran for 2 hours 40 minutes and died at the 150-minute step
timeout, with `dotnet` processes orphaned and no failing package named. Cause:
`fsharp/event-loop`'s tests start a thread that does `Thread.Sleep(10); loop.Stop()` while
the main thread calls `loop.Run()`. `Run()` clears the stop flag when it starts, so if
the main thread reaches `Run()` more than 10 ms late (a loaded CI runner doing JIT while
other builds share the cores) the stop lands first, is erased, and the loop spins on
`Thread.Yield()` forever. The C# and Python ports had the identical test shape.

Reproduce it deterministically instead of waiting for a slow runner: inject a delay
before `Run()` (100 ms was enough) and the old tests hang under a `timeout` guard.

Fix: the stopping thread must wait for proof that the loop is running, never for time.
Add an event source whose `Poll()` sets a `ManualResetEventSlim` / `threading.Event`, and
have the stopper wait on it (with a generous timeout and a message) before stopping.
Then re-run the fixed tests with the same injected delay: they pass.

Related: a test that registers into a process-global registry (`cas-pretty-printer`'s head
formatters) must use a name no other test uses, because Rust runs tests on parallel
threads. A shared `Foo` made `mathematica_user_function_camel` fail about 3% of the time
(13 of 400 runs); a unique name made it 0 of 800.

Also in F#: `TimeSpan.FromSeconds(30)` inside a lambda is an ambiguous overload on newer
compilers (FS0041); pass an int millisecond timeout (`Wait(30000)`) instead.

C's `event-loop` test (`stopper_thread` naps 5 ms) has the same shape and is not yet fixed.
