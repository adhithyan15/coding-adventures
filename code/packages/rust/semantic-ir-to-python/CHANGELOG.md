# Changelog

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
