# Changelog — lispy-runtime

## [0.2.0] — 2026-04-29

### Added — supporting LANG20 PR 5 (twig-vm closures / globals / symbols)

- **`builtins::make_symbol(name)`** — return the symbol named by
  `name` (which is itself a symbol value under the dispatcher's
  string-as-symbol convention).  Identity-with-type-check; the
  IR compiler emits a call to it for every quoted-symbol
  literal `'foo`.

- **`builtins::make_closure(name, ...captures)`** — allocate a
  user-fn closure capturing `captures*` over the IIRFunction
  named `name`.  Routes through the existing `alloc_closure`
  factory.

- **`builtins::make_builtin_closure(name)`** — allocate a
  closure-shaped wrapper around a builtin so it can be passed
  in higher-order positions (e.g. `(define (apply-it f x) (f x))
  (apply-it + 2 3)`).  No captures by construction; flagged via
  `CLOSURE_FLAG_BUILTIN` so apply-time dispatch routes through
  `LispyBinding::resolve_builtin` rather than the user-fn lookup
  path.

- **`heap::alloc_builtin_closure(name)`** — factory function
  for builtin-wrapping closures.  Same Box::leak pattern as
  `alloc_closure`.

- **`heap::CLOSURE_FLAG_BUILTIN`** + **`Closure::is_builtin()`**
  + **`Closure::new_builtin(name)`** — the ABI bit + Rust API
  for distinguishing the two closure flavours at apply time.

### Changed

- **`Closure._reserved`** field renamed to **`Closure.flags`**
  (still `u32`, same offset, same alignment — pure rename).
  Bit 0 is now `CLOSURE_FLAG_BUILTIN`; higher bits remain
  reserved (planned: arity hint).

- **`LispyBinding::resolve_builtin`** extended with the three
  new builtins (`make_symbol`, `make_closure`,
  `make_builtin_closure`).  `apply_closure` / `global_set` /
  `global_get` are deliberately NOT registered here — they
  need access to per-VM state (globals table, module
  reference, dispatcher recursion) and are handled inline in
  `twig-vm::dispatch`.

### Tests

- 105 unit + 1 doc — unchanged at the test level; the new
  builtins inherit the test scaffolding from the existing
  builtins.  Closure-flag round-trip is exercised
  transitively through `twig-vm::dispatch::tests` (which
  passes under Miri).

## [0.1.0] — 2026-04-29

### Added

- **PR 2 of [LANG20](../../../specs/LANG20-multilang-runtime.md)
  §"Migration path"**: first concrete `LangBinding` implementation.
  Shared by Lisp / Scheme / Twig / Clojure frontends — every
  Lispy frontend reuses everything in this crate and only writes
  its own AST → IIR step.
- **`LispyValue`** (`value.rs`) — tagged i64 with 3-bit tag in low
  bits.  Immediates: integer (high 61 bits), nil, true, false,
  symbol (high 32 bits = SymbolId).  Heap pointer with low 3
  bits cleared (8-aligned allocations).  Compile-time const
  assertion that the type is exactly 8 bytes (LANG20 ABI
  commitment for `LangBinding::Value`).
- **`heap` module** (`heap.rs`) — `ConsCell` and `Closure`
  `#[repr(C)]` types prefixed with the LANG20 16-byte
  `ObjectHeader`.  Compile-time-asserted layout (`ConsCell` is
  exactly 32 bytes; `Closure` is 8-aligned).  Manual `Debug`
  impls that skip the header (`AtomicU32` Debug isn't useful in
  test output).  `alloc_cons` / `alloc_closure` factory
  functions, plus `car`, `cdr`, `is_cons`, `is_closure`,
  `as_closure` accessors.
- **`intern` module** (`intern.rs`) — process-global symbol
  intern table backed by a `Mutex<HashMap<String, SymbolId>>` +
  `Mutex<Vec<String>>`.  Eagerly interns the empty string at
  startup so `SymbolId::EMPTY` is always `""`.  `intern(name)` /
  `name_of(id)` / `len()` API.
- **`builtins` module** (`builtins.rs`) — TW00 builtin handlers.
  Variadic arithmetic with Scheme identity semantics
  (`(+) == 0`, `(*) == 1`); binary comparisons; cons / car /
  cdr; null? / pair? / number? / symbol?; print.  Each builtin
  is a `BuiltinFn<LispyBinding>` so `LangBinding::resolve_builtin`
  returns them as fn pointers the JIT/AOT can emit direct calls
  to.  Wrapping arithmetic (`wrapping_add`, `wrapping_sub`,
  `wrapping_mul`) for overflow safety; division-by-zero raises
  `RuntimeError::TypeError`.
