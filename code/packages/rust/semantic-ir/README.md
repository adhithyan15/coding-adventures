# semantic-ir

Narrow-waist Semantic IR — a language-neutral intermediate
representation between language frontends and code-emitting
backends.  Implements the design specified in
[SIR10](../../../specs/SIR10-narrow-waist-semantic-ir.md).

## Why?

Without a shared IR, every pair of N source languages and M target
languages requires its own translator — N × M.  With it, every
frontend lowers to SIR and every backend consumes SIR — **N + M**.

This is the **hourglass** / **narrow-waist** architecture used by
LLVM, GCC, MLIR, Pandoc, and the protobuf wire format.

```text
                 ┌─────────────────────────────────┐
   Twig AST ────►│                                 │──► TypeScript
   Lisp AST ────►│       Semantic IR (SIR)         │──► Python
   Rust AST ────►│      semantic-ir crate          │──► Rust
   Python AST ──►│                                 │──► Java
                 └─────────────────────────────────┘
```

## Design principles

1. **Strict → loose only.**  The IR carries semantic information
   from a more-featured source language to a less-featured target
   language.  Loose → strict translation (e.g. Python → Rust)
   requires inventing information that doesn't exist (type
   inference, ownership inference) and is **out of scope**.
2. **Disambiguation is the frontend's job.**  Every semantic concept
   is a distinct, named node.  There is never a case where a
   backend has to ask "what did the programmer mean here?".
3. **Module-level feature manifest** for O(1) backend rejection.
4. **Target-tagged opaque intrinsics** with strict discipline (the
   escape hatch — see SIR10 for the rules).
5. **Optional type carrier**; SIR does not infer or verify types.
6. **Source positions on every node.**
7. **Deterministic, human-readable text format.**

## Public API surface

```rust
use semantic_ir::{
    Module, Function, Param, Block, Stmt, Expr, Scope,
    SirType, EffectSet, Effect, Feature, FeatureManifest,
    Span, Metadata, CURRENT_SIR_VERSION,
    validate, Backend, BackendRegistry, Artifact,
    print_module, print_expr,
};
```

## Pipeline

```text
source code
   │
   ▼  language-specific frontend  (e.g. twig-to-semantic-ir)
semantic_ir::Module
   │
   ▼  validator                   (semantic_ir::validate)
validated Module
   │
   ▼  language-specific backend   (e.g. semantic-ir-to-typescript)
Artifact { source, ... }
```

## v0 scope

What's covered:

- Modules with manifest, imports, exports, metadata
- Functions with params, return types, captures, effects
- Atomic literals (int, bool, nil, symbol, string)
- VarRef with explicit scope tags
- If, Block, LetBinding, LetStarBinding, ExprStmt
- DirectCall, IndirectCall, BuiltinCall
- MakeClosure
- Intrinsic with escape-hatch discipline
- SirType (Any, Int, Bool, Nil, Symbol, Str, Pair, Closure, Fn)
- EffectSet bitset
- FeatureManifest
- Textual form (printer; parser deferred)
- Validator (errors + warnings)
- Backend trait + registry

What's deferred to later versions:

- Ownership / borrow markers
- Async / await / coroutines
- Exception handling (Raise / Try / Catch)
- Pattern matching (Match)
- Records / unions / type aliases
- The stdlib primitive set beyond Twig needs
- Text format parser (round-trip via printer only in v0)

## Related crates

- [`twig-to-semantic-ir`](../twig-to-semantic-ir/) — first frontend.
- [`semantic-ir-to-typescript`](../semantic-ir-to-typescript/) —
  first backend.

## Relationship to existing SIR00

There is an older Python implementation of a different "Semantic IR"
design at [`code/packages/python/semantic-ir/`](../../python/semantic-ir/)
following [SIR00](../../../specs/SIR00-semantic-ir.md).  The two
designs are intentionally not compatible — SIR10 (this crate) drops
features that SIR00 included (per-language extension bags, an
`INFERRED` loose→strict mode) in favour of stricter narrow-waist
discipline.

## License

MIT
