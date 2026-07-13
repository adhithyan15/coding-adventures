# Changelog

## 0.9.0 — SIR23 symbolic/pattern codegen (HML01 Stream B, item 7)

Real codegen for the SIR23 symbolic-expression + pattern/rewrite domain —
previously deferred (panicking on `Expr::SymSymbol`/`SymRational`/`SymApply`/
`SymPatternBlank`/`SymPatternNamed`/`SymRule`/`SymReplaceAll`, gated out at
the capability check). A compiled Wolfram/Macsyma/Maxima program using
pattern-matching or rewrite rules (`x /. a -> b`, `x //. rules`) now compiles
to TypeScript that constructs/consumes a real term-tree value at runtime via
the imported `@coding-adventures/sir-runtime-symbolic` package, bound as
`__SirSym`:

- `SymSymbol`/`SymRational` → `__SirSym.sym(...)`/`__SirSym.rational(...)`.
- `SymApply { head, args }` → `__SirSym.apply(head, [args...])`, recursing
  through `emit_sym_operand` — a child that is an `IntLit`/`FloatLit`/
  `StrLit` (the three literal kinds SIR23 "reuses directly" rather than
  defining new leaf nodes for) gets wrapped into the matching
  `__SirSym.int`/`numberNode`/`stringNode` constructor, since a bare host
  number/string is never a valid `IRNode` term; every other child (a nested
  symbolic node, or a `VarRef`/call whose value is already a term by the
  frontend's own convention) emits unchanged.
- `SymPatternBlank`/`SymPatternNamed` → `__SirSym.blank()`/`blankTyped(...)`/
  `named(...)`.
- `SymRule { delayed }` → `__SirSym.rule(...)` (`->`) or `ruleDelayed(...)`
  (`:>`).
- `SymReplaceAll { repeated }` → `__SirSym.unwrap(__SirSym.replaceAll(...))`
  (`/.`) or `unwrap(replaceRepeated(...))` (`//.`) — wrapped in `unwrap`
  because both rewrite functions can return a `DepthLimitError`/
  `RewriteCycleError` sentinel instead of a real term; a compiled
  `SymReplaceAll` must evaluate to a term value or fail loudly, never
  silently hand that sentinel to code expecting an `IRNode`.

Accepts `Feature::SymbolicExpr`, `Feature::PatternMatching`, and
`Feature::Rationals` (the last shared with SIR22, scoped exactly to
`SymRational` — no other construct in this backend triggers it yet). The
`@coding-adventures/sir-runtime-symbolic` import is gated by `uses_symbolic`
(either feature), so a purely-numeric module never gains the dependency.

Required a small, additive extension to `@coding-adventures/sir-runtime-symbolic`
itself (0.1.0 → 0.2.0): re-exports of `symbolic-ir`'s leaf-term constructors
(`sym`/`int`/`rational`/`numberNode`/`stringNode`, previously missing) and a
new `unwrap(result)` helper for the depth/cycle-error-sentinel handling
above.

Tested with unit shape assertions for every new node kind (leaf constructors,
literal wrapping, both pattern-blank forms, `rule` vs `ruleDelayed`,
`replaceAll` vs `replaceRepeated`, the import gate) plus a real end-to-end
test compiling actual Wolfram source (`x /. a -> b`) through
`wolfram-to-semantic-ir` and this backend, not just hand-built `Module`s.

The JavaScript backend (`semantic-ir-to-javascript`) still defers these nodes
— it inlines its own runtime rather than importing packages, so SIR23 there
needs a from-scratch inlined term-rewriting engine, not a straightforward
import; tracked as follow-up work.

## 0.8.0 — source-language display convention: Ruby booleans (`true`/`false`)

Emits the display-convention selection (SIR display-convention spec) for the
TypeScript backend — the **last** of the five backends, completing the boolean
convention across the whole stack. A **Ruby**-sourced module now emits
`__Sir.setDisplayConvention("ruby");` once after the runtime import, so a
translated `puts true` prints `true` instead of the Twig/Lisp `#t`. Requires
`@coding-adventures/sir-runtime-core` ≥ 0.1.10 (which adds
`setDisplayConvention`).

Non-Ruby modules emit **nothing extra** (the setter uses the already
namespace-imported `__Sir`, so there is no new import either) — existing Twig
output is byte-for-byte unchanged. The emitted argument is a hardcoded `"ruby"`
literal chosen by an exact `source_language == "ruby"` check (never
source-derived → no injection).

Scope: booleans only (the flagship divergence); `nil`, symbols, string
`inspect` quoting, and the Ruby hash `=>` form remain follow-ups per the spec.
Verified: the core lib's `setDisplayConvention`/`toDisplay` under `vitest`, and
an emitter unit test asserting a Ruby module emits the setter while a Twig
module does not.

## 0.7.0 — Ruby mixins: `include` / `extend` emit arms (MX3)

Part of the `sir-mixins` cascade (spec `code/specs/sir-mixins.md`). Adds the
TypeScript emit arms for the mixin builtins the Ruby frontend (MX1) lowers, so
the emitted code drives the `@coding-adventures/sir-runtime-oop` MX3 runtime
(`0.1.11`). Class/method/module names all arrive as `StrLit`s and emit through
the normal expression path (`quote_ts_string`) — no source-derived name is ever
interpolated raw (the C3 RCE lesson).

### Added

- **`__include__("Owner", "M")` → `__SirOop.includeModule("Owner", "M")`** and
  **`__extend__("Owner", "M")` → `__SirOop.extendModule("Owner", "M")`** — the
  two mixin directives now route to the OOP runtime's include-list and
  class-method-copy helpers.
- **`__class_method__("Class", "method", …args)` →
  `__SirOop.callClassMethod(...)`** (issue #59, mirrored from the Python
  backend, which already had it). Previously a `Foo.bar` class-method call fell
  through to the generic `__Sir.callBuiltin("__class_method__", […])` dispatch
  on the TS side; it now routes to the OOP runtime, which is what makes a
  method mixed in via `extend M` callable as `Owner.method`.
- The OOP-runtime import gate now also fires on `__include__`, `__extend__`,
  and `__class_method__`.
- Execution proofs in `run_with_node.rs` (MX3): five tests lower real Ruby
  mixin programs → TypeScript → `node`, proving an included module method is
  callable, a class method shadows the module's, the most-recently-included
  module wins, a diamond include resolves once, and `extend` makes a module
  method a class method. Plus emit-shape unit tests for the three new arms.

## 0.6.0 — `puts` builtin (Ruby semantics)

### Added

- `puts` now maps **directly** to the variadic runtime helper `__Sir.puts(...)`
  (like `print` → `__Sir.print`) instead of routing through the generic
  `__Sir.callBuiltin("puts", […])` dispatch. This is possible now that
  `@coding-adventures/sir-runtime-core` implements Ruby `puts` semantics
  (string+newline, no-arg → one newline, arrays flattened element-per-line,
  trailing-newline suppression).
- Execution proof `run_with_node.rs::end_to_end_ruby_puts_executes_ts` lowers
  real Ruby `puts "hi"` through `ruby-to-semantic-ir`, compiles to TypeScript,
  and runs the result under `node`, asserting stdout is exactly `hi\n`.

## Unreleased

### Tests (PO2 — polymorphic `+`/`*` execution proof)

Added `tests/polymorphic_operators.rs` (no emitter change — `+`/`*` already lower
to `__Sir.add`/`__Sir.mul`; the polymorphic dispatch lives in the
`@coding-adventures/sir-runtime-core` runtime). The test builds SIR `+`/`*`
programs over strings and arrays, compiles them to TypeScript, and runs each
under `node` with a faithful inline `__Sir` stub whose `add`/`mul`/`toDisplay`
transcribe the PO2 runtime logic — proving end-to-end that `"a" + "b"` → `ab`,
`"ab" * 3` → `ababab`, `[1] + [2]` → the array `[1, 2]` (displayed `1,2`),
`[0] * 3` → `[0, 0, 0]` (`0,0,0`), `[1, 2] * ", "` → `1, 2`, and that numeric
`1 + 2` → `3` / `2 * 3` → `6` are unchanged.

### Tests (O2 — Ruby OOP end-to-end execution proof)

Added an execution-proof test (no source change),
`end_to_end_ruby_oop_new_and_dispatch_executes_ts`, that lowers real Ruby OOP
source (the P1 `Dog#speak` program) through `ruby-to-semantic-ir`, compiles the
SIR to TypeScript, and runs it under `node` with a faithful inline `__SirOop`
dispatch + instance-variable stub — proving the frontend's O2 production
(`__def_method__` / `__new__` / `__method__` / `@ivar`) executes on the TS side
and prints `Rex says woof`.

## 0.5.0 — OOP object-model emit arms (O1)

Additive emit support for the object-model builtins the Ruby frontend will
produce in O2. **No existing program changes behaviour** — nothing emits these
builtins yet; the arms only fire once they appear, at which point they route to
the `@coding-adventures/sir-runtime-oop` method-table helpers.

- **New `emit_builtin_call` arms**, mirroring the existing `__method__` →
  `__SirOop.callMethod` routing:
  - `__new__(class, ...ctor_args)` → `__SirOop.callNew(class, args…)`
  - `__super__(method, class, ...args)` → `__SirOop.callSuper(method, class, args…)`
  - `__def_method__(class, method, closure)` → `__SirOop.defMethod(...)`
  - `__def_class_method__(class, method, closure)` → `__SirOop.defClassMethod(...)`
  - `__self__()` → `__SirOop.currentSelfVal()`
  Class/method-name `StrLit`s are emitted through the normal expression path
  (`quote_ts_string`) — never raw interpolation of a source-derived name.
- **Import gating** (`uses_oop`) now also fires on the new builtins, so the
  `import * as __SirOop` header is present whenever any is emitted.
- **Tests:** emit-shape unit tests for each arm and an end-to-end execution
  proof — a hand-built `Dog#speak` module (`__def_method__` + `__new__` +
  `__method__`) run under `node` with a faithful method-table stub prints
  `Rex says woof`, proving the emit → runtime wiring executes.

## 0.4.0 — user-defined exception-class ancestry (E2)

The backend now threads user `class Child < Parent` inheritance edges into the
exception matcher, so a `rescue StandardError` catches a raised user
`MyErr < StandardError` — not just built-in subclasses.

- **Emit.** For any module that uses `Feature::Exceptions` **and** declares at
  least one class with a superclass, the emitter collects every
  `Stmt::ClassDef { name, superclass: Some(sc) }` pair (walking nested class /
  module / try / loop bodies) and emits a single program-init
  `__SirExc.registerAncestry({"MyErr": "StandardError", …})` call, placed after
  the runtime imports and before any function or `main` runs. Modules with no
  superclass edge emit no registration (no empty, meaningless call).
- Built-in matching is unchanged; user edges are purely additive. Requires
  `@coding-adventures/sir-runtime-exceptions >= 0.2.0`.

## 0.3.0 — keyword-parameter & argument emission (KW3)

Keyword parameters (`ParamKind::Keyword`) and keyword arguments
(`Expr::KeywordArg`) now lower to real TypeScript, replacing the KW1
compile-compat stubs. `Feature::KeywordParams` is added to the backend's
accepted feature set (mirroring `DefaultParams`). See
`code/specs/sir-keyword-params.md` §4.

- **Def side — trailing options object.** TypeScript has no native keyword
  arguments, so all of a function's `Keyword` params collapse into ONE trailing
  parameter, `__kw`, appended after the positionals. The function body opens
  with a destructure prologue binding each keyword:

  ```text
  def f(a, x:, y: 1)  →  function f(a: __Sir.Val, __kw: __Sir.Val): __Sir.Val {
                           const { x, y = 1 } = (__kw ?? {}) as { [k: string]: __Sir.Val };
                           …
                         }
  ```

  A *required* keyword (`default: None`) destructures as a bare name; an
  *optional* one (`default: Some(e)`) carries its default expression (`y = 1`),
  so an omitted optional falls back to it. `__kw ?? {}` tolerates a caller that
  supplies no keyword object at all (every keyword optional and omitted). The
  `__`-prefix is the backend's reserved namespace (`__Sir`, `__l`, …), so
  `__kw` cannot shadow a user binding. A function with no keyword params is
  byte-for-byte unchanged (no `__kw`, no prologue).

- **Call side — collapsed object literal.** Every `KeywordArg` in a call
  (always trailing, per the validator) collapses into one trailing object
  literal; positionals emit as before. `f(1, y: 2)` → `f(1, { y: 2 })`; a call
  with no keyword args emits no trailing object.

- **Direct lowering, no runtime helper** — same posture as `DefaultParams`.
  Default and value expressions lower through the ordinary expression emitter.

- **Tests.** Emitted-shape unit tests (destructure prologue for
  required+optional; call collapsing to one/two-entry objects; positional +
  keyword mix; no-keyword-args emits no object) plus a `node` execution proof
  (`tests/run_with_node.rs`): the spec's `greet(greeting:, name: "world")`
  program prints `hi, world` (optional omitted) and `hi, sir` (supplied), and a
  positional+keyword mix prints `A!` / `B-`. Node is optional — the test
  degrades to shape assertions when it is absent.

## 0.2.0 — default-parameter emission (P2b)

Default parameters now lower to TypeScript-native defaults. A `Param` whose new
SIR `default` field is `Some(expr)` emits `name: __Sir.Val = <default>`, where
`<default>` is the default expression run through the ordinary expression
emitter. So `def f(a, b = a + 1); b; end` emits
`function f(a: __Sir.Val, b: __Sir.Val = __Sir.add(a, 1)): __Sir.Val`.

- **Why native is exact.** A SIR default is conceptually evaluated per call in
  the callee's *parameter scope* and may reference *earlier* parameters.
  TypeScript's own default-value parameters have precisely these semantics
  (later defaults see earlier params; defaults run at call time when the
  argument is omitted), so the lowering is a direct inline — no runtime helper,
  no desugaring.
- **No call-site padding.** The core validator now lets a `DirectCall` omit
  trailing defaulted arguments. The emitter simply emits the arguments present
  (`f(5)` stays one argument); TS native defaults fill the omitted trailing
  parameters. `f(5, 10)` still passes both. `IndirectCall` is unchanged
  (closures-with-defaults deferred).
- **Capability.** `Feature::DefaultParams` is added to `ACCEPTED_FEATURES`, so a
  default-using module passes the backend's feature check.
- **Scope of `default`.** Only a `Required` parameter emits a default; a `Rest`
  (`...rest`) parameter has none, and a `KwRest` (`**opts`) keeps its v0 object
  fallback with no default surface form.
- **Version.** Minor bump `0.1.x → 0.2.0` (Cargo manifest); the changelog
  realigns onto the manifest version here.

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
