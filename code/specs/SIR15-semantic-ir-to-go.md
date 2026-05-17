# SIR15 — Semantic IR → Go (Rust backend)

## Status

Fourth backend for the narrow-waist Semantic IR.  Joins
[SIR12](SIR12-semantic-ir-to-typescript.md) (TypeScript),
[SIR13](SIR13-semantic-ir-to-rust.md) (Rust), and
[SIR14](SIR14-semantic-ir-to-python.md) (Python).  Implemented as the
Rust crate `semantic-ir-to-go`.

The crate consumes [`semantic_ir::Module`] and emits a **self-contained
Go source file** — `package main` plus inlined runtime helpers; no
external module imports required.  The output compiles with
`go build <file>.go` and runs.

## Public API

```rust
pub fn compile(module: &semantic_ir::Module)
    -> Result<semantic_ir::Artifact, semantic_ir::BackendError>;

pub struct GoBackend;
impl semantic_ir::Backend for GoBackend {
    fn target_tag(&self) -> &'static str { "go" }
    ...
}
```

## Capability declaration

Accepts: `Closures`, `Pairs`, `Symbols`, `Strings`, `DynamicTyping`,
`OptionalTypeAnnotations`, `MutualRecursion`, `Globals`.

Rejects: `TailCalls` (Go does not guarantee TCO), `Intrinsics`
(empty whitelist).

## Value model

```go
type Value interface{}

type Symbol struct { Name string }
type Pair   struct { Car, Cdr Value }
type Closure struct { Fn func(args []Value) Value }
```

- `Value` is Go's empty interface — matches the SIR `Any` widening.
- Symbols are interned via a `map[string]*Symbol` (pointer equality
  on the interned `*Symbol` gives O(1) `eq?`).
- `Pair` and `Closure` are referenced by pointer (`*Pair`, `*Closure`)
  so cloning a `Value` is a pointer copy.
- Globals: `map[string]Value`.

## Block-as-expression

Go has no expression form for multi-statement blocks.  Non-trivial
SIR `Block`s lower to an immediately-invoked function expression:

```go
func() Value {
    x := someExpr
    return _sir_plus([]Value{x, intLit(2)})
}()
```

Trivial blocks (empty `stmts`) render the value expression inline.

## Per-node lowering rules

| SIR node                      | Emitted Go                                                  |
|-------------------------------|-------------------------------------------------------------|
| `IntLit { value }`            | `int64(<value>)` *cast for `Value`-shaped contexts*         |
| `BoolLit { value }`           | `<value>`                                                   |
| `NilLit`                      | `nil`                                                       |
| `SymLit { name }`             | `_sir_intern("<name>")`                                     |
| `StrLit { value }`            | `"<escaped>"`                                               |
| `VarRef { Local | Param | Capture }` | `<name>`                                            |
| `VarRef { Global }`           | `_sir_global_get_static("<name>")`                          |
| `VarRef { Builtin }`          | `_sir_builtin_closure("<name>")`                            |
| `If`                          | IIFE: `func() Value { if _sir_truthy(cond) { return then } else { return else } }()` |
| `Block` (with stmts)          | IIFE: `func() Value { stmts...; return value }()`           |
| `LetBinding` / `LetStarBinding` | `<name> := <value>`                                        |
| `DirectCall`                  | `<fn>(<args>)`                                              |
| `IndirectCall`                | `_sir_apply(<target>, []Value{<args>})`                     |
| `BuiltinCall`                 | `_sir_<helper>([]Value{<args>})` or fixed-arity            |
| `MakeClosure`                 | `_sir_make_closure(<fn>, []Value{<cap-values>})`            |

## Identifier sanitisation

Go identifiers match `[A-Za-z_][A-Za-z0-9_]*`.  Same rules as the
Python backend, except keywords get an underscore prefix `Go_` to
keep the identifier exported-by-default-style (capitalised) when
the original was; for unexported names a simple suffix suffices.

SIR's `main` is renamed to `_sir_user_main` so it does not collide
with Go's process entry `main()`.

## Output formatting

- Indentation: tab (`\t`) per Go convention.
- Line endings: `\n`.
- Function-span comments before each `func`.
- `sanitize_comment` strips line terminators per the SIR12 / SIR13
  defence.

## Tests

`cargo test -p semantic-ir-to-go` covers per-node lowering rules,
identifier sanitisation, deterministic output, and end-to-end
pipelines from Twig source.

## Out of scope

- Static-type narrowing (every value is `Value`).
- Goroutines / channels.
- Source maps.
- Raw-Go intrinsic injection.
