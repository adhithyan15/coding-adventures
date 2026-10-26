---
category: Testing & coverage
---

# A try/catch recovery path around an engine call that reports failure as {ok:false} never runs

**What went wrong.** The TaskApp web host restored the saved workspace with
`try { engine.load(saved.snapshot) } catch { preserve it; start fresh }`. The
task-wasm accessor's `load` reports a snapshot it cannot read as
`{ ok: false, error }`. It does not throw, like every other engine call. So the
catch never ran, no recovery copy was made, the engine stayed empty, and the
first edit saved over the rejected record. The recovery path had a unit test,
but that test called `preserveRejectedWorkspace` directly, so it passed while
the path it guarded was dead.

**The fix.** Turn `ok: false` into a throw inside the `try`
(`const loaded = engine.load(...); if (loaded?.ok === false) throw ...`). Then
add a boot-level test that runs the real host over the real engine with a bad
record and checks what is stored afterwards. That test failed until the
`ok: false` check went in.

**What to do differently.**

- Before writing `try/catch` around an FFI or engine call, check how that API
  reports failure. In this repo the wasm and C ABI accessors mostly return a
  result object.
- Test a recovery path from its trigger, a real bad input through the real
  call, not by invoking the recovery function on its own. A recovery function
  that is correct but never reached looks exactly like a working one.
