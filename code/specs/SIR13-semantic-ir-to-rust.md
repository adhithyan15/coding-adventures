# SIR13 — Semantic IR → Rust (Rust backend)

## Status

Second backend for the narrow-waist Semantic IR
([SIR10](SIR10-narrow-waist-semantic-ir.md)).  Where SIR12 emits
TypeScript, this spec emits Rust source code.  Implemented as the
Rust crate
[`semantic-ir-to-rust`](../packages/rust/semantic-ir-to-rust/).

The crate consumes a [`semantic_ir::Module`] and emits a single
**self-contained** Rust source file — no external crate
dependencies; all runtime helpers live in an inlined `__sir`
module.  The output is a complete Rust program that compiles with
`rustc <file>.rs` and runs.

## Overview

```text
semantic_ir::Module                 (narrow-waist SIR v0)
   │
   ▼  semantic_ir_to_rust::compile
Artifact { filename, source, metadata }
        │
        ▼ source is a single .rs file
        │
        ▼ no external crate dependencies
        ▼ inlined __sir module covers all Twig builtins + value model
```

## Public API

```rust
use semantic_ir::{Backend, Module, Artifact, BackendError};

pub struct RustBackend { /* ... */ }

impl RustBackend {
    pub fn new() -> Self;
}

impl Backend for RustBackend {
    fn target_tag(&self) -> &'static str { "rust" }
    fn accepts_features(&self) -> &'static [Feature] { ... }
    fn accepts_intrinsics(&self) -> &'static [&'static str] { &[] }  // v0
    fn compile(&self, module: &Module) -> Result<Artifact, BackendError>;
}

pub fn compile(module: &Module) -> Result<Artifact, BackendError>;
```

## Capability declaration (v0)

The Rust backend accepts:

- `Closures` — lowered to `Rc<__sir::Closure>` handles wrapping a
  `Box<dyn Fn(Vec<Value>) -> Value>` that closes over the captured
  values.
- `Pairs` — `Rc<__sir::Pair>`.
- `Symbols` — interned `Rc<str>` strings.
- `Strings` — `Rc<str>`.
- `DynamicTyping` — every value is `__sir::Value`, a tagged enum
  union.
- `OptionalTypeAnnotations` — accepted; current v0 widens to
  `Value` regardless (no static narrowing yet).
- `MutualRecursion` — Rust function items at module scope can
  reference each other freely.
- `Globals` — module-level `thread_local!` storage in `__sir`,
  written via the `global_set` builtin in the synthesised `_init`
  function, read directly through `VarRef { scope: Global }` →
  generated `&str` lookups.

It **rejects**:

- `TailCalls` — Rust does not guarantee TCO.  Frontends that
  require it must target a different backend.
- `Intrinsics` — the v0 backend has an empty intrinsic whitelist.
  A future revision may add `rust`-tagged intrinsics for raw-Rust
  embedding.

## Value model

Every Twig value lowers to a `__sir::Value`:

```rust
#[derive(Clone)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Nil,
    Sym(Rc<str>),
    Str(Rc<str>),
    Pair(Rc<Pair>),
    Closure(Rc<Closure>),
}

pub struct Pair { pub car: Value, pub cdr: Value }

pub struct Closure {
    pub fun: Box<dyn Fn(Vec<Value>) -> Value + 'static>,
}
```

