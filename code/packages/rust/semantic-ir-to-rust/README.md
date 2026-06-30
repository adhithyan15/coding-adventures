# semantic-ir-to-rust

Second backend for the narrow-waist Semantic IR.  Lowers
[semantic-ir](../semantic-ir/) modules into **self-contained** Rust
source code — every produced `.rs` file embeds the runtime helpers
inline as a `mod __sir { ... }` block, so the output compiles with
`rustc <file>.rs` and has no external crate dependencies.

Implements [SIR13](../../../specs/SIR13-semantic-ir-to-rust.md).

## Pipeline

```text
semantic_ir::Module
   │
   ▼  semantic_ir_to_rust::compile
Artifact { filename, source, metadata }
```

## Public API

```rust
use semantic_ir_to_rust::{compile, RustBackend};
use semantic_ir::Backend;

let artifact = compile(&sir_module)?;
// or:
let backend = RustBackend::new();
let artifact = backend.compile(&sir_module)?;
```

## Capability declaration

Accepts (v0): `Closures`, `Pairs`, `Symbols`, `Strings`,
`DynamicTyping`, `OptionalTypeAnnotations`, `MutualRecursion`,
`Globals`.

Accepts (SIR16 / v1, landing incrementally): `Floats`, `ShortCircuit`,
`MutableBindings`, `Loops`.

- `Floats` adds a `Value::Float(f64)` arm to the runtime value model with
  numeric promotion (`int op float ⇒ float`).
- `ShortCircuit` `&&`/`||` emit a truthy-guarded block so the rhs is
  evaluated only when the lhs decides.
- `MutableBindings` lets a `let`-binding be re-targeted by a later
  assignment: a per-function pre-pass declares every reassigned name
  `let mut`, and the assignment emits a bare `<name> = <value>;`
  (Local/Param/Capture) or a runtime `global_set` (Global).
- `Loops` covers `while`, `for-range`, and `for-each`.  `while` and
  `for-range` route through SIR truthiness / cached `i64` bounds;
  `for-each` iterates a cons-list (`Pair`-chain) via the runtime
  `seq_iter` helper, so it needs no dedicated `Sequences` value yet.

The other two v1 features (`Sequences`, `Maps`) are not yet accepted and
are still rejected at the capability check.

Rejects: `TailCalls` (Rust does not guarantee TCO), `Intrinsics`
(empty whitelist in v0).

## Value model

```rust
#[derive(Clone)]
enum Value {
    Int(i64), Float(f64), Bool(bool), Nil,
    Sym(Rc<str>), Str(Rc<str>),
    Pair(Rc<Pair>),
    Closure(Rc<Closure>),
}
```

- Single-threaded (`Rc`, not `Arc`).
- Closures wrap a `Box<dyn Fn(Vec<Value>) -> Value>` inside an `Rc`.
- Symbols and strings are interned `Rc<str>` for cheap clones.
- Globals live in a `thread_local!` `HashMap<String, Value>`.

## `main` collision

SIR's synthesised `main` function is renamed to `__sir_user_main`
in the generated Rust because `main` is Rust's process entry
point.  The emitter generates its own `main()` that calls `_init()`
(if present) then `__sir_user_main()`.

## Tests

`cargo test -p semantic-ir-to-rust`

Covers per-node lowering, identifier sanitisation (including
raw-identifier syntax for Rust keywords), deterministic output,
and end-to-end pipelines from Twig source.

## Related crates

- [`semantic-ir`](../semantic-ir/) — the IR itself
- [`twig-to-semantic-ir`](../twig-to-semantic-ir/) — first frontend
- [`semantic-ir-to-typescript`](../semantic-ir-to-typescript/) —
  sister backend