- **`binding` module** (`binding.rs`) — `LispyBinding` unit
  struct implementing the full `LangBinding` trait.
  - `LispyClass` enum: Int / Nil / Bool / Symbol / Cons /
    Closure.  Maps to LANG20 `ClassId` via `to_class_id` (stable
    integer mapping).
  - `LispyICEntry` `(tag: u32, target: usize)` — IC entry shape
    for type-keyed dispatch.  Lispy doesn't use method dispatch,
    so this is a placeholder satisfying the trait constraint
    `ICEntry: Copy + 'static`.
  - `type_tag` returns the immediate-tag bits for immediates and
    the header's `class_or_kind` for heap values (so the
    profiler distinguishes Cons from Closure without
    re-dereferencing).
  - `equal` is structural for cons cells (recurses on car/cdr)
    and bitwise for everything else.
  - `identical` is bitwise equality of the value words.
  - `apply_callable` validates the value is a closure; PR 2
    returns a placeholder `RuntimeError::Custom("PR 4")` because
    actual dispatch needs vm-core wiring (LANG20 PR 4).
  - `send_message` / `load_property` / `store_property` return
    `NoSuchMethod` / `NoSuchProperty` because Lispy has no
    method dispatch.
  - `resolve_builtin` resolves all 15 TW00 builtin names.
  - `materialize_value` / `box_value` round-trip cleanly through
    every `BoxedReprToken` variant.
- **`abi` module** (`abi.rs`) — `extern "C"` surface for JIT/AOT
  codegen: `lispy_cons`, `lispy_car`, `lispy_cdr`,
  `lispy_make_symbol`, `lispy_make_closure`,
  `lispy_apply_closure`, `lispy_closure_capture_count`.  Per
  LANG20 §"Per-language symbols" — symbol names are locked.
  Empty-array entry points handle `n == 0` without violating
  `slice::from_raw_parts`'s alignment precondition.
  PR 2 panic-on-misuse error policy is documented; PR 4+ swaps
  to the shared `rt_*` thread-local error channel.
- 101 unit tests + 1 doc test exercising every module.

### Hardening from security review (HIGH + MEDIUM + 3 LOW addressed before push)

- **HIGH (Finding 1+2):** `LispyValue`'s inner `u64` is **private**.
  Safe Rust cannot fabricate a fake heap-tagged value.  The
  reconstructor (`LispyValue::from_raw_bits`) is `unsafe` and
  documented as such.  All `extern "C" fn lispy_*` symbols are now
  `unsafe extern "C" fn` (matching the contract that callers must
  pass bits from a prior `lispy_*` call).
- **HIGH (Finding 1):** All heap accessors (`heap::car` / `cdr` /
  `is_cons` / `is_closure` / `as_closure`) are `unsafe fn`.  Every
  call site has an `unsafe { }` block with a `// SAFETY:` comment
  justifying why the contract holds.
- **MEDIUM (Finding 3):** Arithmetic builtins use `checked_*`
  operations and return `RuntimeError::TypeError("integer overflow")`
  on overflow rather than panicking or silently wrapping.  Catches
  the `i64::MIN / -1` overflow and the analogous cases for
  `+`/`-`/`*`.
- **MEDIUM (Finding 4):** Symbol intern table capped at
  `MAX_SYMBOLS = u32::MAX - 1` to (a) keep `SymbolId::NONE`
  reserved (was a sentinel-collision bug) and (b) bound memory
  growth under adversarial input.  Returns `SymbolId::NONE` on
  overflow rather than handing out a colliding id.
- **LOW (Finding 5):** `LispyValue::from_heap` is `unsafe fn` with
  a hard `assert!` (not `debug_assert!`) for alignment.  An
  unaligned pointer panics in both debug and release.
- **LOW (Finding 6):** `LangBinding::materialize_value`'s
  `DerivedPtr` arm explicitly clears the low 3 bits before OR'ing
  the heap tag so non-aligned derived offsets can't corrupt the
  address.
- **LOW (Finding 7):** `LispyValue::int` debug-asserts the value
  fits the 61-bit signed range; tests cover the assertion.

### Provenance

- `LispyValue::from_heap` and `as_heap_ptr` use the strict-
  provenance APIs (`expose_provenance` / `with_exposed_provenance`)
  for the tagged-pointer round-trip.  Default Miri (without
  `-Zmiri-strict-provenance`, which is fundamentally incompatible
  with tagged-pointer schemes) accepts the round-trip and catches
  every other UB class — what we actually care about.

### Safety CI

- Added `.github/workflows/lang-runtime-safety.yml`:
  - **Miri** (`-Zmiri-ignore-leaks` for the PR-2 intentional-leak
    allocator) runs the full test suite to catch UB at runtime.
    All 101 tests pass; we run them on every PR that touches
    `lang-runtime-core/` or `lispy-runtime/`.
  - **cargo-geiger** publishes an unsafe-expression report per
    crate so reviewers can see whether new unsafe was introduced.

### Trait-level invariants

- `<LispyBinding as LangBinding>::Value` is exactly 8 bytes
  (compile-time assertion via `value.rs`'s `const _: () = ...`).
- `LANGUAGE_NAME == "lispy"` (lower-snake-case ASCII per LANG20).
- `LispyClass::to_class_id` is stable: two calls return the same
  id; distinct kinds get distinct ids.

### Known limitations (intentional, follow in later PRs)

- Allocator is `Box::leak` — PR 4+ (LANG16 gc-core wiring) ships
  the real collector.
- `apply_callable` is a placeholder; PR 4 wires it through
  `DispatchCx::call_iir_function`.
- `send_message` / `load_property` / `store_property` return errors
  by design (Lispy has no method dispatch).  Real implementations
  for Ruby/JS frontends land in those crates.
- C ABI panic-on-misuse is intentional for PR 2; PR 4+ introduces
  the `rt_*` thread-local error channel.
- No JVM/CLR/BEAM/WASM backends — those remain on the
  host-runtime path per LANG20 §"Compilation paths" and don't
  consume this crate.
