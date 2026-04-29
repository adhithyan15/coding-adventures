# lang-runtime-core

Implementation of [LANG20 — Multi-Language Runtime Architecture](../../../specs/LANG20-multilang-runtime.md).

The substrate every language frontend (Lisp, Ruby, JS, Smalltalk, Perl, Tetrad, Twig, …) plugs into via the [`LangBinding`](src/binding.rs) trait. Owns the GC interface, allocator, safepoints, write barriers, stack maps, root scanning, symbol intern, inline-cache infrastructure, deopt protocol, and C ABI plumbing — all generic over the per-language [`Value`] / [`ClassRef`] / [`ICEntry`] choice.

## What this crate ships today (PR 1 of LANG20 §"Migration path")

**Trait skeleton + supporting types only.** No live GC, no interpreter wiring, no real C ABI exports. Subsequent PRs activate the substrate progressively per the migration plan.

### Modules

| Module | Type | Purpose |
|--------|------|---------|
| [`binding`](src/binding.rs) | `LangBinding`, `BuiltinFn`, `DispatchCx` | The 15-method trait every language implements |
| [`value`](src/value.rs) | `SymbolId`, `BoxedReprToken` | Intern handles + deopt repr discriminator |
| [`object`](src/object.rs) | `ObjectHeader`, `header_flags::*` | Uniform 16-byte heap header (LANG20 ABI commitment) |
| [`ic`](src/ic.rs) | `InlineCache<E>`, `ICState`, `ICId`, `ClassId`, `ICInvalidator`, `MAX_PIC_ENTRIES` | Generic V8-style inline cache infrastructure |
| [`deopt`](src/deopt.rs) | `FrameDescriptor`, `RegisterEntry`, `NativeLocation`, `DeoptAnchor`, `InlinedDeoptDescriptor` | Frame-descriptor format for JIT/AOT → interp deopt |
| [`visitor`](src/visitor.rs) | `ValueVisitor`, `RootVisitor` | Non-generic visitor traits the GC and root scanner use |
| [`error`](src/error.rs) | `RuntimeError` | Cross-language runtime error transport |

## Why a single `LangBinding` trait?

Per LANG20, the trait surface is sized so that:

- **No method is optional** (every binding implements every method) — no implicit "this language doesn't have method dispatch" footgun.
- **Three methods have correct default impls** for most languages (`is_truthy`, `identical`, `finalize`, `invalidate_ics`).
- **Every method maps 1:1 to a runtime mechanism the IIR dispatches through.** Adding more would creep semantics into the trait; removing any would force per-language code into `lang-runtime-core`.

See LANG20 §"Why these specific 15 methods" for the per-method justification table.

## Usage

```rust
use lang_runtime_core::{LangBinding, SymbolId, BoxedReprToken,
                        InlineCache, RuntimeError, BuiltinFn};

// A language frontend implements LangBinding for its own type.
// PR 2 ships LispyBinding as the first real impl; this crate's
// tests include a TestBinding that exercises every method.
```

The `binding.rs` test module's `TestBinding` is the canonical reference for how to implement the trait — it's a complete (toy) implementation in ~80 lines.

## Architecture (where this crate sits)

```text
       Twig / Lisp / Ruby / JS / Smalltalk / Perl source
                              │
                              ▼
                          typed AST
                              │
                              ▼
                         IIRModule  ◄─ universal interchange (LANG01)
                              │
       ┌──────────────────────┴────────────────────────┐
       │                                               │
       ▼                                               ▼
┌──────────────────────────┐                ┌────────────────────────────┐
│   LANG-runtime path      │                │    Host-runtime path       │
│   (LANG20)               │                │    (existing per-target)   │
│                          │                │                            │
│   vm-core / jit-core /   │                │   ir-to-jvm-class-file     │
│   aot-core               │                │   ir-to-cil-bytecode       │
│                          │                │   ir-to-beam               │
│   + lang-runtime-core    │  ◄─ this       │   ir-to-wasm-compiler      │
│   + LangBinding<L>       │     crate      │                            │
└──────────────────────────┘                └────────────────────────────┘
```

This crate is the foundational layer of the LANG-runtime path. The host-runtime path bypasses it entirely and lowers IIR to JVM/CLR/BEAM/WASM directly — see LANG20 §"Compilation paths" for why both paths matter.

## Tests

```bash
cargo test -p lang-runtime-core
```

68 unit tests + 1 doc test. Coverage includes:
- Every supporting type's invariants (`SymbolId` is 4 bytes, `ObjectHeader` is 16 bytes, `ICState` transitions, etc.).
- A full `TestBinding` impl exercising every `LangBinding` method end-to-end (proves the trait surface is implementable from the doc alone).
- `materialize_frame` walks a `FrameDescriptor` and reconstructs interpreter values.
- `ValueVisitor` and `RootVisitor` work through `&mut dyn` trait objects.

## Where it fits in the stack

```
LANG01  interpreter-ir            ← IIRModule format
LANG02  vm-core                   ← interpreter
LANG03  jit-core                  ← JIT (consumes feedback slots)
LANG04  aot-core                  ← AOT (consumes profiles)
LANG15  vm-runtime                ← linkable C ABI
LANG16  gc-core                   ← heap + GC algorithms
LANG20  lang-runtime-core         ← THIS CRATE — multi-language overlay
        ├── lispy-runtime         (PR 2 — Lisp/Scheme/Twig/Clojure)
        ├── ruby-runtime          (future)
        ├── js-runtime            (future)
        ├── smalltalk-runtime     (future)
        └── perl-runtime          (future)
```
