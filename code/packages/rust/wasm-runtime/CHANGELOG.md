# Changelog

All notable changes to this package will be documented in this file.

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
