# Changelog

## 0.2.0 — default-parameter emission via sentinel + body prologue (P2c)

The backend now accepts `Feature::DefaultParams` and lowers default parameters.

SIR defaults are **call-time** and may reference **earlier params** (e.g. a
default of `a + 1`), which Python's native def-time defaults cannot express —
`def f(a, b=a)` raises `NameError`, and even a constant default would evaluate
once at definition time, not per call. So native default syntax is wrong for
this model. Instead:

- **Sentinel.** The core runtime header now defines a module-level
  `_SIR_MISSING = object()` — a unique value distinct from every real argument
  (including `None`).
- **Signature.** A defaulted param emits `name=_SIR_MISSING` (its native Python
  default is the sentinel), so callers may omit the trailing argument.
- **Resolve-prologue.** The function body opens with, in **param order**, one
  guard per defaulted param:

  ```python
  if name is _SIR_MISSING:
      name = <default expr>
  ```

  The default expression is emitted through the ordinary expr path and runs in
  the body, where earlier params are already bound — giving call-time,
  param-scoped semantics correctly. Param order guarantees an earlier defaulted
  param is resolved before a later default that references it.
- **Calls.** `DirectCall` is unchanged: it emits only the arguments present (no
  padding). An omitted trailing defaulted argument leaves the param bound to the
  sentinel, which the prologue then resolves. `IndirectCall`/closure defaults
  are deferred.

Execution-proof (via the PYTHONPATH-aware `run_emitted_python` harness):
`def f(a, b)` with `b`'s default `a + 1`, returning `b`; `print(f(5))` prints
`6` (default resolved against the earlier param) and `print(f(5, 10))` prints
`10` (supplied argument suppresses the default).

## 0.1.23 — case-equality `case_eq` emission (M5)

A `case_eq` builtin (emitted by a `when` clause for range/regex/literal
patterns) now routes to `_sir_oop_case_eq(pattern, value)`. The OOP import
header gains the alias, and `uses_oop` fires on `case_eq` so a class-less
`case/when` still imports `sir-runtime-oop`. Execution-proof: a `case/when` over
`when 10..20` / `when /hi/` / `when Integer` / `else` dispatches by Ruby
case-equality and prints the right branch per scrutinee. The shared
`run_emitted_python` harness now also puts `sir-runtime-range` and
`sir-runtime-regex` on `PYTHONPATH` so range/regex programs run.

## 0.1.22 — coverage: outer-local block captures (M4)

No emitter change — the backend already prepends `MakeClosure` captures as
leading parameters. Adds an execution-proof that a block reading an enclosing
local (`def run; base = 100; apply { |n| print n + base }; end`) emits
`def __block_0(base, n):` with `base` threaded by the closure, and prints
`105` through a real interpreter. The frontend now produces such captures (see
`ruby-to-semantic-ir` 0.97.0, M4).

## 0.1.21 — variadic parameter emission (`*args` / `**kwargs`) (M3)

A `Param` carrying the new SIR `ParamKind` now emits Python's native variadic
forms: `Rest` → `*name`, `KwRest` → `**name`; `Required` is unchanged. So
`def f(a, *rest, **opts); end` emits `def f(a, *rest, **opts):`.

