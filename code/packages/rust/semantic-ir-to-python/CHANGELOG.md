# Changelog

## 0.1.10 — regex builtin → sir-runtime-regex (gated import)

The Ruby `/pat/flags` literal lowers to `BuiltinCall("regex", [pattern,
flags])`, which previously hit the unknown-builtin dispatch and raised at
runtime. It now emits a call into the new `coding-adventures-sir-runtime-regex`
package (native `re` compile with Ruby→Python flag translation), per
`code/specs/sir-runtime.md`.

- `BuiltinCall("regex", …)` → `_sir_regex_compile(pattern, flags)`.
- New gated `RUNTIME_REGEX` import header, appended **only** when a module calls
  the `regex` builtin. Because regex carries no SIR `Feature`, the gate
  (`uses_regex`) is a content walk — an exhaustive `Stmt`/`Expr` recursion that
  finds a `BuiltinCall` by name (the compiler forces every node to be handled,
  so a new node can't silently hide a use).
- New direct-SIR tests assert the gated import + `_sir_regex_compile("ab+c",
  "i")` and that a non-regex module omits the import. Exec-proofed on CPython
  against the real package (case-insensitive search, unanchored `is_match`).

## 0.1.9 — pairs extracted to sir-runtime-pairs (gated import)

Cons pairs (`cons`/`car`/`cdr`/`pair?`) now ship in the dedicated
`coding-adventures-sir-runtime-pairs` package (core re-exports them for
back-compat), per `code/specs/sir-runtime.md` (per-concern runtime modules).

- New gated `RUNTIME_PAIRS` import header (`from
  coding_adventures_sir_runtime_pairs import cons as _sir_cons, …`), appended
  **only** when a module uses the `Pairs` feature (`uses_pairs`). The
  `cons`/`car`/`cdr`/`is_pair` aliases are removed from the always-on core
  import header, so pure non-pair modules no longer depend on the pairs package.
- The emitter's `_sir_cons`/`_sir_car`/`_sir_cdr`/`_sir_is_pair` call names are
  unchanged — only the *source* of the aliases moved — so `emit.rs` is
  behaviour-preserving aside from the new gated import.
- New direct-SIR tests assert the gated import + `_sir_car(_sir_cons(1, 2))` and
  that a non-pair module omits the import. Cross-package display wiring (core
  injects `to_display` into the pairs package) is covered by the core package's
  own pytest list-display tests and an exec-proof on CPython.

## 0.1.8 — SIR17 exceptions (native try/except + sir-runtime-exceptions)

Accepts and emits the SIR17 `Exceptions` feature, per
`code/specs/sir-runtime.md`. `begin/rescue/ensure` translates to a **native**
`try: … except Exception as __exc: … finally: …`; the two pieces with no
faithful native equivalent come from the new
`coding-adventures-sir-runtime-exceptions` package, imported (aliased
`_sir_exc_*`) **only** when a module throws or rescues.

- `Stmt::TryCatch{body, rescues, ensure_body}` → `try:` block. Because Python's
  `except` matches by Python class while Ruby has an ordered list of typed
  `rescue` clauses, the handler catches broadly (`except Exception as __exc`)
  and the body is an `if`/`elif` chain calling
  `_sir_exc_rescue_matches(__exc, [class names])` per clause in source order; a
  `rescue Foo => e` binds `e = __exc`; if no clause matches the original
  exception is re-`raise`d (Ruby's "propagate when unrescued"). `ensure_body` →
  a `finally:` block (omitted when absent). Empty bodies emit `pass`.
- `BuiltinCall("raise", …)` → `_sir_exc_raise_error(…)`: a `Const` class operand
  (`raise Foo` / `raise Foo, "m"`) is passed as its *name string* with the
  optional message; a non-`Const` first arg (`raise "m"`) becomes an implicit
  `RuntimeError` carrying that message; bare `raise` → a generic re-raise.
- `block_has_loop` now also forces a `TryCatch`-bearing block to lift to a
  nested `def` in expression position (a compound statement is not a walrus
  expression); `collect_nonlocals` descends into try/rescue/ensure bodies.
- `ACCEPTED_FEATURES += Exceptions`.

New Ruby→Python and direct-SIR tests (begin/rescue/ensure shape, message-only
`raise`, bare-rescue catch-all + re-raise, non-throwing module omits the
import). Emitted output verified to execute on CPython against the real
`coding-adventures-sir-runtime-exceptions` (ancestor-matched rescue with bound
message, `ensure` runs, unmatched exception propagates). Mirrors the TypeScript
backend's Q7a.

## 0.1.7 — SIR17 OOP & scopes (native + sir-runtime-oop)

Accepts and emits the SIR17 object-orientation statements and scopes, per
`code/specs/sir-runtime.md`. Because the Ruby→SIR frontend **hoists methods to
detached, receiver-less top-level functions**, there is no native `self` to hang
members on; the object model is supplied by the new
`coding-adventures-sir-runtime-oop` package, imported (aliased `_sir_oop_*`)
**only** when a module uses an OOP feature.

- `Stmt::ClassDef{name, superclass, body}` → `_sir_oop_define_class(name, super)`
  (registers ancestry) followed by the body statements (constant / class-var
  assigns). `Stmt::ModuleDef` → `_sir_oop_define_class(name, None)`.
  `Stmt::SingletonClassDef` → its (non-`def`) body statements.
- `Scope::Instance` (`@x`) → `_sir_oop_ivar_get`/`ivar_set` against the
  current-self stack; `Scope::ClassVar` (`@@x`) → `_sir_oop_cvar_get`/`cvar_set`;
  `Scope::Const` → an ordinary module-level `NAME = value` (reads emit the bare
  identifier). All four are also handled in the walrus (block-as-expr) path.
- `BuiltinCall("__method__", [recv, "meth", args…])` → `_sir_oop_call_method(
  recv, "meth", …)`; for the class predicates a `Const`-scoped class operand is
  passed as its **name string** so it works without a binding for the built-in
  class name.
- `ACCEPTED_FEATURES += Classes, Modules, InstanceVars, ClassVars, Constants`.

**v0 limitation (documented):** since the frontend does not thread receivers,
the current-self is a process-global stack and class variables share one
namespace — single-instance / single-class programs are faithful and never
raise; full multi-object semantics await frontend receiver threading. New
Ruby→Python and direct-SIR tests; a non-OOP module is asserted to omit the OOP
import; emitted output verified to execute on CPython against the real
`coding-adventures-sir-runtime-oop` (`is_a?` ancestry/exact/primitive, ivar
round-trip, cvar, `class`, const). Mirrors the TS backend's Q6a.

## 0.1.6 — SIR16 mutation & loops (native)

Accepts and emits the SIR16 mutation and loop statements as **native** Python
(per `code/specs/sir-runtime.md`):

- `Stmt::Assign` → `name = value` (Local/Param/Capture); a `Global` target
  writes the module-level `_globals` dict (`_globals["n"] = value`), matching
  how `_init`/`global_set` and `VarRef::Global` reads are rendered.
- `Stmt::SeqSet` → `s[i] = v`; `Stmt::MapSet` → `m[k] = v`.
- `Stmt::While` → `while _sir_truthy(cond):` — the test routes through SIR
  truthiness (only `False`/`None` falsy), never Python's.
- `Stmt::ForRange` → `for v in range(start, stop, step):` — Python's `range` is
  already half-open and direction-aware, so it matches SIR `ForRange` exactly.
- `Stmt::ForEach` → `for v in iter:`. Empty loop bodies emit `pass`.

**Expression-position loops.** Python has no multi-statement expression, so the
existing walrus-tuple strategy for statement-bearing blocks-in-expression-
position cannot express a loop. Such a block is now lifted to a nested
`def __block_N(): …` (queued in a hoist buffer, flushed before the enclosing
statement); the call site emits `__block_N()`. The lifted def declares
`nonlocal` for every `Assign`-target local that is bound in an enclosing scope
(computed by walking the block and its inline loop bodies, minus names
introduced locally), so mutations reach the outer binding. Blocks *without* a
loop keep the walrus form, now extended to handle `Assign`/`SeqSet`/`MapSet`
(`(x := v)`, `s.__setitem__(i, v)`, `m.__setitem__(k, v)`).

`Assign` to an instance var / class var / constant still rejects at the
capability check (those features are not yet accepted). `ACCEPTED_FEATURES +=
MutableBindings, Loops`. New direct-SIR and Ruby→Python tests; emitted output
verified to execute on CPython against the real
`coding-adventures-sir-runtime-core` (while counter, `range`+index-set,
`for`-each, map-set, and a lifted expression-position loop with `nonlocal`).

## 0.1.5 — SIR16 expression features (native)

Accepts and emits the SIR16 expression features, all translated to **native**
Python (per `code/specs/sir-runtime.md`):

- `Feature::Floats` → float literal; `Feature::Sequences` → list literal /
  `s[i]` / `len(s)`; `Feature::Maps` → dict literal / `m[k]`.
- `Feature::ShortCircuit` (`LogicalAnd`/`LogicalOr`, emitted by case/in pattern
  desugaring) → a **truthy-guarded lambda** `(lambda __l: (rhs) if
  _sir_truthy(__l) else __l)(lhs)`: keeps the rhs lazy AND uses SIR truthiness
  (only `False`/`nil` falsy), never a bare Python `and`/`or`.
- `Feature::StringInterpolation` (`StrConcat`) → parts joined through
  `_sir_to_display` (a string part renders to itself).

`to_display` added to the runtime import header as `_sir_to_display`.
New Ruby→Python E2E tests for array literal, hash literal, pattern short-circuit,
and interpolation. (TS counterpart lands separately.)

## 0.1.4 — import runtime from `coding-adventures-sir-runtime-core`

The Python runtime is no longer inlined into every artifact.  Emitted modules
now `import` it from the published `coding-adventures-sir-runtime-core` package
(per `code/specs/sir-runtime.md`), so nothing language-specific is pasted into
the generated file.

- `runtime.rs` `RUNTIME` is now an import header (`from
  coding_adventures_sir_runtime_core import (… as _sir_*)`) instead of a ~170-line
  class/function prelude.  The aliases keep the emitter's historical `_sir_*`
  call names, so `emit.rs` and the emitted user-code shapes are unchanged
  (behaviour-preserving).
- Tests updated to assert the import header rather than the inlined `class Symbol`.

## 0.1.3 — Ruby → Python end-to-end tests (tests only)

Adds end-to-end tests that drive the **Ruby** frontend
(`ruby-to-semantic-ir`) through this Python backend, proving the
narrow-waist Semantic IR decouples frontends from backends: Ruby source
in, runnable Python out, with zero Ruby-specific code in this crate.

- New dev-dependency `ruby-to-semantic-ir` (alongside the existing
  `twig-to-semantic-ir`).
- New tests: `end_to_end_ruby_to_python_puts`,
  `end_to_end_ruby_to_python_def_and_call`,
  `end_to_end_ruby_to_python_locals`,
  `end_to_end_ruby_to_python_is_deterministic`.
- Snippets are restricted to the backend's `ACCEPTED_FEATURES`
  (puts/arithmetic/defs/locals); Ruby constructs lowering to
  `Sequences`/`Maps`/`ShortCircuit` are intentionally excluded (rejected
  at the capability check by design). No production-code or output
  changes.

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

## 0.1.0 — initial release (SIR14 v0)

Third backend for the narrow-waist Semantic IR.  Emits
self-contained Python 3 source from a `semantic_ir::Module`.

### Added

- `PythonBackend` implementing `semantic_ir::Backend` with
  `target_tag = "python"`, accepting the v0 feature set minus
  `TailCalls` and `Intrinsics`.
- `compile(module)` convenience function.
- Per-node lowering matching SIR14 §"Per-node lowering rules".
- Inlined Python runtime (~140 lines) with `Symbol`, `Pair`,
  `Closure` classes, all 15 Twig builtins, symbol interning,
  module globals, and a builtin dispatch table.
- Block-as-expression via Python 3.8+ assignment expressions
  (walrus) — `((x := 1), (y := x + 2), result)[-1]`.
- Identifier sanitisation:
  - Valid Python identifiers pass through.
  - Python keywords get an underscore suffix (`def_`, `class_`).
  - Invalid characters encoded as `_<hex>` forms.
  - Empty input → `_sir_empty`.
  - SIR's `main` is renamed to `_sir_user_main`.
- `sanitize_comment` strips line terminators from external strings
  written into `#` comments — mirrors SIR12 / SIR13.
- 18 unit + end-to-end tests covering identity, arithmetic, and
  closure-adder pipelines from Twig source.

### Deferred

- Type hint enrichment (`def foo(x: int) -> int:`).
- Source maps.
- `async def` / `await` support.
- Raw-Python intrinsic injection.
