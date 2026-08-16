# Changelog

All notable changes to this package will be documented in this file.

## [0.6.3] — 2026-08-16 (task #100 — instantiate() requires a validated module)

### Changed (breaking)

- `WasmRuntime::instantiate()` now takes `&ValidatedModule` instead of
  `&WasmModule`. This crate's own `ValidatedModule` doc comment always
  documented the intent that "downstream code (the runtime) can accept
  `ValidatedModule` instead of `WasmModule` to ensure validation is
  never accidentally skipped", but `instantiate()` never actually
  enforced it: it took a plain `&WasmModule` and never called
  `validate()` itself, so every `validate()` check -- including the
  memory/table allocation caps added for task #96's security review --
  was silently bypassable by any caller who called `instantiate()`
  directly instead of going through `WasmRuntime::validate()` first.
  Callers now call `validate()` (or `load_and_run()`, fixed to actually
  thread its own `validate()` result through instead of discarding it
  and re-passing the raw module) and pass the resulting
  `ValidatedModule` -- the guarantee is now a compile-time fact instead
  of a caller convention.
- Confined blast radius: outside this crate's own test suite, only
  `wasm-conformance`'s harness calls `instantiate()`, and it already
  called `validate()` first (it just threw away the `ValidatedModule`
  and re-passed `&validated.module` -- trivially updated to pass
  `&validated` instead). No other crate in the workspace calls
  `instantiate()` directly.

### Security

- Found via `/security-review` as a follow-up to task #96's memory/
  table allocation caps: those caps (and every other `validate()`
  check) were bypassable by any embedder calling `instantiate()`
  directly. Closed by making `ValidatedModule` the only way to reach
  `instantiate()` at all.

## [0.6.2] — 2026-08-15 (W16, task #85 — multi-memory first slice)

### Changed (breaking)

- `WasmInstance.memory: Option<LinearMemory>` is now `memories:
  Vec<LinearMemory>`. Import resolution now accumulates (`memories.
  push(...)`) instead of overwriting, so a module importing more than one
  memory keeps all of them instead of silently retaining only the last.
  `instantiate()` allocates every entry in `module.memories`, not just
  `module.memories[0]`.
- `build_engine`/`call_engine`/`call_engine_with_v128` thread the
  `Vec<LinearMemory>` through via the same unconditional-even-on-trap
  restore discipline the singular field already used.
- Data-segment application still only ever targets memory 0 regardless of
  `seg.memory_index` -- a deliberate scope boundary, not an oversight; see
  `code/specs/W16-wasm-multi-memory-first-slice.md`'s "What does NOT
  change".

### Fixed

- `RegistryHost::resolve_memory` (`wasm-conformance`) discarded the
  resolved export's memory INDEX and always cloned "the" single memory --
  harmless before this change (an instance had at most one memory), but a
  real latent bug once an exporting instance can have more than one:
  importing memory export #1 from a 2-memory module would have silently
  returned memory #0 instead. Fixed alongside this crate's own change
  since it shares the same root field.

See `code/specs/W16-wasm-multi-memory-first-slice.md` for the full design.

## [0.6.1] — 2026-08-15 (W15, task #79 — v128 persistent storage)

### Fixed (breaking)

- `WasmInstance` gains a `pub v128_heap: Vec<[u8; 16]>` field -- the
  instance's own persistent v128 (SIMD) value storage, replacing the old
  per-call-only `wasm_execution::WasmExecutionContext::v128_heap`. A
  v128-typed global's `WasmValue::V128(handle)` used to go stale the
  moment one `call`/`call_typed` invocation ended (the heap it indexed
  into was thrown away and rebuilt fresh every call); it now survives
  across separate invocations on the same instance, exactly like
  `globals`/`memory`/`tables` already do. `instantiate()` builds this
  field up directly (starting from the reserved all-zero entry) as it
  evaluates global/data/element initializers, so a `v128.const` inside
  one of those (previously a hard instantiation failure -- see the
  companion `wasm-execution` 0.9.2 release) now allocates straight into
  the instance's own long-lived heap.
- `build_engine`/`call_engine`/`call_engine_with_v128` thread
  `v128_heap` through the exact clone/restore shape `globals` already
  uses (`build_engine` calls the new
  `WasmExecutionEngine::set_v128_heap`; both `call_engine` variants
  restore `instance.v128_heap = state.v128_heap` after the call,
  unconditionally, matching the existing regardless-of-trap discipline
  documented on `call_engine` itself).
