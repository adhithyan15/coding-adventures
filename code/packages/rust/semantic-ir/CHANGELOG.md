# Changelog

All notable changes to the `semantic-ir` crate are documented here.

## 0.12.0 — Param default values (core IR representation; behavior-neutral)

Adds the IR representation for **default parameter values** — the `1` in
Ruby `def f(a = 1)` and the Python / JavaScript equivalents. Previously a
parameter had no place to carry its default, so frontends silently dropped
it. This is **PR 1 of a sequence**: the IR can now *represent* a default,
the whole workspace still compiles, and **all existing behavior is
unchanged** (no frontend produces a default yet, and no backend emits one
yet). Backend emission and frontend lowering land in follow-up PRs.

### Added

- `Param.default: Option<Box<Expr>>` — `None` for an ordinary parameter;
  `Some(expr)` for `name = expr`. Boxed to keep `Param` a fixed size despite
  the recursive `Param → Expr → Function → Param` cycle (a default
  expression may contain a closure whose own params have defaults).
- `Feature::DefaultParams` (text name `default-params`) — observed by the
  validator whenever any param carries a default. **Not yet accepted by any
  backend**, so a default-using module is correctly rejected by the
  capability check until each backend gains support (intended).
- Validator: when a param has a default it observes `Feature::DefaultParams`
  and recursively validates the default `Expr` against the parameters
  declared *so far* (a default may reference an earlier param but not a
  later one).
- Walker: `walk_function_default` now visits each param's default
  expression before the body, so passes that walk the IR see them.
- Text printer: a param with a default renders an extra `(default <expr>)`
  clause — `(a any (default (int 1)))` — while defaultless params keep the
  original `(name type)` shape, so existing modules print unchanged.
- Unit tests: default validates + observes the feature, default-expr
  features are observed, a default may reference an earlier param (and may
  not reference a later one), the walker visits the default expr, and the
  printer renders the default clause.

### Changed

- `Param` now derives only `PartialEq` (not `Eq`): it holds an `Expr`, which
  contains an `f64` (`FloatLit`) and so cannot be `Eq` — consistent with
  `Expr` / `Block`.
- Every literal `Param { … }` construction across the SIR backends
  (typescript/rust/python/go/javascript) and the twig/ruby frontends now
  sets `default: None`. This is a mechanical addition; backends read params
  by field access, so the new field does not affect their behavior.

## 0.11.0 — SIR19: variadic parameter kinds (`Param.kind` / `ParamKind`) (M3)

Closes the def-side variadic limitation: previously a splat parameter
(`def f(*rest)` / `def g(**opts)`) lost its splat-ness at the SIR level and
lowered to an ordinary positional `Param`, so the emitted Python/TypeScript
declared a fixed positional parameter and a variadic call (`f(1, 2, 3)`) broke.

### Added

- `ParamKind` enum — `Required` (default), `Rest` (`*rest`), `KwRest`
  (`**opts`) — re-exported from the crate root.
- `Param.kind: ParamKind` — a new field on `Param`. Every in-tree construction
  sets it explicitly.
- Validator rules (`validate`): at most one `Rest` and at most one `KwRest`
  per parameter list, and ordering — required positionals precede the lone
  `Rest`, which precedes the lone `KwRest`. The reserved trailing block
  parameter `__sir_block__` (Q9e) is exempt (always `Required`, always last).
- Text printer renders `*name` / `**name` for the two variadic kinds so a
  round-tripped module preserves splat-ness.

### Changed

- Every literal `Param { … }` construction (smoke tests, printer tests) now
  sets `kind`. Backends read params by field access, so the added field does
  not affect their reads — only constructions.

## 0.10.0 — SIR18: string interpolation (`Expr::StrConcat`)

Introduced by the Ruby frontend's Phase 20b (`"a#{x}b"` interpolation).

### Added

- `Expr::StrConcat { parts, span }` — a first-class string-concatenation
  node that replaces the v0 `BuiltinCall("string_concat", parts)` marker
  (the same marker→node move `Stmt::TryCatch` made for
  `__rescue_marker__`).  A dedicated node lets backends emit native
  string building (`format!` / template literals / f-strings) instead of
  routing through a runtime helper.  Invariant: `parts.len() >= 2`.
