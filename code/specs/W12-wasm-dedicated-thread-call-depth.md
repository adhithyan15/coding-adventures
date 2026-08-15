# W12 — WASM10: run `call_function` on a dedicated thread, raise `MAX_CALL_DEPTH`

## Purpose

`wasm-execution`'s `MAX_CALL_DEPTH` guard (currently **80**, see
`wasm-execution/src/lib.rs:1433` and its own extensive doc comment) exists to
turn unbounded WASM recursion into a clean "call stack exhausted" trap instead
of a real host-process stack overflow (an uncatchable abort). It was bisected
against a **512 KiB** assumed caller thread stack, safe at depth 120, crashing
at 130, with an ~33% margin landing on 80.

That conservative ceiling has a known, already-documented cost: the official
testsuite's `call.wast` has two genuinely bounded (terminating)
mutual-recursion cases — `even(100)` / `odd(200)` — that need more than 80
levels and now correctly-but-unfortunately trap where they used to "pass" only
by relying on the old, unguarded/200-depth version's luck. Confirmed still
true as of this write-up: `wasm-conformance`'s committed baseline
(`tests/fixtures/testsuite-status.json`) shows `call.wast`'s `assert_return` at
`pass: 67, fail: 2` — exactly these two cases.

This spec covers running `call_function` (the top-level, public entry point —
not each individual nested call) on a **dedicated thread with an explicit,
generous stack size**, decoupling the depth ceiling from whatever stack the
*caller* happened to provide. Once execution's real stack size is under this
crate's own control, `MAX_CALL_DEPTH` can be re-bisected and raised with real
safety margin, comfortably clearing 200+.

## What's actually blocking this today

`WasmExecutionContext` (built fresh inside `call_function`, see
`wasm-execution/src/lib.rs:4200-4237`) holds:

- `memory: Option<*mut LinearMemory>`
- `tables: Vec<*mut Table>`
- `host_functions: Vec<Option<Box<dyn HostFunction>>>`

Raw pointers and `Box<dyn HostFunction>` are not `Send` by default (the trait
object requires `HostFunction: Send` to make `Box<dyn HostFunction>: Send`,
which `HostFunction`/`HostInterface` do not declare today), so
`WasmExecutionContext` as a whole is not `Send` and cannot simply be moved into
a spawned thread's closure.

**The tempting fix — adding `Send` bounds to `HostFunction`/`HostInterface` —
is explicitly rejected.** These are public traits with real, breaking-change
consequences for every implementor across the workspace:

- `wasm-runtime`'s WASI syscall functions (`ProcExitFunc`, `FdWriteFunc`, ~9
  others) are already `Send`-clean (its own callbacks already use `Arc<dyn Fn
  + Send + Sync>`) — no cost here.
- `wasm-conformance`'s `RegistryHost`/`CrossModuleFunction` (WASM05, real
  cross-module linking) are built on `Rc<RefCell<WasmInstance>>` — **not**
  `Send`, and not a cheap fix: that crate's documented reentrancy safety net
  is `RefCell`'s runtime borrow panic (see its own module doc comment). Making
  it `Send` means `Arc<Mutex<..>>`, a genuine correctness-sensitive rewrite of
  a crate that has nothing to do with WASM10's actual goal (bigger stack,
  higher depth ceiling).
- Several test-only host functions in `lang-aot`'s and `wasm-execution`'s own
  test suites, trivially `Send`, but not worth forcing a trait-wide bound over.

So this spec's design goal is: **get a bigger, dedicated stack for
`call_function` without touching `HostFunction`/`HostInterface`'s public
signature at all.**

## Design

### One thread per top-level `call_function` call, not per nested call

`call_function_inner` (the free function nested `call`/`call_indirect`/tail
calls recurse through) is untouched — it keeps recursing through the Rust call
stack exactly as it does today. The only change is *which* stack that
recursion happens on: `call_function` (the public, top-level entry point)
spawns **one** dedicated OS thread with a large, explicit `stack_size`, runs
its entire existing loop body (decode → dispatch → tail-call-transition loop,
unchanged) inside that thread, and `.join()`s it synchronously before
returning. Nested calls never spawn additional threads — they get the bigger
stack "for free" because they're already running on it.

