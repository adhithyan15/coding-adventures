# Changelog

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