- `Feature::StringInterpolation` — observed whenever a module contains a
  `StrConcat` node.  Distinct from `Feature::Strings` (a plain `StrLit`):
  a backend may support string literals without yet knowing how to build
  a concatenation, so the two capabilities are tracked separately.

### Validator

- `StrConcat` observes `Feature::StringInterpolation` and recursively
  checks every part.  A concat with fewer than two parts is a hard error
  (a frontend should emit the bare part instead).

### Text format

- `print_expr` renders `StrConcat` as `(str-concat <part…>)` (kind name
  `str-concat`).  Span and visitor (`walker`) coverage extended to the
  new node.

## 0.9.0 — SIR17: structured exception handling (`Stmt::TryCatch`)

Introduced by the Ruby frontend's Phase 16a (`begin/rescue/ensure/end`).

### Added

- `Stmt::TryCatch { body, rescues, ensure_body, span }` — a first-class
  exception-handling statement that replaces the earlier
  `__rescue_marker__` / `__ensure_marker__` inline `BuiltinCall`
  placeholders.  `body` and `ensure_body` are bare statement lists (like
  `ClassDef.body`); `ensure_body` is `Option`al.
- `RescueClause { exception_types, binding, body, span }` — one `rescue`
  clause.  `exception_types` is a list of advisory class names (empty =
  bare catch-all, not resolved by the validator); `binding` is the
  optional `=> e` exception variable, in scope as a `Scope::Local`
  within that clause's `body` only.
- `Feature::Exceptions` (kebab `exceptions`) — declared by any module
  containing a `TryCatch`.  Backends that don't accept it reject the
  module at the capability check before emit.

### Changed

- `Stmt::span()`, the walker, the validator (`check_stmt_seq`), the text
  printer (`(try-catch … (rescue (types …) (bind …) …) (ensure …))`),
  `backend::walk_intrinsics_in_stmt`, and all four reference backends'
  statement-emit match gain a `Stmt::TryCatch` arm.  The validator walks
  the body, each rescue body (with the binding introduced as a local),
  and the ensure body in fresh local-env scopes; the backend arms are
  unreachable `panic!`s (rejected pre-emit by the capability check).

New tests (4): `print_try_catch_with_rescue_and_ensure`,
`try_catch_validates_and_binding_is_in_scope`,
`try_catch_without_manifest_feature_is_error`,
`try_catch_binding_does_not_leak_past_rescue`.  Test count: 102 → 106.

This is a **breaking enum change** for any exhaustive `match` on `Stmt`
without a `_` rest arm.

## 0.8.0 — SIR17: constant scope (`Scope::Const`)

Introduced by the Ruby frontend's Phase 15c (`FOO` / `MyClass`).

### Added

- `Scope::Const` — a constant (Ruby `FOO`, `MyClass` — any
  uppercase-initial name).  Like `Scope::Instance` / `Scope::ClassVar`,
  it needs **no prior declaration**: `check_varref` performs no
  scope-existence check (a constant resolves against the constant scope,
  not a `let` binding).  `Scope::name()` / `from_name()` gain the
  `"const"` tag.
- `Feature::Constants` (kebab `constants`) — declared by any module that
  references a `Scope::Const`.  The validator observes it from each
  Const-scoped `VarRef`; backends that don't list it in their accepted
  set reject such modules at the capability check, before emit.

### Changed

- `check_varref` gains a `Scope::Const` arm (observe-only, no
  resolution).  The text printer renders `(var-ref FOO const)` via the
  existing `scope.name()` path.  The four reference backends'
  `emit_var_ref` gain an unreachable `panic!` arm for `Scope::Const`
  (rejected pre-emit by the capability check).

New tests (3): `print_var_ref_const_scope`,
`const_ref_needs_no_declaration`,
`const_ref_without_manifest_feature_is_error`.  Test count: 99 → 102.

This is a **breaking enum change** for any exhaustive `match` on
`Scope` without a `_` rest arm.

## 0.7.0 — SIR17: class-variable scope (`Scope::ClassVar`)

