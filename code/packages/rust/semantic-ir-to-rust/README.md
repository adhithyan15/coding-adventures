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

Accepts (P2e): `DefaultParams` — a `Param` may carry a `default`
expression that runs when the caller omits that trailing argument.  Rust
functions are fixed-arity over `__sir::Value` with no native defaults, so
the backend uses a **runtime-mimic** strategy that preserves the
language's **call-time, param-scope** semantics:

- A `Value::Missing` sentinel marks an *omitted* positional argument,
  with `__sir::missing()` (constructor) and `__sir::is_missing(&Value)`
  (predicate) helpers.  `format` renders a stray `Missing` as `<missing>`
  and `value_eq` treats it as equal only to another `Missing`.
- Each defaulted param gets a **body-top prologue**
  `let p = if __sir::is_missing(&p) { <default> } else { p };`.  Emitting
  the default *inside the body* — in declaration order — is what gives the
  call-time + param-scope rule: an earlier parameter is already bound, so
  a default like `b = a + 1` resolves `a`.
- A `DirectCall` that omits trailing defaulted arguments **pads** the
  omitted positions with `__sir::missing()` so the emitted Rust call is
  full-arity.  The callee's parameter count comes from the same
  `FN_ARITY` table the closure emitter uses.

Non-default behaviour is byte-for-byte unchanged.

Accepts (KW5): `KeywordParams` — name-matched keyword parameters
(`def f(a:)` / `def f(a: 1)`) and keyword arguments (`f(a: 1)`).  Rust has
**no native keyword-argument syntax**, so the backend resolves keywords to
a plain positional call **statically, at emit time** (no runtime library):

- **Def side** — a `Keyword` param emits as an *ordinary positional*
  Rust parameter in its declared order (the by-name affordance is dropped;
  the name becomes the Rust parameter name).  An *optional* keyword (one
  with a `default`) reuses the same `DefaultParams` body-top prologue —
  it is a defaulted parameter like any other.
- **Call side** — for a `DirectCall` whose callee signature is known
  (looked up in a `FN_PARAMS` signature table, populated alongside
  `FN_ARITY`), the emitter builds the full positional argument list in the
  callee's *declared* order: positionals fill positional params in order;
  each `KeywordArg { name, value }` fills the param whose name matches
  (a name→position reorder); an omitted optional keyword is padded with
  `__sir::missing()` so the callee's prologue substitutes its default
  (deferring default evaluation to callee scope — correct even when a
  default references an earlier param).  The result is a plain positional
  Rust call `f(a, b_val, c_default)`.

Worked example — `def greet(greeting, name: "world")`:

```text
  greet("hi")               → greet("hi", missing())   // name omitted → default "world"
  greet("hi", name: "ada")  → greet("hi", "ada")        // name supplied by keyword
```

Out of scope (v0): an `IndirectCall`/closure call carrying keywords has no
statically-known signature, so it cannot be resolved; the frontends do not
emit it.

Executes (C6): **collection-method dispatch**.  A source-level
`recv.meth(args…)` / `recv.meth { |x| … }` reaches every backend as the
narrow-waist envelope
`BuiltinCall("__method__", [recv, StrLit("meth"), …args, block?])`.  This
needs **no new feature gate** — `__method__` observes no dedicated feature,
and a pure collection module's observed features (`Sequences`/`Closures`/
`Strings`) are already accepted.  The backend now *executes* the dispatch
rather than panicking:

- **Emit** — the `"__method__"` arm lowers to
  `__sir::call_method(<recv>, "meth", vec![<args>])`.  The method name is
  lifted out of the `StrLit` to a Rust `&str` **literal**, so dispatch is a
  closed, compile-time-known set.  A trailing block (a `MakeClosure`, or an
  `&:sym` block-pass lowered via the `"block_pass"` arm to
  `__sir::sym_to_proc`) is the last `Value::Closure` in the arg `Vec`.
- **Runtime catalog** — `call_method` in the inline `__sir` module matches
  **explicitly** on `(receiver type, method name)`, ported from the
  Python/TS `sir-runtime-oop` reference for parity:
  - **Array** (`Seq`): `length`/`size`, `first`, `last`, `push`/`append`,
    `pop`, `include?`, `reverse`, `sort`, `join`, `map`, `select`/`filter`,
    `reject`, `find`, `reduce`/`inject`, `each`, `any?`/`all?`/`none?`.
  - **Hash** (`Map`): `keys`, `values`, `size`, `has_key?`, `each`, `map`,
    `select`.
  - **String** (`Str`): `length`, `upcase`, `downcase`, `reverse`, `strip`,
    `include?`, `split`.
  - **Numeric** (`Int`/`Float`): `abs`, `to_i`, `to_f`, `even?`, `odd?`,
    `zero?`, `times`; plus a universal `to_s`.
  - Block-taking methods apply the trailing closure via `apply_closure`;
    `sym_to_proc` implements `Symbol#to_proc` (`&:sym`).
- **Security** — the catalog *is* the allowlist.  Dispatch is the explicit
  match only; an unknown method name returns a controlled Ruby `nil`
  (`unknown_method`), never a reflective lookup on the raw name.  No new
  `unsafe`.

Rejects: `TailCalls` (Rust does not guarantee TCO), `Intrinsics`
(empty whitelist in v0), and the SIR17/18 features above.

## Value model

```rust
#[derive(Clone)]
enum Value {
    Int(i64), Float(f64), Bool(bool), Nil,
    Missing,                                 // P2e DefaultParams sentinel
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