- `Int` is a native `i64` (matches SIR's `Int`).
- `Sym` and `Str` are `Rc<str>` for cheap clones (symbol interning
  via a `thread_local!` `HashMap<String, Rc<str>>` in `__sir`).
- `Pair` and `Closure` are heap-allocated via `Rc`.
- Closures wrap a heap-allocated `dyn Fn` that closes over the
  captured values.  `Closure` is intentionally `!Clone`; the
  cloneable thing is the `Rc<Closure>` handle (which clones the
  refcount, never the function body).
- The runtime is single-threaded.  `Rc` (not `Arc`) is used
  deliberately: no `Send` / `Sync` overhead.

## Closure ABI

Each SIR `Function` with non-empty `captures` is emitted as a Rust
function that takes captures-then-params in positional order:

```rust
fn __lambda_0(captured_n: __sir::Value, x: __sir::Value) -> __sir::Value {
    __sir::plus(vec![x, captured_n])
}
```

The corresponding `MakeClosure { fn_name = "__lambda_0",
captures = [{n: <value_expr>}] }` lowers to:

```rust
{
    let __cap_n: __sir::Value = /* lowered value_expr */;
    __sir::Value::Closure(::std::rc::Rc::new(__sir::Closure {
        fun: Box::new(move |__args: Vec<__sir::Value>| {
            let mut __it = __args.into_iter();
            __lambda_0(
                __cap_n.clone(),
                __it.next().unwrap_or(__sir::Value::Nil),
            )
        }),
    }))
}
```

An `IndirectCall` runs through `__sir::apply_closure(target, args)`,
which dispatches the boxed `dyn Fn` after a runtime check that
`target` is a `Value::Closure`.

## Per-node lowering rules

| SIR node                    | Emitted Rust                                              |
|-----------------------------|-----------------------------------------------------------|
| `IntLit { value }`          | `__sir::Value::Int(<value>)`                              |
| `BoolLit { value }`         | `__sir::Value::Bool(<value>)`                             |
| `NilLit`                    | `__sir::Value::Nil`                                       |
| `SymLit { name }`           | `__sir::intern("<name>")`                                 |
| `StrLit { value }`          | `__sir::Value::Str(::std::rc::Rc::from("<escaped>"))`     |
| `VarRef { name, Local }`    | `<name>.clone()`                                          |
| `VarRef { name, Param }`    | `<name>.clone()`                                          |
| `VarRef { name, Capture }`  | `<name>.clone()`                                          |
| `VarRef { name, Global }`   | `__sir::global_get_static("<name>")`                      |
| `VarRef { name, Builtin }`  | `__sir::builtin_closure("<name>")`                        |
| `If { cond, then, else }`   | `(if __sir::truthy(&<cond>) { <then-block> } else { <else-block> })` |
| `Block { stmts, value }`    | `{ <stmts>; <value> }`                                    |
| `LetBinding`                | `let <name>: __sir::Value = <value>;`                     |
| `LetStarBinding`            | `let <name>: __sir::Value = <value>;`                     |
| `ExprStmt`                  | `<expr>;`  *(or `let _ = <expr>;` if its value is unused)*|
| `DirectCall { fn, args }`   | `<fn>(<args>)`                                            |
| `IndirectCall { tgt, args}` | `__sir::apply_closure(&<tgt>, vec![<args>])`              |
| `BuiltinCall { name, args}` | `__sir::<helper>(vec![<args>])`  *(some have fixed arity)*|
| `MakeClosure { fn, caps }`  | See "Closure ABI" above                                   |

Note: SIR `VarRef { Local | Param | Capture }` emit `<name>.clone()`
rather than a bare `<name>` because `Value::Closure` carries a non-
copyable `Rc<dyn Fn ...>` and is `Clone` not `Copy`.  All values are
passed by `Clone` to keep the lowering simple; the optimizer is
expected to elide cheap `Rc` clones at codegen time.

## Per-builtin dispatch

The Rust backend emits direct calls to typed helpers in `__sir`:

| Builtin name | Helper             | Arity      | Notes                                      |
|--------------|--------------------|------------|--------------------------------------------|
| `+`/`-`/`*`/`/`| `plus`/`minus`/`times`/`divide` | variadic | take `Vec<Value>`, return `Value::Int`     |
| `=`/`<`/`>`  | `eq`/`lt`/`gt`     | 2          | return `Value::Bool`                       |
| `cons`/`car`/`cdr` | same           | 2 / 1 / 1  | construct / destructure `Pair`             |
| `null?` / `pair?` / `number?` / `symbol?` | same | 1 | predicate, returns `Value::Bool`          |
| `print`      | `print`            | 1          | writes formatted value to stdout, returns `Nil` |
| `global_set` / `global_get` | same    | 2 / 1      | thread-local global table                  |

Unknown builtin names fall through to a dispatch table
(`__sir::call_builtin_by_name(name, args)`) so forward-compat new
builtins land without breaking existing modules.

## Generated file layout

```rust
// Generated by semantic-ir-to-rust v0.1 from SIR module `<name>`.
// Source language: <metadata.source_language>
// Do not edit by hand.

#![allow(non_snake_case, unused_imports, unused_parens, dead_code, clippy::all)]

mod __sir {
    // ~150 lines of inlined runtime: Value, Pair, Closure, intern,
    // apply_closure, truthy, plus/minus/times/divide, eq/lt/gt,
    // cons/car/cdr, null?/pair?/number?/symbol?, print, format,
    // global_set/global_get, dispatch table.
}

// User-defined functions and globals follow:
fn <user-fn>(/* params */) -> __sir::Value { ... }

// Synthesised entry:
fn main() {
    let _ = _init();   // if module has _init
    let _ = r#main();  // SIR's `main` function — Rust's reserved
                       // word handled via raw-identifier syntax
                       // (sanitize_ident).
}
```

### `main` collision

SIR's synthesised `main` function clashes with Rust's program entry
`main`.  The emitter renames the SIR function to `__sir_user_main`
and emits a real `main` that calls it (and `_init` first if
present).  The same rule applies to any other SIR function name
that is a Rust keyword — `sanitize_ident` handles this.

## Output formatting

- Indentation: 4 spaces (matches `rustfmt` default; output can be
  fed straight to `rustfmt` if desired but doesn't require it).
- Line endings: `\n`.
- Generated file ends with a trailing newline.
- A `// SIR span: <span>` comment is inserted before each function
  declaration for source-trace.  As in SIR12, the span string is
  passed through `sanitize_comment` to prevent `//`-comment escape
  via embedded newlines or U+2028/U+2029.

## Validation

The backend calls `semantic_ir::validate(module)` before lowering.
Failed validation produces `BackendError { kind: InvalidModule, ... }`.
The default `Backend::check_module` capability check runs next
(manifest features + intrinsic whitelist + target tag).  Only after
both pass does emission begin.

## Tests

- **Unit tests** for each node-kind lowering rule.
- **Golden tests** comparing canonical Twig programs' emitted `.rs`
  against fixture files.
- **Execution tests** — invoke `rustc` on the emitted source (if
  available on the CI box) and run the resulting binary,
  comparing stdout to expected output.  Skipped with a message if
  `rustc` is not on PATH (defensive: keeps unit tests green even
  on minimal runners).
- **End-to-end** integration with `twig-to-semantic-ir`:
  factorial, closure adder, higher-order programs lower from Twig
  through SIR to Rust.

Coverage target: **≥ 95%**.

## Out of scope (deferred)

- **Static type narrowing.**  Optional SIR types currently widen to
  `Value`; a future revision could emit `i64` for typed numeric
  vars.
- **`no_std` / `alloc`-only target.**  v0 uses `std::rc`,
  `std::cell`, `std::collections`, `std::io::stdout`.
- **`#[inline]` / optimisation hints.**
- **Source-map generation.**  Only function-level comments today.
- **Raw-Rust intrinsic injection.**  Designed for a future
  revision; v0 rejects all intrinsics.
- **`async fn` support.**  No SIR async support yet (SIR10 v0
  defers async).