Introduced by the Ruby frontend's Phase 15b (`@@x`).

### Added

- `Scope::ClassVar` — a class variable (Ruby `@@x`).  Like
  `Scope::Instance`, a class var needs **no prior declaration**:
  `check_varref` performs no scope-existence check for it (reading an
  unset `@@x` yields nil in Ruby).  `Scope::name()` / `from_name()` gain
  the `"class-var"` tag.
- `Feature::ClassVars` (kebab `class-vars`) — declared by any module
  that references a `Scope::ClassVar` var.  The validator observes it
  from each ClassVar-scoped `VarRef`; backends that don't list it in
  their accepted set reject such modules at the capability check,
  before emit.

### Changed

- `check_varref` gains a `Scope::ClassVar` arm (no resolution; observes
  `Feature::ClassVars`).  The text printer renders
  `(var-ref @@x class-var)` via the existing `scope.name()` path.  The
  four reference backends' `emit_var_ref` gain an unreachable `panic!`
  arm for `Scope::ClassVar` (rejected pre-emit by the capability check).

New tests (3): `print_var_ref_class_var_scope`,
`class_var_ref_needs_no_declaration`,
`class_var_ref_without_manifest_feature_is_error`.  Test count:
96 → 99.

This is a **breaking enum change** for any exhaustive `match` on
`Scope` without a `_` rest arm.

## 0.6.0 — SIR17: instance-variable scope (`Scope::Instance`)

Introduced by the Ruby frontend's Phase 15a (`@x`).

### Added

- `Scope::Instance` — an object instance variable (Ruby `@x`).  Unlike
  `Scope::Local`, an instance var needs **no prior declaration**:
  `check_varref` performs no scope-existence check for it (reading an
  unset `@x` yields nil in Ruby).  `Scope::name()` / `from_name()` gain
  the `"instance"` tag.
- `Feature::InstanceVars` (kebab `instance-vars`) — declared by any
  module that references a `Scope::Instance` var.  The validator
  observes it from each Instance-scoped `VarRef`; backends that don't
  list it in their accepted set reject such modules at the capability
  check, before emit.

### Changed

- `check_varref` gains a `Scope::Instance` arm (no resolution; observes
  `Feature::InstanceVars`).  The text printer renders
  `(var-ref @x instance)` via the existing `scope.name()` path.  The
  four reference backends' `emit_var_ref` gain an unreachable `panic!`
  arm for `Scope::Instance` (rejected pre-emit by the capability check).

New tests (3): `print_var_ref_instance_scope`,
`instance_var_ref_needs_no_declaration`,
`instance_var_ref_without_manifest_feature_is_error`.  Test count:
93 → 96.

This is a **breaking enum change** for any exhaustive `match` on
`Scope` without a `_` rest arm.

## 0.5.0 — SIR17: singleton-class declarations (`Stmt::SingletonClassDef`)

Introduced by the Ruby frontend's Phase 14e (`class << self … end`).

### Added

- `Stmt::SingletonClassDef { target: String, body: Vec<Stmt>, span: Span }`
  — a singleton-class (metaclass) declaration.  `target` is the
  receiver whose singleton class is opened (`"self"` for the dominant
  `class << self` idiom, or a bare object name).  Like
  `ClassDef`/`ModuleDef`, method `def`s in the body are hoisted to
  top-level `Function`s by the Ruby lowerer; `body` carries the
  non-`def` statements.  Reuses `Feature::Classes` (a singleton class
  is a class-opening construct, not a new feature) — no manifest
  change.

### Changed

- `Stmt::span()`, the walker, the validator (marks `Feature::Classes`,
  walks the body via `check_stmt_seq` in a scoped env mark/rewind with
  the `MAX_IR_DEPTH` guard — same shape as `ClassDef`), the text
  printer (`(singleton-class-def << Target …)`), and the
  intrinsic-walk backend helper gain a `SingletonClassDef` arm.  The
  four reference backends gain an unreachable `panic!` arm
  (`Feature::Classes` absent from their accepted sets → rejected at the
  capability check before emit).

