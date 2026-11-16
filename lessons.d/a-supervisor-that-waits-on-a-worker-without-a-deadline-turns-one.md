---
category: Rust
---

# A supervisor that waits on a worker without a deadline turns one stuck call into a hung job

The generated Warrant evidence harness runs every `heap`/`output`-bounded adapter call on a worker thread inside an allocation window, and the supervisor (the test thread) waited for the reply with `Condvar::wait` in a loop and `lock().unwrap()` on the reply slot. On 2026-11-14 the Windows CI run of `rust/lzw` (whose evidence target runs every one of its 16.8 million instances under that observer) idled for 4h47m until the job limit killed it. The log could not say which call hung (the build tool buffers output; BT-TIMEOUT fixes that half), so the root cause is not proven; the supervisor had four ways to wait for ever: a worker that spins or deadlocks without allocating (the observer only trips on allocation), a worker thread that died, a reply-slot write skipped because the slot was poisoned (the allocator-side trip wrote only on `Ok(lock)`), and a poisoned `lock().unwrap()` panicking the supervisor instead of failing the instance.

Fix (WD-HARN1): every wait is `wait_timeout` against one deadline (120 s) and fails the instance with `observer-timeout`, abandoning the worker; abandoned workers are never joined and the generated `main` leaves through `process::exit`; every `Mutex` is taken through `lock_or_recover` (`unwrap_or_else(|e| e.into_inner())`). A runtime test compiles the generated harness against a subject that stalls and panics and requires the run to finish in bounded time.

Do differently: any thread handoff in generated or test code needs a deadline on every wait, a reply path that cannot be skipped by poisoning, and no join on a thread you have given up on; and when a hang has no log, harden every unbounded wait rather than guess which one fired.
