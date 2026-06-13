# Changelog

## 0.1.6 — SIR17 OOP & scopes (native + sir-runtime-oop)

Accepts and emits the SIR17 object-orientation statements and scopes, per
`code/specs/sir-runtime.md`. Because the Ruby→SIR frontend **hoists methods to
detached, receiver-less top-level functions**, there is no native `this`/`self`
to hang members on; the object model is supplied by the new
`@coding-adventures/sir-runtime-oop` package, imported (as `__SirOop`) **only**
when a module uses an OOP feature.

- `Stmt::ClassDef{name, superclass, body}` → `__SirOop.defineClass(name, super)`
  (registers ancestry) followed by the body statements (constant / class-var
  assigns) in source order. `Stmt::ModuleDef` → `defineClass(name, null)`.
  `Stmt::SingletonClassDef` → its (non-`def`) body statements.
- `Scope::Instance` (`@x`) → `__SirOop.ivarGet/ivarSet` against the current-self
  stack; `Scope::ClassVar` (`@@x`) → `__SirOop.cvarGet/cvarSet`; `Scope::Const`
  → an ordinary module-level `const NAME` (reads emit the bare identifier).
- `BuiltinCall("__method__", [recv, "meth", args…])` (reflective dispatch, e.g.
  `is_a?`) → `__SirOop.callMethod(recv, "meth", …)`; for the class predicates
  (`is_a?`/`kind_of?`/`instance_of?`) a `Const`-scoped class operand is passed
  as its **name string** so the predicate works without a binding for the
  built-in class name.
- `ACCEPTED_FEATURES += Classes, Modules, InstanceVars, ClassVars, Constants`.

**v0 limitation (documented):** since the frontend does not thread receivers,
the current-self is a process-global stack and class variables share one
namespace — single-instance / single-class programs are faithful and never
crash; full multi-object semantics await frontend receiver threading. New
Ruby→TS and direct-SIR tests; a non-OOP module is asserted to omit the OOP
import; emitted output verified to compile under `tsc --strict` and to execute
on Node against the real `@coding-adventures/sir-runtime-oop`
(`is_a?` ancestry/exact/primitive, ivar round-trip, cvar, `class`).

## 0.1.5 — SIR16 mutation & loops (native)

Accepts and emits the SIR16 mutation and loop statements as **native**
TypeScript (per `code/specs/sir-runtime.md`):

- `Stmt::Assign` → bare reassignment `name = value;` (Local / Param / Capture
  / Global all resolve to a plain identifier). A `LetBinding` whose name is
  later reassigned now emits `let` instead of `const` (a per-function pre-pass
  collects `Assign` targets), so the reassignment type-checks; immutable
  bindings stay `const`.
- `Stmt::SeqSet` → `((s) as __Sir.Val[])[(i) as number] = v;`;
  `Stmt::MapSet` → `((m) as Map<__Sir.Val, __Sir.Val>).set(k, v);`.
- `Stmt::While` → `while (__Sir.truthy(cond)) { … }` — the test routes through
  SIR truthiness (only `false`/`nil` falsy), never JS truthiness.