(A cross-module host call, WASM05's `CrossModuleFunction::call`, re-enters a
*different* `WasmExecutionEngine`'s own `call_function` — so it recursively
spawns its own dedicated thread, nested inside the first. This is fine: still
fully sequential — a spawn immediately followed by a blocking `.join()` before
any further work happens on the outer thread — so no two threads ever run
concurrently or touch the same data at once, only nested in the sense that OS
thread stacks nest like Rust call frames do.)

### `std::thread::Builder::spawn_scoped`, not `thread::scope`

`std::thread::Scope::spawn` has no `stack_size` parameter. The primitive that
gives both a configurable stack size *and* scoped (non-`'static`) borrowing is
`std::thread::Builder::new().stack_size(N).spawn_scoped(&scope, closure)`
(stable since Rust 1.63) — this avoids needing to make anything `'static` or
`Arc`-wrap engine state that only needs to live for the duration of this one
call.

### Crossing the `Send` boundary honestly: one localized, justified `unsafe impl`

Rather than changing `HostFunction`/`HostInterface`, wrap the *call-local*
non-`Send` payload — the raw memory/table pointers and the `host_functions`
`Vec<Option<Box<dyn HostFunction>>>` moved in via `std::mem::take` (already the
existing pattern at `lib.rs:4209`), plus a raw pointer to `self.vm` — in one
newtype:

```rust
struct AssertSend<T>(T);
// SAFETY: see the safety argument below — this wrapper crosses the spawned
// thread's Send boundary, but the thread is joined synchronously before
// `call_function` does anything else, so the wrapped data is never actually
// accessed from two threads at once.
unsafe impl<T> Send for AssertSend<T> {}
```

**Safety argument** (must be written in full, in-code, next to the `unsafe
impl` — not just in this spec): `Builder::spawn_scoped`'s closure runs on the
new thread; `call_function` calls `.join()` on the returned handle
immediately, with no other work on the calling thread in between. The raw
`*mut LinearMemory`/`*mut Table` pointers, and the host-function trait
objects, are therefore accessed from exactly one thread at a time — the
spawned thread, for the full duration of the call, then nothing, since the
calling thread is blocked in `.join()` the entire time the spawned thread is
alive. This is the same "logically sequential, not actually concurrent"
argument `wasm-conformance`'s `RefCell`-based registry already relies on
(different mechanism, same shape of argument), made explicit here because
`unsafe impl Send` needs its own standalone justification, not an implicit one
borrowed from elsewhere.

### Stack size and the re-bisection

Pick a generous, explicit stack size (starting point: 8 MiB — 16x the 512 KiB
floor the current 80-depth ceiling was measured against) and re-run WASM01's
own bisection methodology directly against it: build a real unbounded
recursive WASM module through `wasm-wast-parser`, run it via the new
dedicated-thread path with the new stack size, and find the real crash floor
in a **debug build** (the profile `cargo test` uses) — do not estimate this by
linear scaling from the old numbers; measure it the same way 80 was measured.
Apply the same ~33% safety margin convention this crate's other guards use.
Document the new `MAX_CALL_DEPTH` value and its measured floor in the same
style as the existing doc comment (which should be rewritten, not just
tweaked, once the numbers change).

**Acceptance test**: `call.wast`'s `even(100)`/`odd(200)` cases (currently the
only 2 `fail`s in that file's `assert_return` baseline) must genuinely pass —
re-run `wasm-conformance`'s baseline and confirm `call.wast` moves to
`pass: 69, fail: 0`, with the new `MAX_CALL_DEPTH` still comfortably above
what those two cases need (not just barely clearing 200).

### What does NOT change

- `HostFunction` / `HostInterface` — no bound added, no signature change.
- `wasm-conformance`'s `RegistryHost`/`CrossModuleFunction` — untouched.
- `wasm-runtime`'s public API — untouched; `call()`/`call_typed()` continue to
  call `WasmExecutionEngine::call_function` exactly as before, the dedicated
  thread is entirely internal to that one function.