New tests (3): `print_singleton_class_def`,
`singleton_class_def_body_with_let_binding_validates`,
`singleton_class_def_body_undefined_varref_is_error`.  Test count:
90 → 93.

This is a **breaking enum change** for any exhaustive `match` on `Stmt`
without a `_` / `..` rest arm.

## 0.4.0 — SIR17: module declarations (`Stmt::ModuleDef`)

Introduced by the Ruby frontend's Phase 14d (`module M … end`).

### Added

- `Stmt::ModuleDef { name: String, body: Vec<Stmt>, span: Span }` — a
  module (namespace / mixin) declaration.  Structurally a `ClassDef`
  without inheritance: a named declaration whose `body` is a list of
  statements.  Like `ClassDef`, method `def`s inside the body are
  hoisted to top-level `Function`s by the Ruby lowerer; the `body`
  carries the module's non-`def` statements.
- `Feature::Modules` (kebab name `modules`) — declared by any module
  that contains a `Stmt::ModuleDef`.  Distinct from `Classes`: a Ruby
  `module` is a namespace/mixin, not an instantiable class.  Backends
  that do not list it in their accepted-feature set reject such modules
  at the capability check, before emit.

### Changed

- `Stmt::span()`, the walker, the validator (marks `Feature::Modules`,
  walks the body via `check_stmt_seq` in a scoped env mark/rewind with
  the `MAX_IR_DEPTH` guard — same shape as the `ClassDef` arm), the
  text printer (`(module-def Name …)` s-expression), and the
  intrinsic-walk backend helper all gain a `ModuleDef` arm.  The four
  reference backends (TypeScript, Rust, Python, Go) gain an unreachable
  `panic!` arm; `Feature::Modules` is absent from their accepted sets,
  so module-using modules are rejected at the capability check before
  emit.

New tests (5): `print_empty_module_def`, `print_module_def_with_body_stmt`,
`module_def_body_with_let_binding_validates`,
`module_def_body_undefined_varref_is_error`,
`module_def_without_manifest_feature_is_error`.  Test count: 85 → 90.

This is a **breaking enum change** for any exhaustive `match` on `Stmt`
that does not use a `_` / `..` rest arm.

## 0.3.0 — SIR17: class inheritance (`ClassDef.superclass`)

Introduced by the Ruby frontend's Phase 14c (`class Foo < Bar`).

### Added