- This is a breaking change to `WasmInstance`'s public field list -- the
  one hand-built test construction site in this crate's own integration
  tests was updated in the same PR; per this repo's stated preference
  (break compatibility freely, no back-compat shims), no deprecated
  alias or default was added.

See `code/specs/W15-wasm-v128-persistent-storage.md` for the full design
and motivating corpus evidence.

## [0.6.0] — 2026-08-15 (SIMD PR1b-1 — call_typed_with_v128, real v128 results end to end)

### Added

- `WasmRuntime::call_typed_with_v128(&mut WasmInstance, name, args) ->
  Result<(Vec<WasmValue>, Vec<Option<wasm_execution::V128Bytes>>),
  TrapError>` — the host-facing sibling of `call_typed` for functions that
  return real v128 values. Thin-wraps `wasm-execution` 0.9.0's new
  `WasmExecutionEngine::call_function_with_v128`, resolving each `V128`
  result to its actual 16 bytes rather than leaving it as an
  already-meaningless handle once the engine's internal context has been
  torn down.
- Internal refactor to support this without duplicating the run+restore
  bookkeeping: the engine-construction half of `call_engine` (memory/
  tables/host-function ownership transfer into a fresh
  `WasmExecutionEngine`) is extracted into a private `build_engine`
  helper, shared by both the existing `call_engine` (unchanged behavior,
  confirmed via the full existing test suite) and a new sibling
  `call_engine_with_v128`.

### Why `wasm_wast_parser` couldn't test this directly

`wasm-wast-parser` doesn't yet support `v128.const`'s text literal syntax
(deferred to SIMD PR1b-2), so this crate's new integration test
(`tests/call_typed_with_v128.rs`) hand-constructs a `WasmInstance`
directly with raw SIMD bytecode rather than going through
`wasm_wast_parser::parse_module` — every `WasmInstance` field is already
public, so this needed no new test-only surface.

### Added

- `call()`'s existing lossy i64-round-trip conversions (both directions)
  gained a `ValueType::V128`/`WasmValue::V128` arm, matching the
  established pattern for reference types: a deterministic, non-panicking
  placeholder (handle `0`, the reserved all-zero v128), not a real
  conversion — `call()`'s own `i64`-only signature cannot represent a
  128-bit value at all. `call_typed()` should be used for real v128
  arguments/results, same guidance the existing `Ref` comment already
  gives.

## [0.5.3] — 2026-08-15 (WASM05 — real instantiate() link-failure path)

### Changed (breaking behavior, deliberate)

- `instantiate` now returns a real `Err(TrapError)` when any import
  can't be resolved by the host, or resolves to something whose actual
  type doesn't satisfy the module's declared import type. Previously it
  never failed on an import at all: an unresolved function got pushed
  as `None` (failing later, at *call* time, only if that specific
  import was ever invoked), and an unresolved memory/table/global
  silently fabricated a default value from the *declared* type instead
  of erroring. See `code/specs/W10-wasm-real-linking-and-unlinkable.md`.
- Function imports are now checked against `HostFunction::func_type()`;
  memory/table imports are checked via the real spec's limits-
  compatibility rule (actual min ≥ declared min; if declared has a max,
  actual must too, and not exceed it) — `Table` doesn't track its
  declared element type at runtime, so table element-type mismatches
  aren't caught here (a real, named limitation, not silently ignored;
  every table this repo can currently construct is funcref anyway, so
  this doesn't lose real coverage against the vendored corpus).
- **Verified safe for existing real callers**: confirmed by reading
  `WasiEnv::resolve_function` directly that it never actually returns
  `None` for its own module — every unimplemented WASI function falls
  through to a real `EnosysFunc` stub, not `None`. So
  `brainfuck-wasm-compiler`/`nib-wasm-compiler`/`twig-to-wasm`/
  `twig-demo`/`lang-aot`'s existing WASI-based execution paths cannot
  regress from this change; confirmed empirically too, full workspace +
  downstream consumer test suite unchanged.
- 12 new tests covering unresolved/type-mismatched/compatible imports
  for each of function, memory, table, and global.

## [0.5.2] — 2026-08-15 (WASM17 — exhaustive-match fix for Funcref/Externref)

### Fixed

