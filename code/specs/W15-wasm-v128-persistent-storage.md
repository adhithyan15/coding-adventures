# W15 — v128 values need persistent storage across an instance's lifetime

## Purpose

Logged as task #79 during SIMD PR1b-3/W14 follow-up work: vendoring
`simd_const.wast` (task #78) surfaced two related, real failures that both
trace back to the same root cause — `WasmExecutionContext::v128_heap`
lives on the PER-CALL execution context, not on the persistent
`WasmInstance`. This spec fixes the storage layer; it does not add any new
SIMD opcode coverage (that stays its own, separate, ongoing effort —
`code/specs/W07-wasm-post-mvp-epics.md`'s epic list). Confirmed via a
direct diagnostic run against the real, pinned-commit corpus as this spec
was being written: `simd_const.wast` currently has exactly 1 `Trap` (a
module with a `(global (mut v128) (v128.const ...))` initializer failing
to instantiate) and 5 collateral `AssertReturn` `Fail`s ("no module
registered as None") cascading from that same trap — i.e. every single
remaining bad outcome in this file traces back to the one bug this spec
fixes, not a separate, unrelated gap.

## The concrete problem, confirmed by direct inspection

### v128 values are handles into a heap that doesn't survive a call

`code/packages/rust/wasm-execution/src/lib.rs:1532`:

```rust
pub v128_heap: Vec<[u8; 16]>
```

on `WasmExecutionContext` (struct at lib.rs:1461-1533). `WasmValue::V128`
(lib.rs:120) is `V128(u32)` — a plain index into this `Vec`, structurally
identical to `WasmValue::Ref`'s GC-heap handle (lib.rs:111, `Ref(Option<u32>)`).
The **only** production construction site for `WasmExecutionContext` is
inside `WasmExecutionEngine::call_function_impl` (the shared implementation
behind the two public entry points `call_function`/`call_function_with_v128`;
struct literal at lib.rs:4703), and it reseeds `v128_heap: vec![[0u8; 16]]`
(inside that same literal, a few lines below `gc_heap: Vec::new()`) fresh on
**every** call — index `0` is a permanently reserved all-zero entry, nothing
else survives.

Meanwhile `WasmInstance` (`code/packages/rust/wasm-runtime/src/lib.rs:
1084-1103`) already has `pub globals: Vec<WasmValue>` (line 1092), and the
engine-building/restoring plumbing shared by both `call()` and
`call_typed()` already round-trips it on every call exactly like memory and
tables: `build_engine` (wasm-runtime/src/lib.rs:1486-1546) clones
`instance.globals` into `WasmEngineConfig.globals` (line 1495) →
`WasmExecutionEngine::new` (wasm-execution/src/lib.rs:4544) stores it as
`self.globals` → `call_function_impl` clones it into `ctx.globals` (part of
the same 4703 struct literal) → runs → writes back `self.globals =
ctx.globals` (lib.rs:5014) → `into_state()` (lib.rs:4608) surfaces it on
`WasmEngineState` → `call_engine`/`call_engine_with_v128` (wasm-runtime/
src/lib.rs:1559 and 1594, both calling `build_engine` then restoring from
`engine.into_state()`) restore `instance.globals = state.globals`.

So a v128-typed global's `WasmValue::V128(handle)` **does** correctly
survive as a numeric value across calls (it's just a `WasmValue`, riding
the same round-trip every other global type gets for free) — but the
handle becomes garbage, because the heap it indexes into does not travel
with it. Concretely: if global `$g` ends call N holding `V128(5)`, call
N+1's fresh `ctx.v128_heap` (length 1, only the reserved zero entry) makes
the first SIMD op that dereferences handle `5` (e.g. `ExtractLane`,
lib.rs:3820-3829) do `ctx.v128_heap.get(5)`, hit `None`, and trap with
`"v128 operand: heap handle out of range"` (lib.rs:3828). (If a future call
happened to also push ≥6 entries before the global is read, it would
instead silently read the *wrong* v128 value at index 5 — the
out-of-bounds trap is a lucky common case, not a structural guarantee
against corruption.)

### `v128.const` in a constant expression has no heap to allocate into at all

`code/packages/rust/wasm-execution/src/lib.rs:790-861`,
`evaluate_const_expr`:

```rust
pub fn evaluate_const_expr(expr: &[u8], globals: &[WasmValue]) -> Result<WasmValue, TrapError>
```

Its match arms (lines 798-856) cover `0x41`/`0x42`/`0x43`/`0x44` (i32/i64/
f32/f64 const), `0x23` (global.get, reading the `globals` parameter
directly), and `0x0B` (end) — six arms total, no `ctx`, no heap of any
kind reachable from the signature. `0xFD` (v128.const's prefix byte) falls
into the catch-all (lines 850-855):
`Err(TrapError::new(format!("illegal opcode 0x{:02X} in constant expression", opcode)))`.
Called from `WasmRuntime::instantiate` for global initializers (wasm-
runtime/src/lib.rs:1282), data-segment offsets (line 1289), and element-
segment offsets (line 1298) — none can pass a heap because none is
available at that point (`WasmExecutionContext` doesn't exist yet;
instantiation runs before any call). Any module with
`(global (mut v128) (v128.const ...))` — real, spec-legal WAT, and present
in the vendored `simd_const.wast` — fails to instantiate at all, a `Trap`
outcome that cascades into ~20 collateral `Fail`s for every subsequent
bare `invoke` targeting that module (confirmed by direct diagnostic run
against the real corpus during task #78/#79's investigation).

### GC refs have the identical unsolved bug — explicitly out of scope here

`WasmValue::Ref`'s GC-heap handle has the exact same "doesn't persist past
`call_function`" limitation (`gc_heap` also lives only on
`WasmExecutionContext`, reseeded per call; `WasmInstance` has no
`gc_heap`/`gc_state` field, confirmed by grep). This is a real,
pre-existing, separate bug — this spec does **not** fix it. Fixing it
would need its own investigation into GC-specific concerns (collection
timing, root-set tracking across calls) that don't apply to v128's
flat, non-collected byte-array storage. Logging as a new backlog item once
this PR lands, not bundled in.

## Design

### `WasmInstance` grows its own persistent `v128_heap`

`code/packages/rust/wasm-runtime/src/lib.rs:1084-1103` gains
`pub v128_heap: Vec<[u8; 16]>`, threaded through the exact same
take-for-the-call/restore-after shape `memory`/`tables`/`globals` already
use — not a new pattern, a fourth instance of an existing one, and (unlike
memory/tables, which are `.take()`n) mirroring `globals`'s `.clone()`
shape specifically, since nothing about calling a function requires
exclusive ownership of the v128 heap the way memory/table mutation does:

- `WasmEngineConfig` (wasm-execution/src/lib.rs:4486-4494) gains a
  `v128_heap: Vec<[u8; 16]>` field; `build_engine` (wasm-runtime/src/
  lib.rs:1486-1546) passes `instance.v128_heap.clone()` into it (mirroring
  `globals: instance.globals.clone()` at line 1495 today).
- `WasmExecutionEngine::new` (lib.rs:4544-4607) stores it as
  `self.v128_heap`, seeding index 0's reserved all-zero entry if the
  cloned `Vec` is ever empty (an instance's very first call) rather than
  unconditionally reseeding — that unconditional reseed is precisely
  today's bug.
- `call_function_impl`'s `WasmExecutionContext` construction (the struct
  literal at lib.rs:4703) takes `self.v128_heap.clone()` into
  `ctx.v128_heap` (not `vec![[0u8; 16]]`) — mirrors `globals:
  self.globals.clone()` in that same literal.
- After the call, `self.v128_heap = ctx.v128_heap` (mirrors
  `self.globals = ctx.globals` at line 5014), surfaced through
  `into_state()` (lib.rs:4608-4626ish) as `WasmEngineState.v128_heap`.
- Both `call_engine` (wasm-runtime/src/lib.rs:1559-1580) and
  `call_engine_with_v128` (lines 1594-1613) — the two functions that each
  call `build_engine` then restore state from `engine.into_state()` — gain
  `instance.v128_heap = state.v128_heap;` alongside their existing
  `instance.memory = state.memory; instance.tables = state.tables;
  instance.globals = state.globals;` restores, **unconditionally, even on
  a trapped call** (both functions already restore regardless of `Err`,
  per the existing comment on `call_engine` at lines 1567-1578 explaining
  why memory/table/global restoration must not be skipped on trap: a trap
  can still have mutated shared state before the trap point, and losing
  that state would itself be a correctness bug independent of this spec).

This alone fixes the v128-global-across-calls case: a global's stored
`V128(5)` handle stays valid because the heap entry at index 5 is the same
`Vec` slot across every call on that instance, not a fresh one.

### `evaluate_const_expr` takes a mutable v128 heap to allocate into

Signature changes from
`evaluate_const_expr(expr: &[u8], globals: &[WasmValue]) -> Result<WasmValue, TrapError>`
to
`evaluate_const_expr(expr: &[u8], globals: &[WasmValue], v128_heap: &mut Vec<[u8; 16]>) -> Result<WasmValue, TrapError>`.
A new `0xFD` arm decodes the SIMD sub-opcode (`0x0C` = `v128.const`, the
only SIMD opcode legal in a constant expression per spec — any other
`0xFD`-prefixed sub-opcode in this position is itself illegal and falls
through to the existing catch-all), reads the 16 literal bytes, pushes
them onto `v128_heap`, and returns `WasmValue::V128(v128_heap.len() - 1)`
— the same "push and return the new index" shape `push_v128` (lib.rs:
3779-3790) already uses inside the main execution loop, not a new pattern.

`WasmRuntime::instantiate` (wasm-runtime/src/lib.rs) constructs the new
`WasmInstance`'s `v128_heap` starting from `vec![[0u8; 16]]` (the reserved
zero entry) **before** evaluating any global initializer / data-segment
offset / element-segment offset, threading `&mut` that same `Vec` through
all three `evaluate_const_expr` call sites (lines 1282, 1289, 1298) in
order, so a later global initializer that references an earlier one via
`global.get` (WASM's spec-legal forward-declared-import-only ordering
rule, already handled by the existing `globals: &[WasmValue]` parameter)
also sees any v128 heap entries the earlier initializer allocated. The
resulting `Vec` becomes the new instance's starting `v128_heap` — the
very same field `build_engine`/`call_engine`/`call_engine_with_v128` will
`.clone()`/restore on every subsequent call, unifying both fixes into one
storage object with one lifecycle.

### Growth is now cumulative over the instance's lifetime, not per-call

`MAX_V128_HEAP_LEN = 1_000_000` (wasm-execution/src/lib.rs:1672) was
previously a per-call bound (irrelevant across calls, since the heap reset
every time); it is now a bound on the **instance's entire cumulative v128
allocation history**, since nothing is ever reclaimed (no GC, no
free-list, matching `v128_heap`'s existing no-reclamation design — this
spec does not add one). This is strictly more protective than before, but
is a real behavior change worth stating plainly: a long-running instance
making many `v128.const`/`splat`-heavy calls will eventually hit the cap
and start trapping on further SIMD allocation, where previously each
call's heap was silently discarded and the cap reset. No vendored corpus
file exercises anywhere near this volume (confirmed: the largest v128
heap any single vendored `.wast` file's grading run produces is under 200
entries), so this is a documented future-scaling note, not a corpus
regression risk today.

## What does NOT change

- `WasmValue::V128`'s representation (still a plain `u32` handle) — no
  opcode handler, typed-stack encoding (`V128_TAG`, `to_typed`/
  `from_typed`), or `default_for` logic changes; they already operate
  purely in terms of "an index into *a* v128 heap," indifferent to which
  `Vec` backs it.
- `push_v128` and every SIMD opcode handler in the main execution loop
  (lib.rs:3779 onward) — they already take `ctx.v128_heap` as a
  parameter/field access; nothing about *which* `Vec` that is changes
  their logic.
- GC-heap (`gc_heap`/`gc_state`) persistence — explicitly out of scope,
  see above.
- `wasm-conformance`'s `run_wast_source`/`Executor` grading logic — this
  is purely a storage-layer fix inside `wasm-execution`/`wasm-runtime`;
  `wasm-conformance` needs no code changes, only a baseline regen once the
  fix lands (`simd_const.wast`'s v128-global and v128.const-in-const-expr
  `assert_return`/`Trap` cases move to real `Pass`/`Fail` instead of
  `Trap`/collateral `Fail`).

## Staged commits

1. This spec (sign-off only).
2. Implementation: `WasmInstance.v128_heap` field + `build_engine`/
   `call_engine`/`call_engine_with_v128`'s clone/restore wiring
   (`wasm-runtime`); `WasmEngineConfig`/`WasmEngineState`
   fields + `WasmExecutionEngine`'s construction/write-back (`wasm-
   execution`); `evaluate_const_expr`'s new signature + `0xFD`/`v128.const`
   arm + the three call sites in `WasmRuntime::instantiate` threading the
   instance's heap through in order. New tests: a v128 global's value
   round-trips correctly across two separate `call_typed` invocations on
   the same instance (the concrete case this spec exists to fix); a module
   with `(global (mut v128) (v128.const ...))` instantiates successfully
   and the global reads back the exact literal bytes; a TEMP-REVERT-CHECK
   proving both are load-bearing (reverting `call_engine`'s restore step
   reproduces the predicted "heap handle out of range" trap on the second
   call; reverting the `0xFD` const-expr arm reproduces the predicted
   "illegal opcode" instantiation trap).
3. Baseline regen against `simd_const.wast` and the rest of the 61-file
   vendored corpus, confirming zero regressions and the specific
   previously-`Trap`/collateral-`Fail` cases now grade for real.

## Verification

- `wasm-execution`/`wasm-runtime` unit tests: a v128 global set inside one
  `call_typed` invocation and read back inside a SECOND, separate
  `call_typed` invocation on the same `WasmInstance` returns the correct
  16 bytes (not a trap, not a wrong value) — the direct regression test
  for the bug this spec fixes.
- A module with a v128-typed global initialized via `(v128.const ...)`
  instantiates successfully (`WasmRuntime::instantiate` returns `Ok`, not
  the current `Trap` from the "illegal opcode" catch-all), and a getter
  function reading that global back returns the exact literal bytes.
- TEMP-REVERT-CHECK on both fixes (see Staged Commits #2) confirms each is
  load-bearing, not incidentally passing.
- `wasm-conformance` baseline regen: `simd_const.wast`'s 1 `Trap` (the
  broken instantiation) and its 5 collateral `AssertReturn` `Fail`s — the
  file's entire remaining bad-outcome count after task #80's hex-float fix
  — move to real graded outcomes; zero regressions anywhere else in the
  61-file vendored corpus (full before/after diff of every file's per-kind
  tally, matching this session's established verification discipline).
- `cargo test -p wasm-execution -p wasm-runtime -p wasm-conformance` and
  `cargo clippy` clean across all three crates.
