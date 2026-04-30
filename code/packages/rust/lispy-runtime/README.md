# lispy-runtime

**LANG20 PR 2** — first concrete `LangBinding` implementation. Shipped as `LispyBinding`, the binding for **Lisp / Scheme / Twig / Clojure** frontends. Every Lispy frontend reuses *everything* in this crate (value rep, builtins, heap, intern table, C ABI) and only writes its own AST → IIR step.

## What this crate ships

| Module | What |
|--------|------|
| [`value`](src/value.rs) | `LispyValue` — tagged i64 (immediate ints / nil / true / false / symbols + heap-tagged pointers) |
| [`heap`](src/heap.rs) | `ConsCell`, `Closure` — `#[repr(C)]` with the LANG20 16-byte header. `alloc_cons`, `alloc_closure`, `car`, `cdr`, `is_*`, `as_closure` |
| [`intern`](src/intern.rs) | Process-global symbol intern table with `intern(name)` / `name_of(id)` |
| [`builtins`](src/builtins.rs) | TW00 builtin handlers: `+ - * / = < >`, `cons car cdr`, `null? pair? number? symbol?`, `print` |
| [`binding`](src/binding.rs) | `LispyBinding` — full `LangBinding` impl + `LispyClass` + `LispyICEntry` |
| [`abi`](src/abi.rs) | `extern "C"` surface: `lispy_cons`, `lispy_car`, `lispy_cdr`, `lispy_make_symbol`, `lispy_make_closure`, `lispy_apply_closure`, `lispy_closure_capture_count` |

## Tag scheme (`LispyValue`)

Single `u64` with a 3-bit tag in low bits — V8-style for integers, with extra tags for nil / booleans / immediate symbols so common values don't need heap allocation:

| Tag (low 3 bits) | Kind | Payload |
|------------------|------|---------|
| `0b000` | Integer | high 61 bits, signed (range ±2⁶⁰ ≈ ±10¹⁸) |
| `0b001` | Nil singleton | (none) |
| `0b010` | Symbol immediate | high 32 bits = `SymbolId` |
| `0b011` | False singleton | (none) |
| `0b101` | True singleton | (none) |
| `0b111` | Heap pointer | full word with low 3 bits cleared |

`LispyValue` is exactly 8 bytes — asserted at compile time via `const _: () = assert!(...)` so an accidental enlargement breaks the build immediately.

## Heap layout

Every heap object starts with the LANG20 uniform 16-byte `ObjectHeader`:

```text
ConsCell (32 bytes):                    Closure (≥ 48 bytes):
┌─────────────────────────────┐          ┌─────────────────────────────┐
│ ObjectHeader (16 bytes)     │          │ ObjectHeader (16 bytes)     │
├─────────────────────────────┤          ├─────────────────────────────┤
│ car: LispyValue (8 bytes)   │          │ fn_name: SymbolId (4 bytes) │
├─────────────────────────────┤          │ _reserved: u32 (4 bytes)    │
│ cdr: LispyValue (8 bytes)   │          ├─────────────────────────────┤
└─────────────────────────────┘          │ captures: Vec<LispyValue>   │
                                          └─────────────────────────────┘
```

Class IDs (`CLASS_CONS = 1`, `CLASS_CLOSURE = 2`) are language-private — only `LispyBinding` reads them. The collector dispatches trace via the binding's `trace_object` so the GC stays language-agnostic.

## Allocator (PR 2 vs. future)

PR 2 ships `Box::leak`-based allocation. Every cons cell and closure lives forever — there is no collector yet. When LANG16's `gc-core` lands, `alloc_cons` / `alloc_closure` swap to the bump-pointer + mark-sweep path without their callers noticing.

## Usage

