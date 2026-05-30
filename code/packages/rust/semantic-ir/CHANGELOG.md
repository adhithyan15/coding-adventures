# Changelog

All notable changes to the `semantic-ir` crate are documented here.

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