- `call_function_inner` — untouched; still a normal recursive free function,
  just now running on a bigger stack.
- Every existing `wasm-execution` unit test that builds an engine directly
  and calls `call_function` — behavior is unchanged from the caller's
  perspective (same signature, same `Result<Vec<WasmValue>, TrapError>`),
  only the stack it happens to run on changes.

## Known trade-offs (deliberately accepted, matching this crate's own existing
convention of naming trade-offs explicitly rather than hiding them)

- **Thread-spawn overhead per top-level call.** `wasm-conformance`'s full
  baseline run invokes `call_function` many thousands of times. OS thread
  creation is not free (typically low tens of microseconds), so the full
  conformance run will get measurably, but not dramatically, slower. Thread
  pooling/reuse is explicitly out of scope for this first slice — ship the
  correctness win, revisit performance only if the measured regression is
  actually disruptive (measure after implementing, don't pre-optimize).
- **The `unsafe impl Send` wrapper is a real, if narrow, unsafe-code
  surface.** It must ship with the explicit safety argument above written
  in-code, and — per this repo's `CLAUDE.md`/session convention — proven via
  a TEMP-REVERT-CHECK-style regression test that actually exercises deep
  recursion through the new path (not just a smoke test that the wrapper
  compiles).
- **A cross-module host call now nests OS threads, not just Rust stack
  frames.** Each nesting level costs a full 8 MiB (or whatever the final
  chosen size is) of address space, not just a shallow Rust frame. This is
  bounded by the same `MAX_CALL_DEPTH` ceiling that already bounds Rust-stack
  nesting today, so it does not introduce a new unbounded-resource risk, but
  it is a real, worth-naming shift in what "one level of WASM call nesting"
  costs.

## Non-goals

- Real multi-threaded/concurrent WASM execution (shared-memory threads,
  `memory.atomic.wait`/`notify`) — a separate, much larger architectural
  question (see `W07-wasm-post-mvp-epics.md`'s Epic 2), explicitly *informed
  by* this spec's resolution but not attempted here. This spec's dedicated
  thread is purely an implementation detail for stack-size control; from the
  outside, `call_function` remains synchronous, single-threaded-in-effect
  (blocking join), single-result semantics, identical to today.
- Thread pooling / reuse for performance.
- Changing `MAX_CALL_DEPTH`'s fundamental purpose (it still exists specifically
  to catch genuinely unbounded recursion, e.g. `fac.wast`'s and
  `call.wast`'s own deliberately-unbounded cases, which must still trap
  cleanly at *some* ceiling — this spec raises that ceiling with real margin,
  it does not remove it).

## Verification

- `cargo test -p wasm-execution` — all existing tests continue to pass
  unmodified (same public API, same synchronous behavior).
- New test(s) proving: (a) a WASM call recursing past the *old* 80-depth
  ceiling but under the *new* one now completes successfully (the concrete
  `even(100)`/`odd(200)` shape, or equivalent), (b) a genuinely unbounded
  recursive call still traps cleanly with "call stack exhausted" at the new
  ceiling, not a host crash — TEMP-REVERT-CHECK this specifically: temporarily
  lower the stack size back to something small with the new higher
  `MAX_CALL_DEPTH` still in place, confirm it actually panics/aborts without
  the dedicated-thread stack backing it, then confirm the real shipped
  combination doesn't.
- Full `wasm-conformance` baseline regen: `call.wast` moves from `pass: 67,
  fail: 2` to `pass: 69, fail: 0`; zero regressions anywhere else in the
  baseline (byte-for-byte diff against the prior committed manifest outside
  of that one file's numbers).
- `cargo clippy -p wasm-execution --all-targets -- -D warnings` clean,
  including on the new `unsafe` block (clippy's `undocumented_unsafe_blocks`-
  style scrutiny — the safety comment must be real and specific, not
  boilerplate).