```rust
use lispy_runtime::{LispyBinding, LispyValue, alloc_cons, builtins, intern};
use lang_runtime_core::LangBinding;

// Build (1 . 2)
let pair = alloc_cons(LispyValue::int(1), LispyValue::int(2));
assert!(LispyBinding::class_of(pair).unwrap() == lispy_runtime::LispyClass::Cons);

// Resolve and call a builtin
let plus = LispyBinding::resolve_builtin("+").unwrap();
assert_eq!(plus(&[LispyValue::int(2), LispyValue::int(3)]).unwrap(), LispyValue::int(5));

// Intern a symbol and tag it as a value
let foo_id = intern("foo");
let foo = LispyValue::symbol(foo_id);
assert!(foo.is_symbol());
```

## What this PR does NOT ship

- **Live GC integration** — the allocator leaks; `gc-core` (LANG16) wires the real collector.
- **Closure dispatch** — `apply_callable` returns a placeholder `RuntimeError`; PR 4 (vm-core wiring) makes it functional.
- **`send` / `load_property` / `store_property` opcode handlers** — Lispy doesn't use these, so the binding correctly returns `NoSuchMethod` / `NoSuchProperty`.
- **JVM/CLR/BEAM/WASM backends** — those remain on the host-runtime path per [LANG20 §"Compilation paths"](../../specs/LANG20-multilang-runtime.md) and are unchanged by this PR.

## Tests

```bash
cargo test -p lispy-runtime
```

101 unit tests + 1 doc test covering:
- Tag round-trips for every immediate kind
- Heap allocation and accessor invariants
- Symbol interning idempotency
- Every builtin's success and error paths
- The full `LangBinding` trait surface (type_tag, class_of, equal/identical, hash, trace, materialize/box round-trips)
- C ABI symbols (`lispy_cons` round-trip, `lispy_make_symbol` interning, `lispy_make_closure` capture recording)

Coverage is comprehensive but does NOT include panic-across-FFI tests — `extern "C"` panics are undefined behavior, so the panic paths are tested via the underlying Rust API instead.

## Safety

The crate uses `unsafe` for the tagged-pointer scheme (`from_heap` / `as_heap_ptr`), heap accessors (`car`/`cdr`/`is_cons`/`is_closure`/`as_closure`), the `extern "C"` ABI surface, and `LangBinding::trace_object`. Three concentric defenses contain it:

1. **Type-system enforcement.** `LispyValue.0` is private; safe Rust cannot fabricate a `LispyValue` with a fake heap tag. The dangerous reconstructor (`from_raw_bits`) is `unsafe`.
2. **Every dangerous operation is `unsafe fn`.** `heap::car`/`cdr`/`is_cons`/`is_closure`/`as_closure` and the `lispy_*` FFI symbols all require an `unsafe { }` block at every call site, with a `// SAFETY: …` comment justifying why the contract holds.
3. **CI safety nets** ([`.github/workflows/lang-runtime-safety.yml`](../../../.github/workflows/lang-runtime-safety.yml)):
   - **Miri** (with `-Zmiri-ignore-leaks` for the PR-2 intentional-leak allocator) runs the full test suite as an interpreter to catch real UB at runtime — out-of-bounds reads, use-after-free, misaligned access, data races, aliasing violations.
   - **cargo-geiger** publishes an unsafe-expression report per crate so PR reviewers see whether new unsafe was introduced.

The CI runs only when files in `lang-runtime-core/` or `lispy-runtime/` change.

## Where this crate sits

```
LANG01  interpreter-ir            ← IIRModule format
LANG02  vm-core                   ← interpreter
LANG03  jit-core                  ← JIT
LANG04  aot-core                  ← AOT
LANG15  vm-runtime                ← linkable C ABI
LANG16  gc-core                   ← heap + GC algorithms
LANG20  lang-runtime-core         ← LangBinding trait + supporting types (PR 1)
        └── lispy-runtime         ← THIS CRATE — first concrete binding (PR 2)
            ├── (twig-frontend)   ← PR 3 ports twig-ir-compiler to use this
            ├── (lisp-frontend)   ← future
            ├── (scheme-frontend) ← future
            └── (clojure-frontend) ← future
```
