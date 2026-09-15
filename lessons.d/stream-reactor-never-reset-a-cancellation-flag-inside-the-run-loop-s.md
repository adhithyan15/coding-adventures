# stream-reactor — never reset a cancellation flag inside the run loop's own entry

**Date:** 2026-06-14

**`StreamReactor::serve()` must NOT reset the stop flag — a `stop()` racing serve startup gets silently swallowed.** The reactor's `serve()` used to do `self.stop_flag.store(false)` at the top. The flag is already `false` at construction, so the reset was redundant on first serve — but it created a race: an FFI binding (JNI/N-API/etc.) that flips a "running" flag and lets the caller request `stop()` *before* the background serve thread enters the loop would have its stop erased, so `serve()` runs forever and `join()` hangs. This was invisible in Python/Ruby because their tests wait for the server to actually be listening before stopping (and Python/Ruby `join` with a timeout); it surfaced in the JVM binding, whose JUnit tests flip running→stop synchronously in one thread, and whose native `join()` has no timeout. **Rule:** a `stop()` request must never be lost — don't reset stop/cancellation flags inside the run loop's own entry. Reset (re-arm) only via an explicit separate operation if a consumer truly needs to re-run. Reproduce FFI hangs single-threaded in pure Rust (spawn serve, `stop()` immediately, `join()`) before blaming the binding; use `sample <pid>`/jstack to see the native thread stuck in `kevent`.

---
