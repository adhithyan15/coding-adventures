# Changelog

## 0.10.1 — `is_python_keyword` missing two of Python's four soft keywords (task #116 audit)

Follow-up to task #110/#112 (`semantic-ir-to-javascript`/`-typescript`'s
`eval`/`arguments` gap): a broader audit of every `semantic-ir-to-*`
backend's reserved-word check for the same class of bug — a contextual
keyword, reserved only in some grammar positions, missing from the
identifier-safety list.

`is_python_keyword` (`emit.rs`) already treated `match` and `case` as
unsafe defensively, even though neither is a syntactic keyword — both are
"soft keywords" (`keyword.softkwlist`), reserved only inside a `match`
statement's own grammar, and otherwise ordinary identifiers. But Python's
official soft-keyword set has *four* entries, not two: `_`, `case`,
`match`, `type` (`_` and `match`/`case` since 3.10's structural pattern
matching, PEP 634-636; `type` since 3.12's type-alias statement, PEP
695). `_` and `type` were missing, so a SIR identifier named `_` or
`type` was previously emitted verbatim by `sanitize_ident` instead of
being suffixed.

Fixed by adding both to the existing soft-keyword group in
`is_python_keyword`'s `matches!` list (same style as the existing
`match`/`case` entries; no restructuring). New unit test
`is_python_keyword_flags_remaining_soft_keywords` pins both as reserved,
confirms `sanitize_ident` suffixes them (`_` → `__`, `type` → `type_`),
and confirms ordinary look-alike identifiers (`_x`, `typing`, `types`)
are untouched.

## 0.10.0 — operator-spelling comparisons: `==`, `!=`, `<=`, `>=`

The Ruby frontend lowers a comparison chain to `==`/`!=`/`<=`/`>=` builtins,
which the Python backend did not lower — so `puts(1 == 1)` raised
`NameError: SIR builtin '==' is not implemented`.

The emitter now maps `==`→`_sir_eq` (a synonym for `=`), `!=`→`_sir_ne`,
`<=`→`_sir_le`, `>=`→`_sir_ge`, and imports the three new helpers from
`coding-adventures-sir-runtime-core` 0.2.0. The `call_builtin` dispatch table in
core gains the same names, so a first-class `:==` symbol dispatches too.

## 0.9.0 — source-language display convention: Ruby booleans (`true`/`false`)

Emits the display-convention selection (SIR display-convention spec) for the
Python backend. A **Ruby**-sourced module now emits, once after the core
runtime import, `_sir_set_display_convention("ruby")`, so a translated
`puts true` prints `true` instead of the Twig/Lisp `#t`. Requires
`coding-adventures-sir-runtime-core` ≥ 0.1.9 (which adds
`set_display_convention`).

Non-Ruby modules emit **nothing extra** — no setter call, no additional import
— so existing Twig output is byte-for-byte unchanged. The emitted argument is a
hardcoded `"ruby"` literal chosen by an exact `source_language == "ruby"` check
(never source-derived → no injection).

Scope: booleans only (the flagship divergence); `nil`, symbols, string
`inspect` quoting, and the Ruby hash `=>` form remain follow-ups per the spec.
Verified end-to-end under `python` against the real runtime library: Ruby
convention → `true`/`false`; default → `#t`/`#f`.

## 0.8.0 — mixin emit arms: `include` / `extend` (MX2 of sir-mixins)

### Added

- **`__include__` / `__extend__` emit arms.** The MX1 frontend lowers
  `include M` / `extend M` (in a class or module body) to
  `__include__("Owner", "M")` / `__extend__("Owner", "M")`. Two new arms in the
  builtin-call emitter route these to the OOP runtime's explicit tables:
  - `__include__("Owner", "M")` → `_sir_oop_include_module("Owner", "M")`
    (appends `M` to the owner's included-modules list, consulted by the MRO
    walk).
  - `__extend__("Owner", "M")` → `_sir_oop_extend_module("Owner", "M")` (copies
    `M`'s instance methods into the owner's class-method table).
  Both names are emitted via `quote_py_string`, never interpolated raw — dispatch
  stays fully table-driven (the C3 dynamic-dispatch RCE lesson).
- **OOP-runtime import gate** now fires on `__include__` / `__extend__`, and the
  `RUNTIME_OOP` import header aliases `include_module`/`extend_module` to
  `_sir_oop_include_module`/`_sir_oop_extend_module`.

### Tests