- `Stmt::ForRange` → a block-scoped, **direction-aware** loop: `start` binds to
  the loop variable, `stop`/`step` are evaluated **once** into `number`
  temporaries (matching Python's `range`), and the condition flips to `>` when
  `step` is negative so a descending range still terminates.
- `Stmt::ForEach` → `for (const v of ((iter) as __Sir.Val[])) { … }`.

Loop bodies render in statement context (trailing block value dropped when it
is `nil`, else emitted as an expression statement). Loops in *expression*
position nest naturally inside the existing block IIFE. `Assign` to an
instance var / class var / constant still rejects at the capability check
(those features are not yet accepted). `ACCEPTED_FEATURES += MutableBindings,
Loops`. New direct-SIR and Ruby→TS tests; emitted output verified to compile
under `tsc --strict` and execute on Node against the real
`@coding-adventures/sir-runtime-core`.

## 0.1.4 — SIR16 expression features (native)

Accepts and emits the SIR16 expression features as **native** TypeScript (per
`code/specs/sir-runtime.md`):

- `Feature::Floats` → number literal; `Feature::Sequences` → array literal /
  `((s) as __Sir.Val[])[i]` / `.length`; `Feature::Maps` → `new Map<…>([[k,v]…])`
  / `.get(k) ?? null`.
- `Feature::ShortCircuit` (`LogicalAnd`/`LogicalOr`, from case/in pattern
  desugaring) → a **truthy-guarded arrow** `((__l: __Sir.Val) => __Sir.truthy(__l)
  ? (rhs) : __l)(lhs)`: rhs stays lazy AND the test uses SIR truthiness (only
  `false`/`nil` falsy), never a bare `&&`/`||`.
- `Feature::StringInterpolation` (`StrConcat`) → parts joined through
  `__Sir.toDisplay`.

Requires `@coding-adventures/sir-runtime-core` ≥ 0.1.1 (its `Val` union now
includes `Val[]` / `Map<Val,Val>` so emitted native arrays/maps typecheck).
New Ruby→TS E2E tests for array literal, hash literal, pattern short-circuit,
and interpolation.

## 0.1.3 — import runtime from `@coding-adventures/sir-runtime-core`

The TypeScript runtime is no longer inlined into every artifact.  Emitted modules
now `import * as __Sir from "@coding-adventures/sir-runtime-core";` (per
`code/specs/sir-runtime.md`), so nothing language-specific is pasted into the file.

- `runtime.rs` `RUNTIME` is now a one-line import header instead of the inlined
  `namespace __Sir { … }` block.
- `emit.rs` updated to the core package's API names: `__Sir.add/sub/mul/div`
  (was `plus/minus/times/divide`), `__Sir.apply` (was `applyClosure`),
  `__Sir.builtinClosure(name)` / `__Sir.callBuiltin(name, [...])` (was
  `__Sir.builtins[name]`), and `null` (was `__Sir.NIL`).  `__Sir.Val` / `__Sir.Sym`
  / `__Sir.Pair` / `__Sir.Closure` are unchanged (re-exported by the package).
- Emitted user-code shapes are otherwise unchanged; tests updated accordingly.

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

## 0.1.0 — initial release (SIR12 v0)

First backend for the narrow-waist Semantic IR.  Emits self-contained
TypeScript source from a `semantic_ir::Module`.

### Added

- `TypeScriptBackend` implementing `semantic_ir::Backend` with:
  - `target_tag() = "typescript"`
  - `accepts_features()` covering Closures, Pairs, Symbols, Strings,
    DynamicTyping, OptionalTypeAnnotations, MutualRecursion, Globals.
  - `accepts_intrinsics()` empty in v0 — all intrinsics rejected.
- `compile(module)` convenience function returning an
  `Artifact { filename, source, metadata }`.
- Per-node lowering rules per SIR12:
  - Literals → JS literals (`null`, `true`, `false`, numbers,
    quoted strings).
  - Symbols → `__Sir.intern("...")`.
  - VarRef Local / Param / Capture / Global → bare identifier.
  - VarRef Builtin → `__Sir.builtins["<name>"]`.
  - If → ternary using `__Sir.truthy(cond)`.
  - Block with statements → IIFE; block without statements → bare
    value expression.
  - LetBinding / LetStarBinding → `const`.  Parallel-let semantics
    were preserved by the frontend; sequential `let*` is naturally
    honored by top-down `const` emission.
  - DirectCall → `<fn>(...)`.
  - IndirectCall → `__Sir.applyClosure(target, [...args])`.
  - BuiltinCall → one of the `__Sir.<op>` helpers, with an
    unrecognised-name fallback through the dispatch table.
  - MakeClosure → `new __Sir.Closure((..._a) => <fn>(<caps>, ..._a))`.
- Inlined `__Sir` namespace runtime (~110 lines) with:
  - `Val` discriminated union type
  - `Sym` / `Pair` / `Closure` classes
  - `intern`, `applyClosure`, `globalSet`, `globalGet`
  - All v0 builtins (`plus`, `minus`, `times`, `divide`, `eq`,
    `lt`, `gt`, `cons`, `car`, `cdr`, `isNull`, `isPair`,
    `isNumber`, `isSymbol`, `print`)
  - `format` and `truthy` helpers
  - `builtins` dispatch table for VarRef Builtin
- Identifier sanitisation: SIR names containing `?`, `!`, `-`, `+`,
  etc. (e.g. `null?`, `pair?`) are rewritten to `_$<hex-escaped>`
  forms; valid TS identifiers pass through unchanged.  TS reserved
  words are also rewritten.
- TypeScript-safe string literal escaping including `\uXXXX` for
  control characters.
- Pre-lowering validation via `semantic_ir::validate`; module-level
  capability check via `Backend::check_module` default impl.
- Special-case lowering of the `BuiltinCall("global_set", SymLit,
  value)` pattern emitted by `_init` — rendered as a direct
  `<global> = <value>;` assignment for readable output.

### Deferred

- Source-map generation (only function-level span comments today).
- Optimisation passes (constant folding, block flattening).
- Async / `await`, top-level await.
- Intrinsic support — the v0 backend rejects all intrinsics; a
  future revision may add `typescript`-tagged ones for raw-TS
  embedding.
