# Changelog

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