- Unit: `oop_include_emits_include_module`, `oop_extend_emits_extend_module`
  (emit shape); the runtime-alias test asserts both new aliases.
- End-to-end (Ruby → SIR → Python → CPython): a module method `include`d into a
  class and called on an instance (`hi`); a class method **shadowing** the
  included module's (`class`); `extend` making a module method a **class**
  method (`7`). See `code/specs/sir-mixins.md`.

## 0.7.0 — `puts` builtin (Ruby semantics)

### Changed

- `puts` now maps **directly** to the variadic runtime helper `_sir_puts(...)`
  (like `print` → `_sir_print`) instead of routing through the generic
  `_sir_call_builtin("puts", […])` dispatch. This is possible now that
  `sir-runtime-core` implements Ruby `puts` semantics; the previous
  dispatch-table routing would have failed at runtime because `puts` was not
  registered. A `sir_puts as _sir_puts` import alias was added to the runtime
  header. Existing Ruby→Python e2e tests were updated to the new call shape,
  and `end_to_end_ruby_to_python_puts` now runs the emitted Python under a real
  interpreter and asserts `puts "hello"` prints exactly `hello\n`.

## [0.6.0] - 2026-07-01

### Added (Issue #59 — class-method CALL dispatch)

- **`__class_method__("Class", "method", …args)` → `_sir_oop_call_class_method`.**
  The Ruby frontend (ruby-to-semantic-ir 0.5.0) now emits `__class_method__`
  for a class-method call on a constant receiver (`Counter.zero`); the emitter
  routes it to the OOP runtime's ancestry-walking `call_class_method`. The
  helper is added to the `RUNTIME_OOP` import header and to the OOP-import
  gating list (`is_oop`), so a module using class-method calls pulls in the
  runtime dependency.

### Tests (Issue #59 — class-method + super-as-expression execution proofs)

- `end_to_end_ruby_class_method_def_and_call_executes_py` — real
  `def self.zero; Counter.new; end` factory; `Counter.zero.val` runs and prints
  `42` under a live interpreter.
- `end_to_end_ruby_super_as_expression_executes_py` — `def describe; super + 1; end`
  with inheritance; the parent's `describe` (40) flows into the enclosing `+`,
  printing `41`. (Uses numeric `+`; string `super + "…"` is a pre-existing
  `_sir_plus` gap — see ruby-to-semantic-ir CHANGELOG.)
- Both use `print` rather than `puts`, since `puts` has no runtime dispatch
  entry on this branch (a parallel PR adds it).

## Unreleased

### Tests (O2 — Ruby OOP end-to-end execution proofs)

Added three execution-proof tests (no source change) that lower real Ruby OOP
source through `ruby-to-semantic-ir`, compile the SIR to Python, and run it
under a real interpreter — proving the O1 emit arms + `sir-runtime-oop` runtime
execute the frontend's O2 production:

- `end_to_end_ruby_oop_new_and_method_dispatch_executes_py` (P1) →
  `Rex says woof` — construction, instance-method dispatch, `@ivar`.
- `end_to_end_ruby_inheritance_super_executes_py` (P2) → `Tom with 4 legs` —
  inheritance + `super` on the shared self.
- `end_to_end_ruby_attr_accessor_and_self_chain_executes_py` (P3) → `2` —
  `attr_accessor` getter/setter, `@ivar` mutation, `self`-return chaining.

## 0.5.0 — OOP object-model emit arms (O1)

Additive emit support for the object-model builtins the Ruby frontend will
produce in O2. **No existing program changes behaviour** — nothing emits these
builtins yet; the arms only fire once they appear, at which point they route to
the `sir-runtime-oop` method-table helpers.

- **New `emit_builtin_call` arms**, mirroring the existing `__method__` →
  `_sir_oop_call_method` routing:
  - `__new__(class, ...ctor_args)` → `_sir_oop_call_new(class, args…)`
  - `__super__(method, class, ...args)` → `_sir_oop_call_super(method, class, args…)`
  - `__def_method__(class, method, closure)` → `_sir_oop_def_method(...)`
  - `__def_class_method__(class, method, closure)` → `_sir_oop_def_class_method(...)`
  - `__self__()` → `_sir_oop_current_self()`
  Class/method-name `StrLit`s are emitted through the normal expression path, so
  they route through `quote_py_string` — never raw interpolation of a
  source-derived name.
