# The same open-ended-dispatch trap applies to native RUNTIME SYMBOLS, not just SIR builtins

Same shape as the `case_eq` lesson above, one layer down. When the precise-GC
track taught `x86_64-backend` to emit `call __twig_gc_write_barrier` after every
`field_store` / `array_set`, `x86-simulator`'s `Simulator::host_call` dispatch
table was not extended to match — its floor is
`Trap::UnresolvedExternal(...)`, so **every** matrix program doing a heap store
trapped. Three LANG-FULL cells sat red on main until an unrelated near-full-repo
rebuild surfaced them (PR #12164).

**Why it stayed hidden for so long**: `x86-simulator`'s BUILD only re-runs when a
diff touches its dependency cone, and the backend crates that emit the symbol
(`x86_64-backend`, `aarch64-backend`) have **no BUILD file at all** — so neither
the emitter's own relocation-assertion tests nor the consumer's execution tests
ran when the emitter changed. A green CI on the PR that added the barrier proved
nothing about either side.

**Lessons:**
- When a backend starts emitting a NEW external runtime symbol, grep every
  *consumer* of that symbol — simulators, runtime shims, linker stubs — before
  assuming it resolves. The emitter-side test (assert the relocation exists) and
  the consumer-side test (assert the symbol resolves) are different tests, and
  passing the first tells you nothing about the second.
- `x86-simulator` is currently the repo's only host-runtime dispatch table (the
  sole `fn host_call` / `Trap::UnresolvedExternal` site) — so it is the one place
  to update, but also the one place with no sibling to cross-check against.
- A **no-op** is sometimes the correct implementation of a runtime hook, not a
  stub: the simulator's bump allocator never collects, so a generational write
  barrier has no remembered set to notify. Say *why* in a comment, or the next
  reader will "finish" it.
- Barrier/GC regression tests are notoriously **vacuous** (see the aarch64 and
  `iir-to-llvm` entries elsewhere in this file — both passed with the barrier
  deleted). Always prove a new guard is load-bearing by temporarily reverting the
  fix and confirming the test goes red.

Discovered: 2026-07-02, while adding trailing-`case` implicit return; a `case`
program printed correctly on Python but panicked on Go/Rust/JS.
