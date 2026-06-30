# Changelog

## 0.4.0 — SIR16 Sequences + Maps (completes SIR16 / v1 parity)

The final two SIR-v1 (SIR16) features land in the Rust backend.  With
`Sequences` and `Maps` now accepted, the Rust backend supports **all six**
SIR16 / SIR-v1 features — `Floats`, `ShortCircuit`, `MutableBindings`,
`Loops`, `Sequences`, `Maps` — reaching full v1 parity with the
TypeScript backend.  Every SIR16 IR node now has a real emit arm; the
only remaining `panic!`s cover SIR17/18 nodes (classes, modules,
singleton classes, try/catch, string interpolation, instance/class/const
vars, intrinsics) whose features stay unaccepted, so they are unreachable
for any validated module.

### Added

- `Feature::Sequences` and `Feature::Maps` in the accepted-feature set
  (`lib.rs`).
- **Runtime value model** — two new shared, mutable `Value` arms:
  - `Value::Seq(Rc<RefCell<Vec<Value>>>)` — a growable vector.  The
    `Rc<RefCell<…>>` is essential: `SeqSet` (`xs[i] = v`) must mutate the
    sequence the caller holds, and aliasing bindings must observe each
    other's writes — the reference semantics of a Python list / JS array.
  - `Value::Map(Rc<RefCell<Vec<(Value, Value)>>>)` — an insertion-ordered
    association list.  Keys compare with the runtime's own `value_eq`
    (linear scan) rather than a `HashMap`, because `Value` is neither
    `Hash` nor `Eq` (floats, closures, nested seqs/maps).  This gives
    correct lookup semantics for *any* key type and preserves insertion
    order for deterministic iteration and printing.
- **Sequences** — `SeqLit`/`SeqIndex`/`SeqLen` expressions lower to the
  `seq_lit`/`seq_index`/`seq_len` helpers; the `SeqSet` statement mutates
  the backing vector through `seq_set`.  Out-of-range index reads/writes
  panic (strict, like `car`/`cdr` on a non-pair).
