# Changelog

## 0.1.22 — case-equality `case_eq` emission (M5)

A `case_eq` builtin (emitted by a `when` clause for range/regex/literal
patterns) now routes to `__SirOop.caseEq(pattern, value)` (the `import * as
__SirOop` already exposes it). `uses_oop` fires on `case_eq` so a class-less
`case/when` still imports `@coding-adventures/sir-runtime-oop`. Emitted-shape
test covers the dispatch.

## 0.1.21 — coverage: outer-local block captures (M4)

No emitter change — the backend already closes over `MakeClosure` captures as
leading parameters. Adds an emitted-shape test that a block reading an
enclosing local emits `function __block_0(base: __Sir.Val, n: __Sir.Val)` with
`base` forwarded into the hoisted block. The frontend now produces such
captures (see `ruby-to-semantic-ir` 0.97.0, M4).

## 0.1.20 — variadic parameter emission (`...rest`) (M3)

A `Param` carrying the new SIR `ParamKind` now emits TypeScript's variadic
forms: `Rest` → `...name: __Sir.Val[]` (native JS rest, already a real Array =
SIR sequence); `Required` is unchanged. So `def f(a, *rest); end` emits
`function f(a: __Sir.Val, ...rest: __Sir.Val[])`.

- **KwRest v0 object fallback.** JavaScript has no keyword-argument call form,
  so a `KwRest` (`**opts`) def parameter has no faithful native declaration. v0
  emits it as a trailing ordinary object parameter (`opts: __Sir.Val`) — the
  call side (Q10f) already collapses `**h` into a single merged trailing
  object, so this binds it. Documented limitation; mirrors the TS double-splat
  call-position treatment.
- **OOP import gating widened.** `uses_oop` now also fires on the `__method__`
  and `__scope__` dispatch builtins, not only the OOP *features* — so a
  class-less dispatch program (`"hi".upcase`, or a rest param used as an Array)
  imports `@coding-adventures/sir-runtime-oop`.

## 0.1.19 — `&:sym` symbol-to-proc on dispatched calls (M2)

A `&:sym` block argument on a method-dispatch call (`recv.map(&:to_s)`) now
emits working code. The Ruby→SIR frontend lowers `&:sym` to
`block_pass(SymLit("sym"))`; the Q9f normalization unwraps `block_pass` only at
*user-method* `DirectCall` sites, so a block-pass to a `__method__` dispatch
call reached the backend intact and previously rendered as a broken
`callBuiltin("block_pass", …)`. `emit_arg` and the `__method__` argument loop
now recognize the surviving envelope:

- `block_pass(SymLit("m"))` → `__SirOop.symToProc(__Sir.intern("m"))` — the
  `Symbol#to_proc` runtime helper returns a `Closure` that calls `recv.m(...rest)`;
- `block_pass(<other>)` → the inner operand, unwrapped (the proc *is* the block).

The `__SirOop` namespace import already covers the new helper (no header
change). New tests `sym_block_pass_on_dispatch_emits_sym_to_proc`,
`proc_block_pass_on_dispatch_unwraps_to_value`,
`sym_block_pass_as_plain_arg_emits_sym_to_proc`. Requires
`@coding-adventures/sir-runtime-oop` ≥ 0.1.6. Operator symbols (`&:+`) remain a
documented v0 boundary (native arithmetic, not dispatched).

## 0.1.18 — `defined?(recv.meth)` → "method" (Q10h)

