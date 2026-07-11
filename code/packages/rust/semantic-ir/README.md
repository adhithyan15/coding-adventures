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
    // SIR22: array/matrix IR extension
    ElementwiseOpKind, IndexArg,
    // SIR23 (symbolic expression + pattern/rewrite IR extension) adds no
    // new standalone type — its seven nodes are `Expr` variants
    // (`SymSymbol`/`SymRational`/`SymApply`/`SymPatternBlank`/
    // `SymPatternNamed`/`SymRule`/`SymReplaceAll`), already covered by the
    // `Expr` re-export above.
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
- SirType (Dynamic, Int(IntSpec), Bool, Nil, Symbol, Str, Pair, Closure, Fn, Float, Seq, Map, Ptr, Struct, Optional,
  NDArray, Rational, Complex, SymExpr)
  — SIR21: `Int` carries an `IntSpec { width, signed, overflow }`; `Dynamic` (was `Any`) is the top type;
  `Ptr`/`Struct`/`Optional` (T1b) are source-fidelity types for typed frontends
  — SIR22: `NDArray { elem, rank }` is a dense N-D numeric array (rank `None` = unknown/dynamic);
  `Rational`/`Complex` are exact-rational and complex scalar carriers, shared with the SIR23
  symbolic-math extension
  — SIR23: `SymExpr` is an opaque symbolic-expression handle with no static shape (the shape lives
  in the `Expr` tree, not the type carrier)
- SIR22 array/matrix nodes (additive, numeric-array languages — MATLAB/Octave and future
  APL/J/K/Scilab/IDL frontends): `Expr::ArrayLit` (matrix literal `[1 2; 3 4]`), `Expr::Range`
  (`1:5`, `0:2:10`), `Expr::MatMul`, `Expr::ElementwiseOp` (the five dotted ops `.+ .- .* ./ .^`
  via `ElementwiseOpKind`), `Expr::Transpose` (`'` vs `.'` via a `conjugate` flag), `Expr::IndexGet`
  / `Stmt::IndexSet` (indexed read/write via `IndexArg::{Scalar,Whole,Range}` — write is a `Stmt`,
  mirroring how SIR16's `Assign` is a `Stmt` and not an `Expr`). New features:
  `NDArrays`/`MatrixOps`/`Rationals`/`Complex`/`ArrayColumnMajor`. Every new node kind maps 1:1 onto
  an `array_runtime::execute()` op shape — see
  [SIR22](../../../specs/SIR22-array-matrix-semantic-ir.md).
- SIR23 symbolic expression + pattern/rewrite nodes (additive, symbolic-CAS math languages —
  Wolfram/Macsyma/Maxima and future Reduce/Derive/Maple frontends): `Expr::SymSymbol` (a bare
  symbol used as data), `Expr::SymRational` (exact rational scalar), `Expr::SymApply`
  (`head[args…]` as data — `head` is a full `Expr`, since a *computed* head like `f[x][y]` is
  legal Wolfram), `Expr::SymPatternBlank` (`_` / `_h`), `Expr::SymPatternNamed` (`x_` / `x_h`),
  `Expr::SymRule` (`->` / `:>` via a `delayed` flag), `Expr::SymReplaceAll` (`/.` / `//.` via a
  `repeated` flag). No new `Stmt` — every node is `Pure`. New features: `SymbolicExpr`/
  `PatternMatching` (`Rationals` is reused from SIR22). Every node maps 1:1 onto
  `symbolic_ir::IRNode`'s five-variant shape — see
  [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md).
- `int_const` — the `int.max`/`int.min`/`int.width` reflection const-intrinsics (T3a);
  pure, const-folding functions of an `IntSpec` (`IntConst::eval`), `None` for arbitrary-precision
- `op_select` — type-directed op selection (T3c): `resolve_binary(op, lhs, rhs)` chooses
  `IntArith`/`FloatArith`/`StrConcat`/`TypedCompare`/`RuntimeDispatch` for any binary operator
  from operand types (`resolve_numeric` is the numeric-only core; pure, no inference)
- EffectSet bitset
- FeatureManifest
- Textual form (printer; parser deferred)
- Validator (errors + warnings)
- Backend trait + registry

What's deferred to later versions:

- Ownership / borrow markers
- Async / await / coroutines
- Exception handling (Raise / Try / Catch)
- Records / unions / type aliases
- The stdlib primitive set beyond Twig needs
- Text format parser (round-trip via printer only in v0)
- Symbolic pattern-matching / rewrite-rule **execution**: SIR23 lands the
  IR vocabulary (`Expr::SymPatternBlank`/`SymPatternNamed`/`SymRule`/
  `SymReplaceAll`), but no backend implements the matcher yet — that's
  `sir-runtime-symbolic`, a separate future package (see the SIR23 spec's
  "Backend impact")

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
