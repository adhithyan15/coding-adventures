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
| name of a top-level function      | `lambda`-marked `MakeClosure`, no captures |
| `(if c t e)`                      | `If`                                  |
| `(let ((x e)...) body...)`        | `Block` with `LetBinding`s            |
| `(let* ((x e)...) body...)`       | `Block` with `LetStarBinding`s        |
| `(begin e1 e2 ...)`               | `Block` with `ExprStmt`s              |
| `(lambda (a b) body...)`          | `BuiltinCall("lambda", [MakeClosure])` + synthesised `Function`|
| `(f a b)` — `f` local             | `IndirectCall`                        |
| `(f a b)` — `f` top-level fn      | `DirectCall`                          |
| `(f a b)` — `f` top-level value   | `IndirectCall` on the `Global`        |
| `(+ a b)`, `(cons a b)`, etc.     | `BuiltinCall` (if nothing above binds the name) |

LANG48 typed forms (records, unions, match, type annotations) are
**deferred** in this v0 frontend — they exist in the Twig AST but
the lowerer treats them as errors.  A later spec will cover them.

## Apply-site dispatch

The frontend commits to call shape at lowering time.  A call's function
position resolves in **the same order as a name in value position**
(TW00 "Name resolution: lexical scope wins"); the first row that binds
the name picks the node:

| Row | Function position bound by                                  | SIR node                                  |
|-----|-------------------------------------------------------------|-------------------------------------------|
| 1   | Local / param / lambda param / capture (innermost first)    | `IndirectCall { target = VarRef, ... }`   |
| 2   | Top-level function `define`                                 | `DirectCall { fn_name, ... }`             |
| 3   | Top-level value `define`                                    | `IndirectCall { target = VarRef Global }` |
| 4   | Builtin (`+`, `cons`, ...)                                  | `BuiltinCall { name, ... }`               |
| —   | Nothing                                                     | `TwigLowerError` ("unresolved name")      |

A program may therefore redefine a builtin, and the definition wins
in call position exactly as in value position:

```scheme
(define (car p) 42)
(car (cons 1 2))               ; row 2: DirectCall car   → 42
(define (f car) (car 1))       ; row 1: IndirectCall on the parameter
(define + (let ((k 100)) (lambda (a b) k)))
(+ 1 2)                        ; row 3: IndirectCall on Global + → 100
```

`print`'s SIR28 lowering to `__sys_write__` is part of row 4, so a
user's `print` (function, value or local) is called like any other
binding.  A `define` whose expression is a bare `lambda` is a function
define (row 2).  Special forms (`if`, `let`, `let*`, `lambda`, `begin`,
`quote`) are parsed to their own AST nodes before resolution, so they
stay syntax.

The lowerer builds the top-level function and value name sets in a
first pass, so a call that appears before the definition it refers to
still resolves to that definition.