`defined?` over a method-call operand `recv.meth` — the `__method__` dispatch
envelope — now emits the constant `"method"` (Ruby's category when the method
resolves) instead of the generic `"expression"`. Purely emit-time shape
inspection; the non-evaluation contract is unchanged. The runtime
respond_to?-presence check, and arbitrary built-in/collection method dispatch
(`arr.each`, `&:sym`'s target, …) which `callMethod` returns `nil` for, are the
documented terminal method-dispatch boundary (see `code/specs/sir-runtime.md`).
New test `defined_method_call_operand_emits_method_ts`.

## 0.1.17 — call-position `**h` via runtime merge helper (Q10f)

JavaScript has no keyword-argument call form, so call-position `**h` could not
be emitted faithfully and previously fell through to the eager dispatch
(`__Sir.callBuiltin("double_splat", …)`), which raised an unknown-builtin error.
This release lowers it: a new call-argument layer (`emit_call_args`, used by
`DirectCall`/`IndirectCall`) collapses each contiguous run of `**` markers into
a **single** trailing argument built by the runtime helper
`__Sir.doubleSplatMerge(h1, h2)` — the conventional JS "options object", except
the bag is a SIR `Map<Val, Val>` so any `Val` key round-trips.  Examples:

| Ruby | TypeScript |
|---|---|
| `f(**h)` | `f(__Sir.doubleSplatMerge(h))` |
| `f(a, **h1, **h2)` | `f(a, __Sir.doubleSplatMerge(h1, h2))` |
| `f(*b, **h)` | `f(...b, __Sir.doubleSplatMerge(h))` |

Runs collapse *in place*, so a trailing block argument (block-param ABI) stays
after the merged map.  `splat` (`*a`) still emits native `...a` via `emit_arg`.

New tests `double_splat_call_arg_merges_via_runtime_helper_ts` and
`double_splat_contiguous_run_collapses_to_single_merge_ts`; the prior
`…_is_deferred_to_dispatch_ts` test is replaced.  v0 cut-line: mixing inline
`key: value` pairs with `**h` at one call site is not modelled — only explicit
`**map` operands are merged (see `code/specs/sir-runtime.md`).  Requires
`sir-runtime-core` ≥ 0.1.5 (`doubleSplatMerge`).

## 0.1.16 — emitted-shape proof for the non-empty block capture (RB3)

Mirror of the Python backend's 0.1.17.  RB1/RB2 (Ruby frontend) introduced the
first SIR shape with a **non-empty** `MakeClosure` capture: a hoisted block that
closes over the enclosing method's `__sir_block__`.  TypeScript captures by
native lexical closure rather than a positional array, so the binding emits as
`twice(new __Sir.Closure((..._a) => __block_0(__sir_block__, ..._a)))` with the
hoisted `function __block_0(__sir_block__: __Sir.Val, x: __Sir.Val)` taking the
captured block as its first parameter.

New test `end_to_end_ruby_block_capture_emits_native_capture_ts` asserts that
binding.  Unlike the Python sibling it does **not** execute: Node cannot run the
emitted *TypeScript* directly (type annotations; the runtime package ships as
`.ts` with no `dist/`), so this proves the capture by shape.  No emitter change
— verification only.

## 0.1.15 — `defined?` lowers to a non-evaluating description (Q9d)

Fourth item of the Q9 structural-builtin tranche.  Ruby `defined?(x)` reaches
the backend as `BuiltinCall("defined?", [operand])`; its operand must **never**
be evaluated.  `emit_builtin_call` now inspects the operand's SIR shape at emit
time and emits a constant description string, never rendering the operand:

- local / param / capture `VarRef` → `"local-variable"`
- `Const` → `"constant"`; `Instance` → `"instance-variable"`;
  `ClassVar` → `"class variable"`; `Global` → `"global-variable"`;
  builtin-name → `"method"`
- any other expression → `"expression"`

Same shape→description table and v0 simplifications as the Python backend (see
its 0.1.16 and `code/specs/sir-runtime.md`); the non-evaluation contract holds
for every shape.  Tests: `defined_local_var_emits_static_description_ts` and
`defined_does_not_evaluate_operand_ts`.

## 0.1.14 — `splat` lowers to native spread; `double_splat` deferred (Q9c)

Third item of the Q9 structural-builtin tranche.  Ruby `*x` / `**x` reach the
backend as `BuiltinCall("splat", [x])` / `BuiltinCall("double_splat", [x])`.
`emit_args` now expands them via a new `emit_arg` helper:

- `splat` → `...x`  (e.g. `f(*a)` → `f(...a)`, `[1, *mid, 3]` → `[1, ...mid, 3]`)
  — fully native JS spread.
- `double_splat` in call position → **deferred (v0 cut-line)**.  JavaScript has
  no keyword-argument call form and an SIR map is a `Map`, which does not spread
  into an object literal or a call, so there is no faithful native form.  `**h`
  falls through to the eager dispatch (`__Sir.callBuiltin("double_splat", …)`),
  which raises a clear unknown-builtin error rather than emitting silently wrong
  code.  (Python, which has `**`, lowers it natively — see its 0.1.15.)

Tests: `splat_in_seq_literal_emits_native_spread_ts` (`[1, ...mid, 3]`),
`splat_call_arg_emits_native_spread_ts` (`(...a)`), and
`double_splat_call_arg_is_deferred_to_dispatch_ts` (asserts the documented
dispatch fallthrough, and that it is *not* mis-emitted as a JS spread).

## 0.1.13 — `range` builtin lowers to the range runtime (Q9b)

Second item of the Q9 structural-builtin tranche.  Ruby `a..b` / `a...b` (and
the begin/endless `a..` / `..b` forms) reach this backend as
`BuiltinCall("range", [start, stop, exclusive])`.  JavaScript has no range type
at all, so `emit_builtin_call` now lowers it to `__SirRange.range(...)` from the
new per-concern package `@coding-adventures/sir-runtime-range` (the SIR
first-class `Range` value).  A `uses_range` content-walk gates the
`RUNTIME_RANGE` import, so pure modules never gain the dependency.  Tests:
`range_builtin_lowers_to_runtime_and_imports_ts` (emits
`__SirRange.range(1, 5, false)` + the import, never the dispatch fallthrough) and
`no_range_import_when_unused_ts`.

## 0.1.12 — `lambda` builtin lowers to its inner closure (Q9a)

First item of the structural-builtin follow-up tranche (Q9) the Q8d audit
flagged.  Ruby `lambda { … }` / `->(x){…}` reach this backend as
`BuiltinCall("lambda", [MakeClosure])`.  The lambda *is* its closure value, so
`emit_builtin_call` now emits the inner `MakeClosure` directly (rendering
`new __Sir.Closure(...)`) instead of routing through the eager `callBuiltin`
dispatch — there is no separate `lambda` runtime helper, the closure already is
the result.  Reuses the existing `MakeClosure` emission and `Closure` runtime
type; no runtime change.  Direct-SIR test
`lambda_builtin_lowers_to_inner_closure_ts`.

## 0.1.11 — boolean/unary operator builtins (audit close-out)

Builtin-coverage audit of what the Ruby frontend emits as a `BuiltinCall`
reaching this backend.  Ruby `&&`/`and`, `||`/`or`, `!`/`not`, and unary minus
lower to `BuiltinCall("and"/"or"/"not"/"neg", …)` — previously they fell
through to the eager `callBuiltin` dispatch and threw at runtime.  Now native:

- `and`/`or` → the same truthy-guarded short-circuiting arrow IIFE as
  `Expr::LogicalAnd`/`LogicalOr` (Ruby short-circuit + SIR truthiness).
- `not` → `(!__Sir.truthy(x))`; `neg` → `(-(x))`.
- Remaining audit-found builtins (`range`, `splat`, `double_splat`,
  `block_pass`, `yield`, `lambda`, `defined?`) are call-ABI / control-flow
  shaped and tracked as a follow-up; until then they hit core's now-descriptive
  unknown-builtin error rather than a cryptic failure.

New direct-SIR tests for `and`/`or`/`not`/`neg` native lowering.

## 0.1.10 — backtick builtin → sir-runtime-shell (gated import)

The Ruby `` `cmd` `` backtick literal lowers to `BuiltinCall("backtick",
[cmd])`, which previously hit the unknown-builtin dispatch and threw at runtime.
It now emits a call into the new `@coding-adventures/sir-runtime-shell` package
(thin `child_process.execSync` wrapper: run via the system shell, return stdout
— Ruby backtick semantics), per `code/specs/sir-runtime.md`.

- `BuiltinCall("backtick", …)` → `__SirShell.backtick(cmd)`.
- New gated `RUNTIME_SHELL` import, emitted **only** when a module calls the
  `backtick` builtin (gate `uses_shell`, reusing the `module_uses_builtin`
  content walk).
- New direct-SIR tests assert the gated import + `__SirShell.backtick("echo
  hi")` and that a non-shell module omits the import. The runtime package's own
  vitest suite covers execution. The shell command is **author-supplied** from
  the compiled program's source — no new untrusted-input path; the package
  declares a `proc`/`exec` capability.

## 0.1.9 — regex builtin → sir-runtime-regex (gated import)

The Ruby `/pat/flags` literal lowers to `BuiltinCall("regex", [pattern,
flags])`, which previously hit the unknown-builtin dispatch and threw at
runtime. It now emits a call into the new `@coding-adventures/sir-runtime-regex`
package (native `RegExp` compile with Ruby→JS flag translation), per
`code/specs/sir-runtime.md`.

- `BuiltinCall("regex", …)` → `__SirRegex.compile(pattern, flags)`.
- New gated `RUNTIME_REGEX` import, emitted **only** when a module calls the
  `regex` builtin. Because regex carries no SIR `Feature`, the gate
  (`uses_regex`) is a content walk — an exhaustive `Stmt`/`Expr` recursion that
  finds a `BuiltinCall` by name.
- New direct-SIR tests assert the gated import + `__SirRegex.compile("ab+c",
  "i")` and that a non-regex module omits the import. The runtime package's own
  vitest suite (100%) covers flag translation and unanchored matching.

## 0.1.8 — pairs extracted to sir-runtime-pairs (gated import)

Cons pairs (`cons`/`car`/`cdr`/`pair?`) now ship in the dedicated
`@coding-adventures/sir-runtime-pairs` package (core re-exports them for
back-compat), per `code/specs/sir-runtime.md` (per-concern runtime modules).

- New gated `RUNTIME_PAIRS` import header (`import * as __SirPairs from
  "@coding-adventures/sir-runtime-pairs"`), emitted **only** when a module uses
  the `Pairs` feature (`uses_pairs`). Pure non-pair modules no longer import the
  pairs helpers at all (they previously rode along in the always-on core header).
- `emit_builtin_call` now routes `cons`/`car`/`cdr` → `__SirPairs.cons/car/cdr`
  and `pair?` → `__SirPairs.isPair`; `null?` stays `__Sir.isNull` (a nil test,
  not a pair op).
- New direct-SIR tests assert the gated import + `__SirPairs.*` call sites and
  that a non-pair module omits the import. Cross-package display wiring (core
  injects `toDisplay` into the pairs package) is covered by the core package's
  own vitest list-display tests.

## 0.1.7 — SIR17 exceptions (native try/catch + sir-runtime-exceptions)

Accepts and emits the SIR17 `Exceptions` feature, per
`code/specs/sir-runtime.md`. `begin/rescue/ensure` translates to a **native**
`try { … } catch (__exc) { … } finally { … }`; the two pieces with no faithful
native equivalent come from the new `@coding-adventures/sir-runtime-exceptions`
package, imported (as `__SirExc`) **only** when a module throws or rescues.

- `Stmt::TryCatch{body, rescues, ensure_body}` → `try { body }`. Because a
  native `catch` binds one variable and catches everything while Ruby has an
  ordered list of typed `rescue` clauses, the `catch (__exc)` body is an
  if/else-if chain calling `__SirExc.rescueMatches(__exc, [class names])` per
  clause in source order; a `rescue Foo => e` binds `const e = __exc`; if no
  clause matches the original exception is re-`throw`n (Ruby's "propagate when
  unrescued"). `ensure_body` → a `finally` block (omitted when absent).
- `BuiltinCall("raise", …)` → `__SirExc.raiseError(…)`: a `Const` class operand
  (`raise Foo` / `raise Foo, "m"`) is passed as its *name string* with the
  optional message; a non-`Const` first arg (`raise "m"`) becomes an implicit
  `RuntimeError` carrying that message; bare `raise` → a generic re-raise.
- `collect_assigned_locals` now descends into try/rescue/ensure bodies so an
  outer local reassigned inside a `begin` still emits `let`.
- `ACCEPTED_FEATURES += Exceptions`.

New Ruby→TS and direct-SIR tests (begin/rescue/ensure shape, message-only
`raise`, bare-rescue catch-all + rethrow, non-throwing module omits the import).
Emitted output verified to execute on Node against the real
`@coding-adventures/sir-runtime-exceptions` (ancestor-matched rescue with bound
message, `ensure` runs, unmatched exception propagates). Mirrors the Python
backend's Q7b.

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