- `call()`'s param-type-to-`WasmValue` conversion match was non-exhaustive
  after `wasm-types` 0.1.1 added `ValueType::Funcref`/`Externref` (a
  compile error, not a behavior change). Added both to the same lossy
  "pass the raw i64 as a null-pointer sentinel" arm the existing GC
  reference types (`Anyref`/`I31ref`/`StructRef`) already use -- this is
  `call()`'s pre-existing legacy behavior; `call_typed()` should be used
  instead when a real `WasmValue::Ref` needs to be passed. No behavior
  change for any existing `ValueType`.

## [0.5.1] — 2026-08-13 (WASM07 — a trapped call must not lose an instance's state)

`call_engine` (shared by `call()` and `call_typed()`) temporarily
`take()`s `instance.memory`/`mem::take`s `instance.tables`/
`instance.host_functions` into a fresh `WasmExecutionEngine` for the
duration of one call, then writes the engine's post-call state back onto
`instance`. That write-back used `let results =
engine.call_function(func_index, wasm_args)?;` — the `?` early-returns on
ANY trap, before the write-back lines ever run. Since the fields were
already taken, `instance.memory` was left `None` (and `instance.tables`/
`instance.host_functions` left empty) **forever after** — not just for
that one trapped call, but for every subsequent call on the same
instance, since nothing else ever puts them back.