- **Maps** — `MapLit`/`MapGet` expressions lower to `map_lit`/`map_get`;
  the `MapSet` statement mutates via `map_set`.  A missing-key `MapGet`
  returns `Nil` (mirroring the TypeScript backend's `?? null`).  Literal
  and `map_set` writes are last-write-wins on an existing key while
  preserving first-seen insertion order.
- **`format`** renders sequences as `[1, 2, 3]` and maps as `{a: 1, b: 2}`
  (insertion order); **`value_eq`** compares seqs/maps structurally
  (element-wise, with an `Rc::ptr_eq` fast path).

### Changed (ForEach reconciliation)

- A2 introduced `Stmt::ForEach` with a `seq_iter` helper that walked a
  cons-list (there was no `Seq` value yet).  Now that a real
  `Value::Seq` exists, `seq_iter` was reconciled to **snapshot a
  `Value::Seq`** as well as walk the legacy cons-list, so a
  `for x in [1, 2, 3]` (a `SeqLit`) iterates end to end while the
  cons-list path keeps working unchanged.
- Fixed a latent `ForEach` emit bug surfaced by the new end-to-end test:
  the loop emitted `for <var>: __sir::Value in …`, but a type annotation
  on a `for` pattern is not valid Rust.  Dropped the annotation
  (`for <var> in …`); the element type is already `Value` from
  `seq_iter`'s return.  This path had no prior compile-and-run coverage
  (the loops test exercised only `while`/`for-range`).
- The mutable-binding pre-pass (`collect_assigned_locals`) now recurses
  into Seq/Map statements and expressions, so an `Assign` nested inside a
  `SeqSet`/`MapSet` value (or a `SeqLit`/`MapLit` sub-expression) is
  still discovered and its binding declared `let mut`.

### Tests

- `tests/compile_and_run_seq_maps.rs`: an end-to-end proof that emits a
  module using a sequence (literal, index, len, set), a map (literal,
  get with a present *and* a missing key, set), and a `for v in <SeqLit>`
  `ForEach` accumulation, compiles it with `rustc`, runs the binary, and
  asserts its stdout (`20`, `3`, `99`, `2`, `nil`, `7`, `10`).
- Unit tests for every new emit arm (seq/map literal, index, len, get,
  and the seq/map set statements in both the block and inline paths),
  for `ForEach`-over-`SeqLit` composition, and for the mutable-name
  pre-pass recursing into a `SeqSet` value; runtime tests for the new
  value arms and helpers.

## 0.3.0 — SIR16 MutableBindings + Loops

The next two SIR-v1 (SIR16) features land in the Rust backend, matching
the TypeScript backend's existing support.  Until now `MutableBindings`
and `Loops` were undeclared and their IR nodes hit the `panic!` reject
group; this PR replaces those arms with real emission.

### Added

- `Feature::MutableBindings` and `Feature::Loops` in the accepted-feature
  set (`lib.rs`).
- **MutableBindings**: a per-function pre-pass (`collect_assigned_locals`)
  finds every name that is later the target of a `Stmt::Assign`.  A
  `LetBinding` for such a name is emitted as `let mut` (immutable bindings
  stay plain `let`), and `Stmt::Assign` then emits a bare
  `<name> = <value>;` for Local/Param/Capture scopes.  A `Global`-scoped
  assign writes through the runtime store
  (`__sir::global_set(&__sir::intern("name"), value)`).  Mirrors the
  TypeScript backend's `const`/`let` mutable-name tracking.
- **Loops** — all three loop statements emit real Rust:
  - `While { cond, body }` → `while __sir::truthy(&(<cond>)) { <body> }`,
    routing the test through SIR truthiness (only `false`/`nil` are
    falsy), never Rust's native `bool`.
  - `ForRange { var, start, stop, step, body }` → a numeric loop that
    caches `stop`/`step` into block-scoped `i64` temporaries (evaluated
    once, like Python's `range`), with a direction-aware condition so a
    negative `step` counts down.  The loop variable is rebound each
    iteration as a fresh `__sir::Value::Int`.  Fresh per-loop temp ids
    keep nested loops collision-free; the counter resets per module for
    deterministic output.
  - `ForEach { var, iter, body }` → `for <var> in __sir::seq_iter(&(<iter>))`.
    This backend has no dedicated `Seq` value yet (Sequences land in a
    later PR), so a "sequence" is the existing cons-list (`Pair`-chain
    terminated by `Nil`); `seq_iter` flattens it into a `Vec<Value>`.
    No `Feature::Sequences` runtime is required — the validator observes
    only `Feature::Loops` for `ForEach`, so accepting `Loops` covers all
    three loop forms with **no reachable `panic!`**.
- Runtime helpers `as_int` (public face of `as_i64`, for the `ForRange`
  bound temporaries) and `seq_iter` (cons-list → `Vec<Value>` for
  `ForEach`).

### Tests

- `tests/compile_and_run_loops.rs`: an end-to-end proof that emits a
  module using a `while` loop, two `for-range` accumulators, and mutable
  reassignment, compiles it with `rustc`, runs the binary, and asserts
  its stdout (`sum 0..5 = 10`, countdown ends at `0`, product `= 6`).
- Unit tests for each new emit arm: bare/global assign, `let mut`
  selection, while/truthy, for-range bound caching + int var binding +
  direction-aware condition + nested fresh ids, and for-each via
  `seq_iter`.

### Notes

- The remaining two SIR16 features (Sequences, Maps) are still
  undeclared; their `SeqSet`/`MapSet` and Seq/Map expression emit arms
  keep the `panic!` (unreachable via the capability check) until a later
  PR extends them.

## 0.2.0 — SIR16 Floats + ShortCircuit

The first two SIR-v1 (SIR16) features land in the Rust backend.  Until
now `Floats` and `ShortCircuit` were undeclared and their IR nodes hit
the `panic!` reject group; the TypeScript and Python backends already
supported them, so this closes part of the cross-backend parity gap.

### Added

- `Feature::Floats` and `Feature::ShortCircuit` in the accepted-feature
  set (`lib.rs`).
- Runtime value model gains `Value::Float(f64)`.  The arithmetic helpers
  (`plus`/`minus`/`times`/`divide`) stay on the exact i64 path while
  every operand is an integer and promote the whole fold to f64 as soon
  as any operand is a float (Python/Ruby/JS "int op float ⇒ float").
  `number?` now covers floats, `=` is cross-representation (`1 == 1.0`
  is true; `NaN == NaN` is false), and `<`/`>` compare numerically.
- `Expr::FloatLit` emits a `Value::Float` literal — `{:?}` keeps the
  trailing `.0` on integral values so the literal is never mistaken for
  an `i64`; non-finite values use `f64::NAN`/`INFINITY`/`NEG_INFINITY`.
- `Expr::LogicalAnd`/`Expr::LogicalOr` emit a truthy-guarded block
  (`{ let __l = lhs; if truthy(&__l) { ... } else { ... } }`) that
  evaluates the rhs only when the lhs decides — same semantics as the
  TypeScript backend's truthy-guarded arrow IIFE.

### Tests

- `tests/compile_and_run_floats.rs`: an end-to-end proof that emits a
  float + short-circuit module, compiles it with `rustc`, runs the
  binary, and asserts its stdout.

### Notes

- The remaining four SIR16 features (MutableBindings, Loops, Sequences,
  Maps) are still undeclared; their emit arms keep the `panic!`
  (unreachable via the capability check) until later PRs extend them.

## 0.1.2 — SIR18 exhaustiveness (no behaviour change)

semantic-ir 0.10.0 adds `Expr::StrConcat` (the SIR18 string-concat
node).  This backend gains a `StrConcat` arm in its expression emitter
so it stays exhaustive.  The arm joins the existing SIR16+ reject group
and `panic!`s with a "capability check should have rejected it"
message: `Feature::StringInterpolation` is not in this backend's
accepted-feature set, so a concat-using module is rejected at the
capability check before emit, making the arm unreachable.  No output or
accepted-feature changes.

## 0.1.1 — SIR17 exhaustiveness (no behaviour change)

semantic-ir 0.2.0 adds `Stmt::ClassDef` (the SIR17 class node).  This
backend gains a `ClassDef` match arm in its statement emitter so it
stays exhaustive.  The arm `panic!`s with a "capability check should
have rejected it" message: `Feature::Classes` is not in this
backend's accepted-feature set, so a class-using module is rejected
at the capability check before emit, making the arm unreachable.  No
output or accepted-feature changes.

## 0.1.0 — initial release (SIR13 v0)

Second backend for the narrow-waist Semantic IR.  Emits self-contained
Rust source from a `semantic_ir::Module`.

### Added

- `RustBackend` implementing `semantic_ir::Backend` with:
  - `target_tag() = "rust"`
  - `accepts_features()` covering the full v0 surface minus
    `TailCalls` and `Intrinsics`.
- `compile(module)` convenience function returning an
  `Artifact { filename, source, metadata }`.
- Per-node lowering rules per SIR13:
  - Literals → typed `__sir::Value::*` constructors.
  - Symbols → `__sir::intern("...")`.
  - VarRef Local/Param/Capture → `<name>.clone()`.
  - VarRef Global → `__sir::global_get_static("...")`.
  - VarRef Builtin → `__sir::builtin_closure("...")`.
  - If → Rust `if/else` with `__sir::truthy(&cond)`.
  - Block → Rust block expression `{ stmts...; value }`.
  - LetBinding / LetStarBinding → `let name: __sir::Value = ...;`.
  - DirectCall → `<fn>(<args>)`; SIR `main` is renamed to
    `__sir_user_main` to avoid collision with Rust's process entry.
  - IndirectCall → `__sir::apply_closure(&target, vec![args])`.
  - BuiltinCall → typed helper or `call_builtin_by_name` fallback.
  - MakeClosure → `__sir::Value::Closure(Rc::new(__sir::Closure {
    fun: Box::new(move |args| <fn>(<captures>, <pos-args>)) }))`.
- Inlined `__sir` runtime (~280 lines) covering:
  - `Value` enum, `Pair` struct, `Closure` wrapping a `Box<dyn Fn>`.
  - `intern` / `apply_closure` / `truthy` / `format`.
  - All v0 builtins (`plus`, `minus`, `times`, `divide`, `eq`,
    `lt`, `gt`, `cons`, `car`, `cdr`, `is_null`, `is_pair`,
    `is_number`, `is_symbol`, `print`).
  - `thread_local!` storage for globals + symbol interning.
  - `call_builtin_by_name` dispatch for VarRef Builtin and
    forward-compat new builtins.
- Identifier sanitisation:
  - Valid Rust identifiers pass through.
  - Rust keywords (`fn`, `type`, `match`, etc.) get the `r#`
    raw-identifier prefix so the original spelling stays visible.
  - Other invalid characters (`?`, `!`, `-`, `+`, `*`) are encoded
    as `_<hex>` underscore-escaped forms.
  - Empty input becomes `"_$empty"`.
  - SIR's `main` is specially renamed to `__sir_user_main`.
- Function arity table threaded via TLS so `MakeClosure` knows
  how many positional arguments to drain from the runtime args
  iterator when calling the synthesised lambda function.
- `sanitize_comment` strips line terminators (`\n`, `\r`, U+0085,
  U+2028, U+2029) from any external string written into `//`
  comments, mirroring the TypeScript backend's defense.
- Pre-lowering validation via `semantic_ir::validate`; capability
  check via the `Backend::check_module` default impl.

### Deferred

- Static type narrowing.  Optional SIR types widen to `Value`.
- `no_std` / `alloc`-only target.
- Source-map generation (function-level comments only).
- Raw-Rust intrinsic embedding.
- Async / `await` support (no SIR async surface yet).
