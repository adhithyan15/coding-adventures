---
category: Rust
---

# A fake-clock callback shares mutable state with the clock closure



When a test passes both a mutable fake clock and a callback that mutates the
same counter, Rust correctly rejects two simultaneous mutable borrows. Store
the counter in `Cell`/`RefCell` or model the clock and callback with one shared
test harness so both closures can advance the same deterministic timeline.
Do not weaken the production signature or use unsafe to satisfy the test.

This appeared while testing calibrated empty guest calls: the timer reads at
the start/end of each batch while the guest callback also advances fake time.