- **Rest-param list normalization.** Python's `*rest` binds a *tuple*, but SIR
  sequence semantics (and Ruby's `*rest`, an `Array`) require a *list* — every
  downstream sequence op (`len`, indexing, dispatched `.map`/`.length`) is keyed
  to `list`. Each `Rest` param is therefore rebound to a `list(...)` in the
  function prologue. (`**opts` already binds a `dict`, matching SIR's map, so
  no fixup.)
- **OOP import gating widened.** `uses_oop` now also fires on the `__method__`
  and `__scope__` dispatch builtins, not only the OOP *features* — so a
  class-less dispatch program (`"hi".upcase`, or a rest param used as an Array)
  imports `sir-runtime-oop`. Previously such modules emitted an undefined
  `_sir_oop_call_method` call.
- Execution-proof: `def f(*a); a.length; end; print f(1, 2, 3)` → `3` through a
  real interpreter (skips gracefully if absent). The shared run harness now
  names its temp file uniquely per call so concurrent proofs don't collide.

## 0.1.20 — `&:sym` symbol-to-proc on dispatched calls (M2)

A `&:sym` block argument on a method-dispatch call (`recv.map(&:to_s)`) now
emits working code. The Ruby→SIR frontend lowers `&:sym` to
`block_pass(SymLit("sym"))`; the Q9f normalization unwraps `block_pass` only at
*user-method* `DirectCall` sites, so a block-pass to a `__method__` dispatch
call reached the backend intact and previously rendered as a broken
`call_builtin("block_pass", …)`. `emit_arg` and the `__method__` argument loop
now recognize the surviving envelope:

- `block_pass(SymLit("m"))` → `_sir_oop_sym_to_proc(_sir_intern("m"))` — the
  `Symbol#to_proc` runtime helper returns a `Closure` that calls `recv.m(*rest)`;
- `block_pass(<other>)` → the inner operand, unwrapped (the proc *is* the block).

Adds `sym_to_proc as _sir_oop_sym_to_proc` to the OOP runtime import header.
New tests `sym_block_pass_on_dispatch_emits_sym_to_proc`,
`proc_block_pass_on_dispatch_unwraps_to_value`,
`sym_block_pass_as_plain_arg_emits_sym_to_proc`. Requires
`coding-adventures-sir-runtime-oop` ≥ 0.1.6. Operator symbols (`&:+`) remain a
documented v0 boundary (native arithmetic, not dispatched).

## 0.1.19 — `defined?(recv.meth)` → "method" (Q10h)

`defined?` over a method-call operand `recv.meth` — the `__method__` dispatch
envelope — now emits the constant `"method"` (Ruby's category when the method
resolves) instead of the generic `"expression"`. Purely emit-time shape
inspection; the non-evaluation contract is unchanged (the receiver, method
name, and call are never rendered). The runtime respond_to?-presence check that
would return `nil` for an absent method, and arbitrary built-in/collection
method dispatch (`arr.each`, `&:sym`'s target, …) which `call_method` returns
`nil` for, are the documented terminal method-dispatch boundary (see
`code/specs/sir-runtime.md`). New test `defined_method_call_operand_emits_method_py`.

## 0.1.18 — lambda marked strict-arity (Q10g)

`make_closure` now produces a **proc-lenient** closure (the runtime's `apply`
adjusts a block's arguments to its arity), so a Ruby lambda — which must stay
**strict** — is wrapped: the `lambda` / `->(){}` builtin arm now emits
`_sir_as_lambda(_sir_make_closure(…))` instead of the bare closure. The new
`as_lambda` helper (imported from `sir-runtime-core` ≥ 0.1.4) flips the
closure's strict flag so an arity mismatch raises rather than being silently
adjusted. Block / proc closures are unchanged in shape and now adjust arity at
call time. `lambda_builtin_lowers_to_inner_closure_py` gains a wrapper
assertion; the runtime-import alias test lists `as_lambda as _sir_as_lambda`.

## 0.1.17 — execution-proof for the non-empty block capture (RB3)

RB1/RB2 (in the Ruby frontend) introduced the first SIR shape that emits a
**non-empty** `MakeClosure` capture: a block passed to a yielding method whose
own body yields to the *enclosing* method's block.  The inner block is hoisted
to a top-level function that closes over the enclosing method's `__sir_block__`
parameter, and the backend binds that by emitting
`_sir_make_closure(__block_0, [__sir_block__])` plus a hoisted
`def __block_0(__sir_block__, x):` (captures are prepended to the parameter
list).

Until now every Ruby→Python test asserted only the *shape* of the emitted
source.  This release adds `end_to_end_ruby_block_capture_executes_py`, which
additionally **runs** the emitted module through a real Python interpreter
(runtime packages on `PYTHONPATH`) and asserts stdout — proving the captured
block actually reaches the enclosing caller's block at runtime, not just on
paper.  The harness probes `python3`/`python` with `-c pass` and **skips the
execution assertion** when no usable interpreter is present (the Windows Store
`python3` stub is treated as absent), so hosts without Python never hard-fail;
a present-but-erroring interpreter still fails the test.

No emitter change — this is verification only.  The receiver-form
(`recv.each { … }`) of the same capture lowers and emits identically, but its
*execution* additionally depends on collection-method dispatch in
`sir-runtime-oop` (`call_method` currently returns `nil` for `each`), which is
the Q10h method-dispatch boundary and out of scope here.

## 0.1.16 — `defined?` lowers to a non-evaluating description (Q9d)

Fourth item of the Q9 structural-builtin tranche.  Ruby `defined?(x)` reaches
the backend as `BuiltinCall("defined?", [operand])` (a `PURE` builtin whose
operand must **never** be evaluated — `defined?(expensive_call)` must not call
it).  Previously it fell through to the eager dispatch
(`_sir_call_builtin("defined?", [operand])`), which both lacked a `defined?`
entry **and** evaluated the operand.  `emit_builtin_call` now inspects the
operand's SIR shape at emit time and emits a constant description string,
without ever rendering the operand:

- local / param / capture `VarRef` → `"local-variable"`
- `Const` `VarRef` → `"constant"`
- `Instance` (`@x`) → `"instance-variable"`; `ClassVar` (`@@x`) → `"class variable"`;
  `Global` (`$x`) → `"global-variable"`; builtin-name → `"method"`
- any other expression (literal, method call, …) → `"expression"`

The non-evaluation contract holds for **every** shape (the property that
matters).  v0 simplifications (documented in `code/specs/sir-runtime.md`):
instance/class/global vars report their static description rather than the
runtime `nil`-when-unset (the per-concern runtimes expose no presence predicate
yet), and a general/method operand reports the generic `"expression"` rather
than Ruby's exact category.  Tests: `defined_local_var_emits_static_description_py`
and `defined_does_not_evaluate_operand_py` (asserts the operand literal is never
emitted).

## 0.1.15 — `splat` / `double_splat` lower to native spread (Q9c)

Third item of the Q9 structural-builtin tranche.  Ruby `*x` / `**x` reach the
backend as `BuiltinCall("splat", [x])` / `BuiltinCall("double_splat", [x])` —
sitting as a trailing call argument or as an array element.  Previously these
fell through to the eager dispatch (`_sir_call_builtin`), which has no entry for
them, so any splat-call crashed at runtime.  `emit_args` now expands them into
Python's faithful native spread syntax via a new `emit_arg` helper:

- `splat` → `*x`  (e.g. `f(*a)`, `[1, *mid, 3]`)
- `double_splat` → `**x`  (e.g. `f(**h)` — keyword spread)

Tests: `splat_in_seq_literal_emits_native_spread_py` (`[1, *mid, 3]`) and
`splat_and_double_splat_call_args_emit_native_py` (`(*a, **h)`), both asserting
the dispatch fallthrough is gone.

## 0.1.14 — `range` builtin lowers to the range runtime (Q9b)

Second item of the Q9 structural-builtin tranche.  Ruby `a..b` / `a...b` (and
the begin/endless `a..` / `..b` forms) reach this backend as
`BuiltinCall("range", [start, stop, exclusive])`.  Python's `range` is half-open
and integer-only and cannot model the inclusive or begin/endless forms, so
`emit_builtin_call` now lowers it to `_sir_range(...)` from the new per-concern
package `coding-adventures-sir-runtime-range` (the SIR first-class `Range`
value).  A `uses_range` content-walk gates the `RUNTIME_RANGE` import header, so
pure modules never gain the dependency.  Tests:
`range_builtin_lowers_to_runtime_and_imports_py` (emits `_sir_range(1, 5, False)`
+ the import, never the dispatch fallthrough) and `no_range_import_when_unused_py`.

## 0.1.13 — `lambda` builtin lowers to its inner closure (Q9a)

First item of the structural-builtin follow-up tranche (Q9) the Q8d audit
flagged.  Ruby `lambda { … }` / `->(x){…}` reach this backend as
`BuiltinCall("lambda", [MakeClosure])`.  The lambda *is* its closure value, so
`emit_builtin_call` now emits the inner `MakeClosure` directly (rendering
`_sir_make_closure(...)`) instead of routing through the eager `call_builtin`
dispatch — there is no separate `lambda` runtime helper, the closure already is
the result.  Reuses the existing `MakeClosure` emission and `make_closure`
runtime helper; no runtime change.  Direct-SIR test
`lambda_builtin_lowers_to_inner_closure_py`.

## 0.1.12 — boolean/unary operator builtins (audit close-out)

Builtin-coverage audit of what the Ruby frontend actually emits as a
`BuiltinCall` reaching this backend.  Ruby `&&`/`and`, `||`/`or`, `!`/`not`,
and unary minus lower to `BuiltinCall("and"/"or"/"not"/"neg", …)` — previously
they fell through to the eager `call_builtin` dispatch and raised at runtime
(so any program using a boolean operator emitted crashing code).  Now they
lower **natively**:

- `and`/`or` → the same truthy-guarded short-circuiting lambda as
  `Expr::LogicalAnd`/`LogicalOr` (Ruby short-circuit + SIR truthiness; the rhs
  is not evaluated when the lhs decides the result — eager dispatch could not
  preserve this).
- `not` → `(not _sir_truthy(x))` (always a bool); `neg` → `(-(x))`.
- The remaining builtins the audit found reaching the backend — `range`,
  `splat`, `double_splat`, `block_pass`, `yield`, `lambda`, `defined?` — are
  call-ABI / control-flow shaped and tracked as a follow-up tranche; until then
  they hit core's now-descriptive unknown-builtin error (names the builtin +
  flags a backend coverage gap) instead of a cryptic failure.

New direct-SIR tests for `and`/`or`/`not`/`neg` native lowering.

## 0.1.11 — backtick builtin → sir-runtime-shell (gated import)

The Ruby `` `cmd` `` backtick literal lowers to `BuiltinCall("backtick",
[cmd])`, which previously hit the unknown-builtin dispatch and failed at
runtime. It now emits a call into the new `coding-adventures-sir-runtime-shell`
package (thin subprocess wrapper: run via the system shell, return stdout —
Ruby backtick semantics), per `code/specs/sir-runtime.md`.

- `BuiltinCall("backtick", …)` → `_sir_shell_backtick(cmd)`.
- New gated `RUNTIME_SHELL` import header, appended **only** when a module calls
  the `backtick` builtin (gate `uses_shell`, reusing the `module_uses_builtin`
  content walk).
- New direct-SIR tests assert the gated import + `_sir_shell_backtick("echo
  hi")` and that a non-shell module omits the import. Exec-proofed on CPython
  against the real package (`` `python -c "print(7*6)"` `` → `42`). The shell
  command is **author-supplied** from the compiled program's source (a string
  literal), mirroring Ruby's own backtick — no new untrusted-input path; the
  package declares a `proc`/`exec` capability.

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
