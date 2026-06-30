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

Accepts (SIR16 / v1 — **all six** features, full v1 parity): `Floats`,
`ShortCircuit`, `MutableBindings`, `Loops`, `Sequences`, `Maps`.

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
  `for-each` iterates via the runtime `seq_iter` helper, which now
  snapshots a real `Value::Seq` **and** still walks the legacy cons-list
  (`Pair`-chain) — so `for x in [1, 2, 3]` works end to end.
- `Sequences` adds a shared, mutable `Value::Seq(Rc<RefCell<Vec<Value>>>)`.
  `SeqLit`/`SeqIndex`/`SeqLen` lower to `seq_lit`/`seq_index`/`seq_len`;
  the `SeqSet` statement mutates the backing vector via `seq_set`.
- `Maps` adds a shared, mutable, insertion-ordered
  `Value::Map(Rc<RefCell<Vec<(Value, Value)>>>)`.  `MapLit`/`MapGet`
  lower to `map_lit`/`map_get`; `MapSet` mutates via `map_set`.  Keys
  compare with the runtime's `value_eq` (so any value type is a key), and
  a missing-key `MapGet` returns `Nil`.

With all six SIR16 features accepted, every SIR16 IR node has a real emit
arm — no reachable `panic!` remains for v1.  The remaining emit panics
cover SIR17/18 nodes (classes, modules, try/catch, string interpolation,
instance/class/const vars) whose features stay unaccepted.

Rejects: `TailCalls` (Rust does not guarantee TCO), `Intrinsics`
(empty whitelist in v0), and the SIR17/18 features above.

## Value model

```rust
#[derive(Clone)]
enum Value {
    Int(i64), Float(f64), Bool(bool), Nil,
    Sym(Rc<str>), Str(Rc<str>),
    Pair(Rc<Pair>),
    Closure(Rc<Closure>),
    Seq(Rc<RefCell<Vec<Value>>>),            // SIR16 Sequences
    Map(Rc<RefCell<Vec<(Value, Value)>>>),   // SIR16 Maps (insertion-ordered)
}
```

- Single-threaded (`Rc`, not `Arc`).
- Closures wrap a `Box<dyn Fn(Vec<Value>) -> Value>` inside an `Rc`.
- Symbols and strings are interned `Rc<str>` for cheap clones.
- Globals live in a `thread_local!` `HashMap<String, Value>`.
- Sequences and maps are `Rc<RefCell<…>>` so `SeqSet`/`MapSet` mutate the
  shared value in place; maps key by `value_eq` (linear lookup) and keep
  insertion order.

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