> **Revision (issue #18456).** This section used to list the rows as
> "top-level name, then local, then builtin", and the implementation
> checked the builtin table *first* in call position.  So
> `(define (car p) 42) (car x)` lowered to `BuiltinCall car`, and a
> parameter named like a top-level function lowered to a `DirectCall`
> to that function.  `twig-to-semantic-ir` 0.2.1 follows the table
> above.

## Closures

Each `(lambda (a b) body...)` becomes:

1. A fresh top-level `Function` named `__lambda_<gensym>`, with the
   captured free variables prepended to its `captures` list and the
   user-visible parameters as `params`.
2. At the lambda's source location, a **strict** closure:
   `BuiltinCall("lambda", [MakeClosure { fn_name, captures }])`, with
   the captures filled in from the surrounding scope.

The synthesised function's body resolves capture references as
`VarRef { scope: Capture }`.  Top-level `(define (f ...) body)` does
**not** allocate captures — it is a plain `Function` with empty
`captures` and the `IndirectCall`/`DirectCall` dispatch table treats
its name as known.

### Every Twig closure is strict

SIR10 ("Strict closures") gives a bare `MakeClosure` Ruby-block
semantics — a call may pass too many arguments (the extras are dropped)
or too few (the missing ones are `nil`) — and the `lambda` marker
around it lambda semantics: the call's argument count must equal the
function's (`Function::exact_arity()`), or the call raises an arity
error at run time.  Twig has no blocks, so this frontend never emits a
bare `MakeClosure`:

```scheme
(define g (begin (lambda (x y) (+ x y))))
(print (g 4))   ; arity error on every backend
```

> **Revision (issue #18461).** This frontend used to emit the bare
> `MakeClosure`, and each backend then did its own thing with the
> program above: C printed `4` (the missing `y` read as `nil`, which `+`
> counts as 0), JavaScript printed `NaN`, Go panicked with an index
> error, Rust failed later with "expected int, got nil".

### A top-level function used as a value

A name that resolves to a top-level function *outside* call position
(`h` in `(define g h)` or `(app h 5)`) lowers to a strict closure over
that function with no captures — a top-level function sees only
globals, functions and builtins, so there is nothing to capture:

```text
(define (h x) (* x 3))
(define g h)      ;  _init: global_set 'g (BuiltinCall lambda [MakeClosure h []])
(g 4)             ;  IndirectCall (VarRef g Global) [4]  →  12
```

> **Revision (issue #18461).** It used to lower to `VarRef { scope:
> Global }`, but functions are code, not entries in the globals store,
> so the read found an empty slot ("non-closure" on C and JavaScript,
> "undefined global" on TypeScript and Python).

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

### Names the compiler owns

`main`, `_init` and the closure-body gensyms `__lambda_<digits>` share
the function namespace with the user's definitions, so they are
reserved:

- *Binding* one is a `TwigLowerError` — by a top-level `define`, as a
  function or lambda parameter, or with `let` / `let*` — at the
  binding form's position: "`` `main` is reserved: the compiler
  generates a function with that name … ``".
- None of them is in the table user names resolve against, so every
  reference — `(main)`, `(print _init)`, `(define g __lambda_0)`,
  `(__lambda_0 5)` — is an "unresolved name" error, not a reach into
  generated code.
- Only the exact names are reserved: `main2`, `_init_count`,
  `__lambda_x` and `my-main` are ordinary.

> **Revision (security review of the #18461 fix, F1).** The first fix
> still added each generated `__lambda_<n>` to the resolution table, so
> `(define (adder n) (lambda (x) (+ x n))) (define g __lambda_0)` made a
> closure with no capture values over a body that expects one; the C
> backend read `caps[0]` through a NULL pointer.  Binding a reserved
> name as a parameter or `let` variable was also allowed, and collided
> with the function of that name in the emitted JS, Python and Go.  The
> validator now also refuses any `MakeClosure` whose capture count
> differs from its function's (SIR10 "Closure construction").

> **Revision (issue #18461).** The function table used to contain
> `main` and `_init`, so `(define main 5) (main)` lowered the bare
> `(main)` to a `DirectCall` to the *generated* `main` — which then
> called itself until the stack ran out.  Mangling the generated names
> instead was rejected: `main` / `_init` are the SIR-wide entry-point
> convention every backend looks up by name.

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
  the function's `captures`, then the module's `globals` table (value
  and function defines), then the builtin table.  The first hit wins
  and supplies the scope tag — except that a top-level *function*
  becomes a closure over it rather than a `VarRef` (see "A top-level
  function used as a value").  Call position uses the same order; see
  "Apply-site dispatch".

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
- Any `BuiltinCall("cons" | "car" | "cdr" | "pair?", ...)`, or one of
  those builtins used as a value → `Pairs`.  (Issue #18460: this rule
  was documented but not implemented, so the validator refused every
  Twig program that used a pair, and so did every validating backend.)
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

- Unresolved name reference (including every reference to `main`,
  `_init` or `__lambda_<digits>`)
- A binding of a reserved name (`main`, `_init`, `__lambda_<digits>`)
  by `define`, a parameter, or `let` / `let*`
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
- **Name resolution** (`tests/name_resolution.rs`): for each row of
  the apply-site table, programs that shadow a builtin or a top-level
  definition, with the expected call node pinned.  The same programs run
  end to end under `node` in `semantic-ir-to-javascript`'s
  `tests/twig_name_resolution.rs`.
- **Pairs, function values, strict closures and reserved names**
  (`tests/pairs_closures_and_entry_names.rs`), with every
  `semantic-ir-to-*` backend running the same programs on its real
  toolchain in `tests/twig_pairs_and_closures.rs` (`node`, `cc`,
  `go run`, `rustc`, `ruby`, `python3`): each must print the same
  answer, and each must stop a mis-called closure with an arity error.

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