- `Stmt::ClassDef` gains a `superclass: Option<String>` field — the
  parent class name (`Some("Bar")` for `class Foo < Bar`, `None` for a
  base class `class Foo`).  It is an advisory name only: SIR v0 has no
  class symbol table, so the validator does not resolve it (mirroring
  how the class's own `name` is not bound as a local).

### Changed

- The text printer emits a `(< Super)` clause right after the class
  name when `superclass` is set: `(class-def Foo (< Bar))`.  Base
  classes are unchanged (`(class-def Foo)`).
- The walker, validator, intrinsic-walk backend helper, and the four
  reference backends' `ClassDef` arms are unaffected by the new field
  (it carries no sub-expressions to traverse and no capability impact);
  class-using modules are still rejected at the capability check before
  emit.

New test: `print_class_def_with_superclass`.  This is a **breaking
struct change** for any code constructing `Stmt::ClassDef` literally —
all in-tree constructors updated to pass `superclass`.

## 0.2.1 — SIR17 validator: walk populated `ClassDef` bodies

No node-shape change.  The Ruby frontend's Phase 14b begins emitting
`Stmt::ClassDef` nodes with a *populated* `body` (Phase 14a always
emitted an empty body), so the validator now actually walks it.

### Changed

- Factored the statement-sequence walk out of `check_block` into a
  new private `check_stmt_seq(&[Stmt], env, depth)` helper.
  `check_block` now calls it for `block.stmts` (then checks the
  trailing `block.value`), preserving the exact prior behaviour —
  parallel-`let` grouping, sequential `let*`, mutable `Assign`,
  loop/scope handling.
- `Stmt::ClassDef`'s validator arm now calls `check_stmt_seq` on the
  body inside a fresh `env.mark()`/`env.rewind()` scope (Phase 14a
  left this loop a documented no-op).  Class-body locals therefore
  do **not** leak into the surrounding statement stream, and a
  bad reference inside a class body is now reported instead of
  silently accepted.  An explicit `MAX_IR_DEPTH` guard bounds
  recursion for pathologically nested `class … class …` bodies.

New tests (3): `class_def_body_with_let_binding_validates`,
`class_def_body_undefined_varref_is_error` (proves the body is
walked, not no-op'd), `class_def_body_local_does_not_leak_to_sibling`.
Test count: 81 → 84 (+3).

## 0.2.0 — SIR17: class declarations

Adds the first object-oriented IR node, introduced by the Ruby
frontend's Phase 14a (empty `class Foo; end`).

### Added

- `Stmt::ClassDef { name: String, body: Vec<Stmt>, span: Span }` — a
  class declaration whose body is a list of statements.  The Ruby
  frontend's Phase 14a lands the *empty-body* case (`body: vec![]`);
  the variant is shaped to carry a populated body in later phases.
  `body` is a `Vec<Stmt>` rather than a `Block` because a class body
  is a declaration, not a value-producing expression.
- `Feature::Classes` (kebab name `classes`) — declared by any module
  that contains a `Stmt::ClassDef`.  Backends that do not list it in
  their accepted-feature set reject such modules at the capability
  check, before emit.

### Changed

- `Stmt::span()`, the validator, the text printer (`(class-def
  Name ...)` s-expression), the walker, and the intrinsic-walk
  backend helper all gained a `ClassDef` arm.  The four reference
  backends (TypeScript, Rust, Python, Go) reject class-using modules
  via their unchanged capability declarations, so their emit paths
  treat the new arm as unreachable.

## 0.1.0 — initial release (SIR10 v0)

First cut of the narrow-waist Semantic IR.  Implements the v0
surface defined in
[SIR10-narrow-waist-semantic-ir.md](../../../specs/SIR10-narrow-waist-semantic-ir.md).

### Added

- `Module`, `Function`, `Block`, `Stmt`, `Expr`, `Scope`, and all
  supporting node types per SIR10 §"Module structure" through
  §"Expressions".
- `SirType` carrier — `Any`, `Int`, `Bool`, `Nil`, `Symbol`, `Str`,
  `Pair`, `Closure`, parametric `Fn`.
- `EffectSet` bitset with effect tags `MayThrow`, `MayPrint`,
  `MayAllocate`, `MayBlock`, `Divergent`; pure is the empty set.
- `FeatureManifest` with the v0 feature list (`Closures`, `Pairs`,
  `Symbols`, `Strings`, `DynamicTyping`,
  `OptionalTypeAnnotations`, `MutualRecursion`, `TailCalls`,
  `Globals`, `Intrinsics`).
- `Metadata` carrier with source-language/version and SIR-version
  fields; advisory only — IR correctness must not depend on it.
- `Span` source-position carrier (1-indexed line and column).
- `validate(module)` — structural and semantic checks:
  - Manifest covers every feature actually used
  - No `VarRef` references an undefined name in its scope
  - `Intrinsic.targets` is non-empty
  - Function / global name uniqueness
  - Parallel `let` does not leak bindings into sibling RHS
  - Sequential `let*` allows prior bindings on subsequent RHS
- `Visitor` trait + free `walk_*_default` functions for read-only
  traversal.
- Canonical S-expression text printer (`print_module`, `print_expr`,
  `print_block`, `print_function`).  Output is deterministic and
  byte-stable.
- `Backend` trait and `BackendRegistry` with built-in capability
  enforcement (manifest features + intrinsic whitelist + target-tag
  matching).
- Test coverage for every public surface, including parallel/let*
  semantics distinguishing tests.

### Deferred to future versions

- Text-format parser (round-trip currently relies on printer
  determinism).
- Ownership / borrow markers (Move / Copy / Borrow).
- Async / await / coroutines.
- Exception handling (Raise / Try / Catch).
- Pattern matching (Match) and record / union / type-alias forms.
- Effect inference (manual annotation only in v0).
- Sequence / Map / Set / Option / Result primitives.
