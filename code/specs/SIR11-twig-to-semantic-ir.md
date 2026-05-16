# SIR11 — Twig AST → Semantic IR (Rust frontend)

## Status

First frontend for the narrow-waist Semantic IR
([`SIR10`](SIR10-narrow-waist-semantic-ir.md)).  Implemented as the
Rust crate
[`twig-to-semantic-ir`](../packages/rust/twig-to-semantic-ir/).

## Overview

This crate consumes the typed AST produced by
[`twig-parser`](../packages/rust/twig-parser/) and lowers it to a
[`semantic_ir::Module`](../packages/rust/semantic-ir/).  It is the
first frontend exercising the narrow-waist SIR; the design choices
that surface here drive what the SIR v0 needs to be able to express.

## Pipeline

```text
Twig source
   │
   ▼  twig_parser::parse
twig_parser::Program  (typed AST: Form / Expr / Lambda / ...)
   │
   ▼  twig_to_semantic_ir::compile
semantic_ir::Module   (narrow-waist SIR v0)
```

## Public API

```rust
pub fn compile(
    program:     &twig_parser::Program,
    module_name: &str,
) -> Result<semantic_ir::Module, TwigLowerError>;

pub fn compile_source(
    source:      &str,
    module_name: &str,
) -> Result<semantic_ir::Module, TwigLowerError>;  // parse + compile

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwigLowerError {
    pub message: String,
    pub line:    usize,
    pub column:  usize,
}
```

`compile_source` is the convenience entry point; `compile` is the
factored form that allows callers to inspect the parsed AST before
lowering.

## Twig v0 surface coverage

The frontend covers everything in
[`TW00-twig-language.md`](TW00-twig-language.md):

| Twig construct                    | SIR node(s)                           |
|-----------------------------------|---------------------------------------|
| `(define x expr)` (value)         | `Global` + assignment in `_init`      |
| `(define (f a b) body...)`        | `Function`                            |
| `42`, `-7`                        | `IntLit`                              |
| `#t`, `#f`                        | `BoolLit`                             |
| `nil`                             | `NilLit`                              |
| `'foo`, `(quote foo)`             | `SymLit`                              |
| `"hello"`                         | `StrLit`                              |
| name reference                    | `VarRef { scope }`                    |
| `(if c t e)`                      | `If`                                  |
| `(let ((x e)...) body...)`        | `Block` with `LetBinding`s            |
| `(let* ((x e)...) body...)`       | `Block` with `LetStarBinding`s        |
| `(begin e1 e2 ...)`               | `Block` with `ExprStmt`s              |
| `(lambda (a b) body...)`          | `MakeClosure` + synthesised `Function`|
| `(f a b)` — `f` top-level         | `DirectCall`                          |
| `(f a b)` — `f` local             | `IndirectCall`                        |
| `(+ a b)`, `(cons a b)`, etc.     | `BuiltinCall`                         |

LANG48 typed forms (records, unions, match, type annotations) are
**deferred** in this v0 frontend — they exist in the Twig AST but
the lowerer treats them as errors.  A later spec will cover them.

## Apply-site dispatch

The frontend commits to call shape at lowering time, matching the
existing `twig-ir-compiler` table from TW00:

| Function position           | SIR node                                  |
|-----------------------------|-------------------------------------------|
| Top-level name              | `DirectCall { fn_name, ... }`             |
| Local / param / capture     | `IndirectCall { target = VarRef, ... }`   |
| Builtin (`+`, `cons`, ...)  | `BuiltinCall { name, ... }`               |

The lowerer maintains a `KnownNames` table (top-level function names
+ builtin names) constructed in a first pass.  When emitting an
`Apply` whose function position is a `VarRef`, the lowerer consults
`KnownNames` to pick `DirectCall` vs `BuiltinCall` vs `IndirectCall`.

## Closures

Each `(lambda (a b) body...)` becomes:

1. A fresh top-level `Function` named `__lambda_<gensym>`, with the
   captured free variables prepended to its `captures` list and the
   user-visible parameters as `params`.
2. A `MakeClosure` node at the lambda's source location, with the
   captures filled in from the surrounding scope.

The synthesised function's body resolves capture references as
`VarRef { scope: Capture }`.  Top-level `(define (f ...) body)` does
**not** allocate captures — it is a plain `Function` with empty
`captures` and the `IndirectCall`/`DirectCall` dispatch table treats
its name as known.

## Free-variable analysis

A pre-pass over each lambda body walks the AST and computes the set
of names referenced that are not bound by the lambda's own parameters
or any inner let/let*/lambda.  These names form the lambda's capture
list.  The analysis:

- Treats `(let ((x e1)) body)` as binding `x` in `body` but not in
  `e1` (parallel semantics).
- Treats `(let* ((x e1)) body)` as binding `x` in subsequent RHS
  expressions and in `body` (sequential semantics).
- Treats `(lambda (a b) body)` as binding `a` and `b` in `body`.
- Treats `(begin e1 e2 ...)` as no new bindings.

Free-name resolution honours the same scope tags as the main
lowering pass — locals shadow captures shadow globals shadow
builtins.

## Synthesised functions

A Twig program with mixed top-level forms gets two synthesised
functions:

- **`_init`** — body is the sequence of top-level `(define x expr)`
  assignments, lowered to `BuiltinCall("global_set", ...)` (or a
  dedicated `Global` write — see below).  Returns `nil`.
- **`main`** — body is the sequence of top-level bare expressions,
  with the value of the last expression as the return value (or
  `nil` if no bare expressions).

If the program has no top-level value defines, `_init` is omitted.
If the program has no bare top-level expressions, `main`'s body is
just `(nil)`.

### Global initialization model

Top-level value defines could be represented in two equivalent ways:

1. As `Global` entries plus `BuiltinCall("global_set", name, value)`
   in `_init`.  Backends that have native top-level let (TypeScript,
   Python) lower this to `let name = value`.
2. As `Global` entries with an explicit init expression on the
   `Global` itself.

This frontend uses **(1)** for v0 — it matches the existing
`twig-ir-compiler` precedent and keeps `Global` simple.  Backends
recognise the `global_set` pattern in `_init` and emit native
top-level declarations.

## Scope resolution

Every `VarRef` carries an explicit `Scope` tag.  The lowerer maintains
a `Scopes` stack:

- Push a `LocalFrame` at the entry of each function body.
- Push a `LetFrame { kind: Let | LetStar, bindings }` at the entry of
  a `let` or `let*` form.
- Lookup walks the stack top-down: first `LocalFrame`s and
  `LetFrame`s (the local scope), then the function's `params`, then
  the function's `captures`, then the module's `globals` table, then
  the builtin table.  The first hit wins and supplies the scope tag.

A name with no hit anywhere is a `TwigLowerError`.

## Builtin table

The v0 builtin set matches TW00:

```text
Arithmetic:  +  -  *  /
Comparison:  =  <  >
Pairs:       cons  car  cdr
Predicates:  null?  pair?  number?  symbol?
I/O:         print
Globals:     global_get  global_set   (lowering-internal)
```

These are emitted as `BuiltinCall` with the operator name in `name`.
Effects are pre-tabulated:

| Builtin                 | EffectSet                  |
|-------------------------|----------------------------|
| `+`, `-`, `*`, `/`      | Pure                       |
| `=`, `<`, `>`           | Pure                       |
| `cons`                  | MayAllocate                |
| `car`, `cdr`            | Pure                       |
| `null?`, `pair?`, etc.  | Pure                       |
| `print`                 | MayPrint                   |
| `global_get`            | Pure                       |
| `global_set`            | (no effect — write to env) |

`/` is integer division (truncation), matching Twig spec.

## Manifest computation

The lowerer accumulates the feature manifest as it walks:

- Any `MakeClosure` or `IndirectCall` → `Closures`
- Any `BuiltinCall("cons" | "car" | "cdr" | "pair?", ...)` → `Pairs`
- Any `SymLit` → `Symbols`
- Any `StrLit` → `Strings`
- Any `Param` or `Global` with `sir_type = None` → `DynamicTyping`
- Any `Param` or `Global` with `sir_type = Some(_)` → `OptionalTypeAnnotations`
- Any `Global` → `Globals`
- Any `Intrinsic` → `Intrinsics`

`MutualRecursion` and `TailCalls` are conservatively included only
if the frontend detects them (v0 includes `MutualRecursion`
unconditionally since Twig's name resolution naturally permits it,
and excludes `TailCalls` — Twig spec does not promise TCO).

## Error model

```rust
pub struct TwigLowerError {
    pub message: String,
    pub line:    usize,
    pub column:  usize,
}
```

Error sites:

- Unresolved name reference
- Empty lambda body
- Empty let / let* / begin body
- Apply on a non-callable atom (e.g. `(42 1 2)`)
- LANG48 forms encountered (records / unions / match / type
  annotations) — explicit "not yet supported in SIR v0" error
- Top-level expression that is neither a `define` nor a callable
  form (Twig's grammar already rejects most of this)

All errors carry the source position from the offending AST node.

## Tests

- **Unit tests** for each lowering rule.  Each rule has at least one
  positive test (correct lowering) and one negative test (error
  case) where applicable.
- **Golden tests** comparing the printed SIR text of canonical
  programs (factorial, mutual recursion, closure adder, let, let*,
  cons / car / cdr, print) against committed `.sir` fixture files.
- **Round-trip integration** with the `semantic-ir` parser: parse a
  Twig program, lower to SIR, print SIR to text, parse the text back
  to SIR, assert equality.
- **Validator integration**: every lowered SIR module must pass
  `semantic_ir::validate` cleanly.

Coverage target: **≥ 95%**.

## Out of scope (deferred)

- LANG48 records, unions, match — separate spec when SIR adds the
  matching node kinds.
- LANG23 / TW05 refinement types — the frontend currently discards
  any `TypeAnnotation` on params / return / value defines (it sets
  `sir_type = None`).  A later spec adds the carrier.
- Twig modules (`(module ...)` declarations + multi-file).  The
  `twig-parser` produces a `Program.module_info`; the lowerer
  currently sets `Module.name = module_name` parameter and ignores
  `module_info` content.  Multi-module compilation will follow when
  `twig-module-driver` is wired into the SIR pipeline.
- Tail-call optimisation.  Twig spec defers it to BEAM.