- **Import gating** (`uses_oop`) now also fires on the new builtins, so the OOP
  runtime import is present whenever any of them is emitted, and the import
  header aliases `call_new`/`call_super`/`def_method`/`def_class_method`/
  `current_self` to their `_sir_oop_*` names.
- **Tests:** emit-shape unit tests for each arm; an import-gating test; and an
  end-to-end execution proof — a hand-built `Dog#speak` module (`__def_method__`
  + `__new__` + `__method__`) run through a real Python interpreter prints
  `Rex says woof`, proving the emit → runtime wiring executes.

## 0.4.0 — user-defined exception-class ancestry (E2)

The backend now threads user `class Child < Parent` inheritance edges into the
exception matcher, so a `rescue StandardError` catches a raised user
`MyErr < StandardError` — not just built-in subclasses.

- **Emit.** For any module that uses `Feature::Exceptions` **and** declares at
  least one class with a superclass, the emitter collects every
  `Stmt::ClassDef { name, superclass: Some(sc) }` pair (walking nested class /
  module / try / loop bodies) and emits a single program-init
  `_sir_exc_register_ancestry({"MyErr": "StandardError", …})` call, placed
  after the runtime imports and before any function or `main` runs. Modules with
  no superclass edge emit no registration (no empty, meaningless call).
- **Runtime alias.** The exception-runtime import header now also aliases
  `register_ancestry as _sir_exc_register_ancestry`.
- Built-in matching is unchanged; user edges are purely additive. Requires
  `coding-adventures-sir-runtime-exceptions >= 0.2.0`.

## 0.3.0 — keyword-parameter & keyword-argument emission (KW2)

The backend now accepts `Feature::KeywordParams` and lowers keyword parameters
and keyword arguments to their **native Python** forms — replacing the KW1
compile-compat stubs (the panicking `Expr::KeywordArg` emit arm and the
positional-fold of `ParamKind::Keyword`).

Python keyword parameters are *keyword-only* parameters: they sit after a `*`
in the signature and bind by name only.

- **Def side — keyword-only params.** A `ParamKind::Keyword` param becomes a
  keyword-only parameter. The backend injects a single bare `*` separator
  immediately before the first keyword param — **unless** a `Rest` (`*args`)
  param is already present, since `*args` itself opens the keyword-only region
  and a second bare `*` would be a `SyntaxError`. The validator's def-side
  ordering (positional → `Rest` → `Keyword*` → `KwRest`) guarantees a single
  whole-list lookahead for a `Rest` suffices. Examples:
  - `[a(Required), b(Keyword,None), c(Keyword,default 1)]` → `def f(a, *, b, c=_SIR_MISSING):`
  - `[rest(Rest), kw(Keyword,default 1)]` → `def g(*rest, kw=_SIR_MISSING):` (no extra `*`)
- **Optional keyword defaults reuse the P2c machinery.** An optional keyword
  param (`Keyword` + `default: Some`) emits the sentinel `name=_SIR_MISSING`
  and is resolved in the body prologue — exactly like a positional optional —
  so keyword defaults are **call-time** and may reference earlier params. (This
  is a deliberate departure from a literal `name=<default-expr>` in the def, for
  the same NameError / evaluation-time reasons documented in 0.2.0.)
- **Call side — keyword arguments.** An `Expr::KeywordArg { name, value }`
  (which the validator permits only inside a call's `args`, after all
  positionals) emits as `name=value`, e.g. `greet("hi", name="ada")`.

Verified with emitted-shape tests (required + optional keyword def, `*`-vs-`*args`
separator, keyword call arg) and execution-proof tests that shell out to
`python3`/`python` (skipped gracefully if absent): `greet("hi")` → `hi, world`
and `greet("hi", name="ada")` → `hi, ada`.

### KW7 — Ruby-frontend-driven execution proof (test only; emission unchanged)

Once the Ruby frontend (`ruby-to-semantic-ir` 0.3.0) began PRODUCING keyword
params/args (KW7), a full-pipeline execution-proof was added here (the natural
home, since this crate already dev-depends on the Ruby frontend): Ruby SOURCE
`def greet(greeting:, name: "world")\n "#{greeting}, #{name}"\nend\n
print greet(greeting: "hi")\nprint greet(greeting: "hi", name: "ada")` →
Ruby SIR → Python → CPython prints `hi, world` then `hi, ada`. This exercises
the KW2 keyword-only-def + `name=value`-call emission end to end from real Ruby
syntax (78 tests, up from 77). No emission behaviour changed, so the crate
version is unchanged.

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