This is exactly the shape of `wasm-conformance`'s own module-registry
model: a script's module registry holds one `WasmInstance` per `(module
...)` directive and runs every `invoke`/`assert_*` against it in order.
The moment ANY of those directives trapped for any reason — an
intentionally-trapping `assert_trap`, or a genuine bug in an unrelated
function — every LATER directive against that same module silently and
permanently failed with a spurious "no memory available"/"undefined
table", masking whatever those later directives were actually checking.
This affected dozens of real testsuite cases across
`load.wast`/`local_tee.wast`/`nop.wast`/`memory_trap.wast`/`call.wast`
(their common `as-call_indirect-*`/`as-load-*`/`as-store-*` cases always
follow an earlier intentionally-trapping case in the same module).

Fixed by capturing the `Result` instead of `?`-ing it immediately, always
restoring `instance`'s fields from `engine.into_state()` (safe regardless
of whether the call trapped — `call_function` takes `&mut self`, so
`engine`, and everything moved into it, is fully intact either way), and
only then returning the captured result.

Also wires the module's real type section into the engine
(`engine.set_type_section(instance.module.types.clone())`) so
`call_indirect` gets real type-checking — see `wasm-execution` 0.6.3's
own changelog for why that was a separate, necessary fix and not
something this crate could paper over on its own.

2 new regression tests (`tests/wasm07_regression.rs`): memory and a
table each independently confirmed to survive an earlier trapped call on
the same instance and remain usable by a later one.

## [0.5.0] — 2026-08-13

### Added — `WasmRuntime::call_typed`, a bit-exact sibling of `call()` (W05 PR-4)

`call()` is the crate's only public execution entry point, and it round-trips
every argument and result through `i64` — lossy for floats. Its result
conversion does `WasmValue::F32(v) => *v as i64` / `F64(v) => *v as i64`, a
numeric *truncation* (Rust's `as` cast), not a bit reinterpretation: a
`3.5f64` result comes back as `3i64`, and a NaN's payload/sign bits are not
preserved at all. This is fine for `call()`'s existing callers (integer-only
WASI/Lisp-value-model workloads), but it means `call()` cannot support
anything that needs the *exact* result the interpreter produced — most
directly, a conformance harness grading the official testsuite's
`assert_return` directives, some of which assert an exact
`nan:0x<payload>` bit pattern.

`call_typed(&self, instance: &mut WasmInstance, name: &str, args: &[WasmValue]) -> Result<Vec<WasmValue>, TrapError>`
is a purely additive sibling: same export-lookup and engine-execution
plumbing as `call()` (now factored into a shared private `call_engine`
helper so neither duplicates the memory/tables/host-functions ownership
transfer and WasmGC struct-field-count wiring), but it takes and returns
typed `WasmValue`s directly, with no `i64` round trip at all. `call()`
itself, its behavior, and its existing callers/tests are unchanged — this
refactor was verified against the existing WASI Tier 3 test suite (17
tests, all still passing) before and after.

New tests in `tests/call_typed.rs` empirically confirm the bug `call_typed`
fixes: one asserts `call()` really does truncate `3.5` to `3`, and a
sibling assertion on the same call confirms `call_typed` returns the exact
`f64` bits instead; another constructs a NaN with a specific,
non-canonical payload via `f64.reinterpret_i64` and asserts `call_typed`'s
result preserves that exact bit pattern, not just "is NaN".

## [0.4.0] — 2026-07-13

### Fixed — struct field counts indexed by deduplicated function-type count (LANG-FULL E6d-5)

`WasmRuntime::call` registered WasmGC struct field counts by padding the front of
the `struct_field_counts` vec with one filler slot per **function** (`instance.
func_types.len()`) before appending the struct counts. But function and struct
types share one wasm type-index space and the encoder **deduplicates** function
types, so the per-function count over-counts whenever two functions share a
signature — and the struct's field-count entry then landed at a type index higher
than the one the emitted `struct.new`/`struct.set` actually reference, leaving the
real struct index registered as a zero-field filler.

The symptom: any module whose functions include duplicate signatures trapped
`struct.set: field 0 out of range`. This is exactly the shape a Twig `record`
produces — a constructor plus N same-shape accessors plus a predicate collapse to
a few distinct function types — so records never ran on the WASM column despite
compiling and validating. Single-function cons programs and the list-op helpers
were unaffected because their function types happened to all be distinct, so the
per-function and deduplicated counts coincided.

Fix: pad by `instance.module.types.len()` (the type section's deduplicated
function-type count, i.e. the exact count the encoder used to place the struct
types) instead of `instance.func_types.len()`. One-line change; no API change; 47
existing wasm-runtime tests still pass.

## [0.3.0] — 2026-06-08

### Added — run WasmGC struct (cons) modules end-to-end (LANG77 / McCarthy L3b-3a-3c-2)

`WasmRuntime::call` now derives each WasmGC struct type's field count from the
parsed module's `struct_types` and registers it with the execution engine
(`set_struct_field_counts`), so a module that uses `struct.new`/`struct.get`
runs **without the embedder calling `set_struct_field_counts` by hand**. Field
counts are placed at their wasm *type index* (function types first, then struct
types — matching the encoder's layout).

With this, a hand-assembled `$LispyPair` cons module computing `(CAR (CONS 7 9))`
parses, instantiates, and **runs to `7`** on the in-repo runtime (both via the
explicit `load`→`instantiate`→`call` path and the all-in-one `load_and_run`).
Before this slice the same module trapped with "no field count registered for
struct type 1".

Note: assumes struct types follow all function types (true for the cons modules
we emit today, which declare no host imports). A module that interleaved
imported-function types after the struct types would need order-preserving type
parsing — not yet emitted or consumed. The reference-return placeholder from
earlier (a returned `Ref` → its handle / `0`) is unchanged; the cons return
boundary unboxes to `i32`, so it isn't exercised here.

2 new tests: the `(CAR (CONS 7 9))` → 7 end-to-end run and a `load_and_run`
regression guard.

## [0.2.0] - 2026-04-06

### Added

- **WASI Tier 3**: 8 new WASI host functions via the new `WasiEnv` struct:
  - `args_sizes_get` — write argc and argv buffer size to WASM memory
  - `args_get` — write argv pointer array and null-terminated strings to WASM memory
  - `environ_sizes_get` — write envc and environ buffer size to WASM memory
  - `environ_get` — write environ pointer array and null-terminated strings to WASM memory
  - `clock_res_get` — write clock resolution (nanoseconds) as i64 little-endian
  - `clock_time_get` — write current clock time (nanoseconds) as i64 little-endian
  - `random_get` — fill a WASM memory region with random bytes
  - `sched_yield` — no-op yield returning errno 0

- **`WasiClock` trait** — injectable clock interface for deterministic testing; `SystemClock` uses `std::time::SystemTime` and a lazy `Instant` for monotonic time

- **`WasiRandom` trait** — injectable random interface for deterministic testing; `SystemRandom` uses a hash-based fallback (NOT crypto-secure; documented and swappable)

- **`WasiConfig` struct** — configuration bundle for args, env, stdout/stderr callbacks, clock, and random; implements `Default`

- **`WasiEnv` struct** — full `HostInterface` implementation that resolves all Tier 3 WASI functions; uses `Arc<Mutex<LinearMemory>>` to share memory between host functions and the runtime

- **Integration tests** in `tests/wasi_tier3.rs` — 14 tests covering all 8 new functions with `FakeClock` and `FakeRandom` for deterministic verification

### Changed

- No breaking changes to existing `WasiStub`, `WasmRuntime`, or `WasmInstance` APIs

## [0.1.0] - 2026-04-05

### Added

- Initial package scaffolding generated by scaffold-generator
