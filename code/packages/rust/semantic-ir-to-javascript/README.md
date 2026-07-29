# semantic-ir-to-javascript

The **JavaScript backend** for the narrow-waist Semantic IR (SIR18) —
the fifth target after TypeScript, Rust, Python, and Go.

It consumes a [`semantic_ir::Module`](../semantic-ir) and emits a
single **self-contained** `.js` file that runs directly under Node.js:

```sh
node out.js
```

No `npm install`, no `require()`, no `import` — the runtime helpers are
pasted inline at the top of every artifact.

## Where it sits in the stack

```text
  frontend            narrow waist            backend (this crate)
┌───────────┐      ┌────────────────┐      ┌──────────────────────┐
│ Twig / …  │ ───▶ │  semantic_ir   │ ───▶ │ semantic-ir-to-      │ ──▶ out.js
│ (source)  │      │  ::Module      │      │ javascript           │
└───────────┘      └────────────────┘      └──────────────────────┘
```

Any frontend that lowers to `semantic_ir::Module` (e.g.
`twig-to-semantic-ir`) can target JavaScript through this crate. The IR
is the contract; the backend never sees source syntax.

## Pipeline

`compile(&module)` runs four steps and returns a
[`semantic_ir::Artifact`](../semantic-ir) (`filename`, `source`,
`metadata`):

1. **Validate** — `semantic_ir::validate`. Structural errors block
   lowering.
2. **Capability check** — every declared `Feature` must be in
   `accepts_features()`; every intrinsic must be whitelisted (none are).
3. **Reject tail-calls** — V8 does not reliably tail-call optimise.
4. **Lower** — walk the IR, emit JavaScript (see `src/emit.rs`).

```rust
use semantic_ir_to_javascript::compile;

let module = twig_to_semantic_ir::compile_source(
    "(define (add a b) (+ a b))\n(print (add 1 2))",
    "demo",
)?;
let artifact = compile(&module)?;       // artifact.source is runnable JS
std::fs::write("demo.js", artifact.source)?;
```

Or via the `Backend` trait:

```rust
use semantic_ir::Backend;
let artifact = semantic_ir_to_javascript::JavaScriptBackend::new().compile(&module)?;
```

## Capability declaration

This backend accepts the **v0 feature set plus all of SIR16 / v1** — the
surface the emitter lowers today. JavaScript supports every SIR16 feature
natively (arrays, `Map`, `while`/`for`, reassignable `let`), so each
lowering is direct.

| Accepted (v0 + SIR16 + E1 + O3 + MX4) | Rejected (deferred / unsupported)  |
|---------------------------------|------------------------------------------|
| `Closures`                      | `StringInterpolation` (`StrConcat`, SIR18) |
| `Pairs`                         | `SingletonClassDef` OOP dispatch         |
| `Symbols`                       | `TailCalls` (V8 has no reliable TCO)     |
| `Strings`                       | `Intrinsics` (empty whitelist)           |
| `DynamicTyping`                 |                                          |
| `OptionalTypeAnnotations`       |                                          |
| `MutualRecursion`               |                                          |
| `Globals`                       |                                          |
| `Floats` (SIR16)                |                                          |
| `ShortCircuit` (SIR16)          |                                          |
| `Sequences` (SIR16)             |                                          |
| `Maps` (SIR16)                  |                                          |
| `MutableBindings` (SIR16)       |                                          |
| `Loops` (SIR16)                 |                                          |
| `DefaultParams` (P2d)           |                                          |
| `KeywordParams` (KW4)           |                                          |
| `Exceptions` (E1, SIR17)        |                                          |
| `Classes` (E2 ancestry + O3 OOP)|                                          |
| `InstanceVars` (O3)             |                                          |
| `ClassVars` (O3)                |                                          |
| `Modules` (MX4 mixins)          |                                          |
| `Constants`                     |                                          |
| `SymbolicExpr` (SIR23)          |                                          |
| `PatternMatching` (SIR23)       |                                          |
| `Rationals` (shared w/ SIR22)   |                                          |
| `NDArrays` (SIR22 base cut)     |                                          |
| `MatrixOps` (SIR22 base cut)    |                                          |
| `ArrayColumnMajor` (SIR22)      |                                          |

`accepts_intrinsics()` is empty. The accept-set is deliberately matched
to what `emit` handles, so a module using a deferred node is turned away
*before* lowering rather than mis-compiled — and every accepted feature
has a real emit arm (the residual `panic!` guards cover only the
still-deferred SIR18 nodes — `SingletonClassDef` OOP dispatch and string
interpolation — plus the still-unimplemented SIR22 "APL addendum" nodes,
one caveat below).

**One caveat on `NDArrays`/`MatrixOps`/`ArrayColumnMajor`**: the SIR22
*base cut* (`ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/`Transpose`/
`IndexGet`/`IndexSet`) has real codegen against an inlined `__Sir.Array`
runtime (a plain-JS port of the published `sir-runtime-array` package —
see `runtime.rs`). The SIR22 *addendum* nodes (`Reduce`/`Scan`/
`OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/
`Catenate`) share these same three features and remain deferred, so this
backend adds a dedicated tree-walk check inside `compile()` (beyond the
ordinary feature-flag capability check) that cleanly rejects a module
using any of the nine rather than let it slip through and panic in
`emit` — see `find_unimplemented_sir22_addendum_node` in `lib.rs`.

`Classes` now covers **full user-defined-class OOP (O3)**: a `ClassDef`
supplies its `superclass` *ancestry edge* (so `raise MyErr; rescue
StandardError` matches, and so method resolution walks the hierarchy), and
the O2 Ruby frontend's OOP builtins (`__new__`, `__super__`,
`__def_method__`, `__def_class_method__`, `__self__`) lower to the inlined
`__Sir` OOP runtime — instantiation, method dispatch, `super`, `self`, and
`@ivar`/`@@cvar` access all execute end-to-end.

### User-defined-class OOP (O3)

Method bodies are hoisted by the frontend to top-level functions and
registered with `__Sir.defMethod("Class", "name", <closure>)`.  Dispatch,
instantiation, and `super` all key on the **class/method *name string***
through a `Map` — never `recv[name]`, `eval`, or `new Function` — so a
class or method named `constructor` / `__proto__` is inert data (a Map
miss floors to `NoMethodError`), closing the same RCE / prototype-pollution
door as the collection-method allowlist.

| SIR (from O2 frontend)          | JavaScript emitted                          |
|---------------------------------|---------------------------------------------|
| `Dog.new("Rex")`                | `__Sir.callNew("Dog", "Rex")`               |
| `super(4)` in `Cat#initialize`  | `__Sir.callSuper("initialize", "Cat", 4)`   |
| `def speak; …; end`             | `__Sir.defMethod("Dog", "speak", <closure>)`|
| `def self.count; …; end`        | `__Sir.defClassMethod("Dog", "count", …)`   |
| `self`                          | `__Sir.currentSelf()`                       |
| `recv.meth(a)` (`SirInstance`)  | `__Sir.callMethod(recv, "meth", a)`         |
| `@name` read / write            | `__Sir.ivarGet("@name")` / `ivarSet(…)`     |
| `@@count` read / write          | `__Sir.cvarGet("@@count")` / `cvarSet(…)`   |

`self` is a dynamic stack: a method pushes its receiver before running and
pops it in a `finally`, so `@ivar` reads resolve against the live receiver
and an exception thrown mid-method still unwinds cleanly.  A `SirInstance`
receiver dispatches to the user method table; every **other** receiver
(array, string, …) falls through to the unchanged built-in / collection
path, so collection methods and exceptions are not regressed.

### Mixins — `include` / `extend` (MX4)

A `module M … end` registers its `def`s into the **same** `methodTable` a
class uses (keyed by the module name), and the two mixin directives lower to
one builtin each; dispatch then follows Ruby's **Method Resolution Order**.

| SIR (from MX1 frontend)         | JavaScript emitted                          |
|---------------------------------|---------------------------------------------|
| `include Greet` in `class C`    | `__Sir.includeModule("C", "Greet")`         |
| `extend Counter` in `class C`   | `__Sir.extendModule("C", "Counter")`        |
| `Widget.tally(1)` (const recv)  | `__Sir.callClassMethod("Widget", "tally", 1)` |

`__Sir.resolveMethod` walks the MRO: **class → its included modules
most-recent-first (each expanded depth-first through its own `include`s) →
superclass → …**  A class-defined method **shadows** a mixed-in module
method, a **diamond** include resolves the shared module **once**, and
`extend` promotes a module's instance methods to **class methods** (found by
`callClassMethod`).  The per-owner `includedModules` / `extendedModules` are
real `Map`s keyed by *name strings* holding module *name strings* — the same
explicit-table, no-reflection bar as the method tables — and a single shared
`seen` set makes a self-including module or cyclic hierarchy **terminate**
(`NoMethodError`) rather than loop.

### SIR16 lowering at a glance

| SIR16 node                       | JavaScript emitted                              |
|----------------------------------|-------------------------------------------------|
| `FloatLit`                       | native `number` (`NaN`/`Infinity` spelled out)  |
| `LogicalAnd` / `LogicalOr`       | `((__l) => __Sir.truthy(__l) ? … : …)(lhs)`     |
| `SeqLit` / `SeqIndex` / `SeqLen` | `[…]` / `(a)[i]` / `(a).length`                 |
| `SeqSet`                         | `(a)[i] = v;`                                    |
| `MapLit` / `MapGet`              | `new Map([[k, v]])` / `((m).get(k) ?? null)`    |
| `MapSet`                         | `(m).set(k, v);`                                |
| `Assign`                         | `name = value;` (`let` bindings are mutable)    |
| `While`                          | `while (__Sir.truthy(cond)) { … }`              |
| `ForRange`                       | direction-aware C-style `for` (bounds once)     |
| `ForEach`                        | `for (let x of iter) { … }`                     |

### Exceptions (E1) — `try`/`catch`/`raise`

`Stmt::TryCatch` lowers to a **native** `try`/`catch`/`finally`. A native
`catch` binds one variable and catches everything, but Ruby has an ordered
list of *typed* `rescue` clauses, so the catch body is an if/else-if chain
that asks the runtime `__Sir.rescueMatches(__exc, [...classNames])` for
each clause in source order — running the first match, binding `=> e` when
present, and re-`throw`ing the original exception if none match:

```js
try {
  __Sir.raiseError("ArgumentError", "x");
} catch (__exc) {
  if (__Sir.rescueMatches(__exc, ["StandardError"])) {
    const e = __exc;
    __Sir.print("caught");
  } else {
    throw __exc;
  }
}
```

An empty `exception_types` is a bare `rescue` (catch-all → `rescueMatches`
returns `true`); an `ensure` becomes a `finally`. The `raise` builtin
lowers to `__Sir.raiseError(cls, msg)` (`raise Foo, "m"` →
`raiseError("Foo", "m")`; a non-class first arg → an implicit
`RuntimeError`; bare `raise` → a generic re-raise).

**User-class ancestry (E2 half).** So that `rescue StandardError` catches a
`raise MyErr` when `class MyErr < StandardError`, the emitter collects
every `ClassDef` inheritance edge and emits one `__Sir.registerAncestry({
"MyErr": "StandardError" })` at program init, merging the pairs into the
runtime's ancestry table. Dispatch is a pure string-map walk — never
`eval` or reflection (see the runtime section).

### Default parameters (P2d)

A `Param` carrying `default: Some(expr)` lowers to a **native JS default
parameter** — `function f(a, b = <expr>) { … }` — because JavaScript's
default-parameter semantics are exactly SIR's:

- **Call-time**: the default runs each call, only when the argument is
  omitted (not a compile-time constant baked in once).
- **Param scope**: a later default may reference an earlier parameter by
  name. SIR emits such a reference as `VarRef { scope: Param }` → a bare
  identifier, which is in scope left-to-right in a JS parameter list.

There is no call-site padding: the SIR validator allows a caller to omit
trailing defaulted args (arity ≥ `required_param_count`), so a
`DirectCall` emits **only the args present** and the native defaults fill
the omitted trailing params. For example, `f(a, b = a + 1)` emits
`function f(a, b = (a + 1)) { … }`; `f(5)` calls it with one arg and
`b` binds to `6` at call time. (`IndirectCall` / closure defaults are
unchanged / deferred.)

## Runtime shape (inlined `__Sir`)

Every artifact pastes one fixed IIFE near the top:

```js
const __Sir = (() => {
  "use strict";
  class Sym { /* interned name */ }
  class Pair { /* car / cdr */ }
  class Closure { /* wraps a JS fn */ }
  function intern(name) { /* one Sym per name */ }
  function applyClosure(c, args) { /* invoke a Closure */ }
  function truthy(v) { /* only false / null are falsy */ }
  function format(v) { /* Lisp-ish display for print */ }
  const builtins = { "+": …, "cons": …, "range": …, "print": … };
  return { Sym, Pair, Closure, intern, applyClosure, truthy,
           format, print, builtins, builtinClosure, callBuiltin };
})();
```

The classic JavaScript module pattern: the classes, symbol table, and
helpers are private to the arrow body; only the returned object escapes,
bound to the single global `__Sir`. This mirrors the TypeScript
backend's `namespace __Sir { … }` — minus the type annotations and the
external package import.

### Value model

| SIR concept   | JavaScript representation                  |
|---------------|--------------------------------------------|
| `Int`/`Float` | native `number`                            |
| `Bool`        | native `boolean`                           |
| `Nil`         | `null`                                      |
| `Symbol`      | `__Sir.Sym` instance (interned `.name`)    |
| `Str`         | native `string`                            |
| `Pair`        | `__Sir.Pair` instance (`car`/`cdr`)        |
| `Closure`     | `__Sir.Closure` instance wrapping a JS fn  |

### Builtin specialisation

For idiomatic output, common builtins emit native JavaScript instead of
a runtime call:

- `+ *` (2 args) → `__Sir.plus(a, b)` / `__Sir.times(a, b)` — **polymorphic**
  (see below); native infix would be wrong for the collection arms.
- `- / %` (2 args) → native infix `(a - b)`, … (numeric-only in the SIR contract)
- `= != < > <= >=` (2 args) → `(a === b)`, `(a !== b)`, …
- `not` (1 arg) → `(!__Sir.truthy(a))`; `neg` → `(-(a))`
- `len` (1 arg) → `(a).length` (arrays and strings)
- `print` (1 arg) → `__Sir.print(a)` (consistent stringification)

A **variadic** operator (`(+ 1 2 3)` — more than two args) and any
unrecognised builtin fall back to `__Sir.callBuiltin("+", […])`, so a new
builtin runs without a backend change.

### Polymorphic `+` / `*` (Ruby operator overloading)

Ruby overloads `+` and `*` by the receiver's runtime type, and all of these
lower to the same SIR `+`/`*` builtins, so the JavaScript backend dispatches at
**runtime** — via the inlined `__Sir.plus` / `__Sir.times` helpers — on the
**first operand's** type:

| Expr           | Result       | Arm                                    |
|----------------|--------------|----------------------------------------|
| `1 + 2`        | `3`          | numeric fold (unchanged)               |
| `"a" + "b"`    | `"ab"`       | String concat                          |
| `[1] + [2]`    | `[1, 2]`     | Array concat (a **new** array)         |
| `"ab" * 3`     | `"ababab"`   | String repeat                          |
| `[0] * 3`      | `[0, 0, 0]`  | Array repeat (a **new** array)         |
| `[1, 2] * ", "`| `"1, 2"`     | Array join (elements via `format`)     |

Dispatch is a runtime **tag** test (`typeof x === "string"` /
`Array.isArray(x)`), never reflection or `eval` on a source-derived name. The
String/Array arms sit strictly ahead of the numeric path, so every existing
numeric program is byte-for-byte unchanged. This also fixes the native-JS
`[1] + [2]` bug, which would otherwise coerce to the string `"1,2"`.

**Security.** The two repeat arms multiply a length by a program-controlled
count. A shared `repeatCount` guard clamps a non-finite / non-integer /
non-positive count to an empty result and rejects an oversized product
(`len * count > Number.MAX_SAFE_INTEGER`) with a Ruby-shaped
`ArgumentError: argument too big` **before** allocating — so a hostile count
can neither OOM the process nor throw a raw `RangeError` (CWE-1284 / CWE-400).

### Exception helpers (E1)

The inlined `__Sir` also carries the exception runtime — a plain-JS port
of the published `@coding-adventures/sir-runtime-exceptions` package, so
the artifact stays self-contained:

- `SirError` — a real `Error` subclass tagged with its Ruby class name in
  `.sirClass`; what `raise` throws.
- `raiseError(cls, msg)` — throws a `SirError`; the target of a lowered
  `raise`.
- `rescueMatches(exc, classNames)` — the per-clause dispatcher: `true` for
  a bare `rescue` (empty list) or `Exception` (universal root), otherwise
  a superclass-chain walk over the ancestry table.
- `registerAncestry(map)` — merges user `{ child: "Super" }` edges into the
  ancestry lookup (called once at program init for the module's inheriting
  classes).

The built-in Ruby ancestry (`RuntimeError`/`ArgumentError`/… →
`StandardError` → `Exception`) is baked in, so `rescue StandardError`
catches the everyday subclasses out of the box.

**Security.** Ancestry resolution is a pure `ancestry[cur]` string-map
walk — never `eval` or dynamic property reflection; class and method names
are treated strictly as data. The mutable map is `Object.create(null)`
(prototype-less), so a user class named `constructor`/`__proto__` cannot
poison the lookup, and a cyclic user map terminates via a `seen` guard.

### Symbolic expressions + pattern/rewrite (SIR23)

The inlined `__Sir` also carries `Symbolic` — a plain-JS port of the
published `@coding-adventures/symbolic-ir` (term-tree type + constructors),
`@coding-adventures/cas-pattern-matching` (the structural matcher/
substitution algorithm), and `@coding-adventures/sir-runtime-symbolic`
(`replaceAll`/`replaceRepeated`/`unwrap`) TypeScript packages, so the
artifact stays self-contained:

| SIR                               | JavaScript emitted                              |
|------------------------------------|-------------------------------------------------|
| `SymSymbol("f")`                   | `__Sir.Symbolic.sym("f")`                       |
| `SymRational(1, 3)`                 | `__Sir.Symbolic.rational(1, 3)`                 |
| `SymApply(f, [x])`                  | `__Sir.Symbolic.apply(sym(f), [x])`             |
| `SymPatternBlank(None)`             | `__Sir.Symbolic.blank()`                        |
| `SymPatternBlank(Some(Integer))`    | `__Sir.Symbolic.blankTyped("Integer")`          |
| `SymPatternNamed("x", pat)`         | `__Sir.Symbolic.named("x", pat)`                |
| `SymRule(lhs, rhs, delayed: false)` | `__Sir.Symbolic.rule(lhs, rhs)`                 |
| `SymRule(lhs, rhs, delayed: true)`  | `__Sir.Symbolic.ruleDelayed(lhs, rhs)`          |
| `SymReplaceAll(e, r, repeated: false)` | `unwrap(replaceAll(e, r))`                   |
| `SymReplaceAll(e, r, repeated: true)`  | `unwrap(replaceRepeated(e, r))`              |

A term is a plain, frozen `{ kind, … }` object (`"symbol"` / `"integer"` /
`"rational"` / `"float"` / `"string"` / `"apply"`) — never a class instance,
so it never collides with `Sym`/`Pair`/`Closure`/`SirInstance` above. A bare
`IntLit`/`FloatLit`/`StrLit` operand is wrapped through `int`/`numberNode`/
`stringNode` (`emit_sym_operand`) before it can sit inside a term tree, since
a raw JS number/string is never a valid term.

**Deliberate divergence from the TypeScript sibling:** terms use plain JS
`number` for `integer`/`rational` values rather than `bigint` — matching how
every other numeric value in this backend already works (`IntLit` emits a
bare JS number literal; there is no `bigint` anywhere else in this runtime).

**Security.** `matchPattern`/`substitute`/`applyRule` recurse only as deep as
a single rule's own (author-written) pattern/RHS shape, never the target
expression's depth, so they need no cap. `replaceAll`/`replaceRepeated` walk
the *entire* target expression, which ordinary program data can build
unboundedly deep — `MAX_TERM_DEPTH = 512` caps that walk (CWE-674). A rule
firing inside `replaceRepeated` loops at the *same* call frame rather than
recursing on the fresh replacement, so a caller-supplied `maxIterations`
(default 100) bounds only CPU time, never native stack depth — carrying
forward the fix the TypeScript sibling package's own `/security-review`
found.

`print`/`puts` render a Symbolic term via `Symbolic.toDisplayString`
(`f(x, 1/3)`-style — Derive-sourced modules get a different, own-language
convention instead; see "Derive's own SIR23 display convention" below),
reached from `formatSeen` by checking for a plain object carrying a
`.kind` tag.

**`Symbolic.evalTerm` (SIR23 addendum, item 1 of 4 — arithmetic/
comparison/logic folding only).** Every top-level SIR23 statement
(`emit.rs`'s `Stmt::ExprStmt` arm, for a bare `SymApply`/`SymSymbol`/
`SymRational`, or the same shape as `print`'s sole argument) is wrapped
in `__Sir.Symbolic.unwrap(__Sir.Symbolic.evalTerm(...))` — a direct JS
port of `symbolic-vm`'s `VM::eval`/`eval_apply` per-head dispatch (see
`code/specs/SIR23-symbolic-pattern-semantic-ir.md`'s own "Addendum" for
the full design). `Expr::SymApply`'s own codegen is unchanged (still a
bare, unevaluated `apply(head, args)`); `evalTerm` recurses into
`head`/args itself, so wrapping happens exactly once per statement, not
once per nested `SymApply`.

This item's scope is intentionally narrow:

- **Wired up:** arithmetic (`Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`/`Inv`/
  `Abs`, with exact-rational results — `1/3` stays `1/3`, `10/2` folds
  to the integer `5`), comparison (`Equal`/`NotEqual`/`Less`/`Greater`/
  `LessEqual`/`GreaterEqual`, folding to the `True`/`False` **symbol**,
  never a JS boolean), logic (`And`/`Or`/`Not`, N-ARY).
- **Declared but inert:** `Assign`/`Define`/`If` (`HELD_HEADS`) have no
  handler yet, so they stay byte-for-byte the same unevaluated data
  today's codegen already produces — no environment, no user-function
  dispatch, no branching. That is item 2's job.
- **Not wired up at all:** calculus/elementary functions (`Sin`, `D`,
  `Integrate`, … — item 3; held-form execution — `Assign`/`Define`/`If`
  dispatch — is item 2, above). `List` needs no handler ever:
  applicative-order argument evaluation alone folds `List(Add(1,1),
  Mul(2,3))` into `List(2, 6)` for free.
- `MAX_EVAL_DEPTH = 2000` is `evalTerm`'s own empirically-measured
  recursion-depth cap (CWE-674) — deliberately not a reuse of
  `MAX_TERM_DEPTH` above, which guards a different function
  (`replaceAll`/`replaceRepeated`'s tree walk) with a different
  per-frame cost. See `runtime.rs`'s own doc comment on the constant for
  the full measurement writeup, and `tests/sir23_eval_depth_guard.rs`
  for the executable proof.

**Derive's own SIR23 display convention (SIR23 addendum, item 4 of 4 —
display only, scoped to Derive).** `Symbolic.toDisplayString` branches,
at its own top, on a fourth `SIR_DISPLAY_*` flag — `SIR_DISPLAY_DERIVE`,
computed from `m.metadata.source_language == "derive"` exactly like the
existing `SIR_DISPLAY_RUBY`/`SIR_DISPLAY_APL_HIGH_MINUS`/
`SIR_DISPLAY_J_UNDERSCORE` flags (see the "Array/matrix domain" section
below for those) — to a separate function family (`deriveRender`/
`derivePrintAt`/`deriveRenderApply`/`deriveRenderList`) that is a direct,
byte-for-byte JS port of `derive-runtime::printer::print_derive`'s own
precedence-based renderer, rather than the generic `head(args, …)` form
every other source language still gets:

- Infix `Add`/`Sub`/`Mul`/`Div` and comparisons `Equal`/`Less`/`Greater`/
  `LessEqual`/`GreaterEqual`, n-ary `And`/`Or`, prefix `Neg`/`Not`, and
  right-associative `Pow` (`a^b^c`), each parenthesised exactly where
  `printer.rs`'s own 9-level precedence ladder says a looser child needs
  it.
- Derive's own `List` bracket convention (D-5): a flat vector prints
  `[a, b, c]`; a "list of lists" prints as a `;`-row-separated matrix,
  `[a, b; c, d]`.
- Case-bridging a fixed table of builtin heads back to Derive's own
  UPPERCASE surface spelling (`D` → `"DIF"`, `Sin` → `"SIN"`, …); any
  other head (a user-defined function) renders as-typed.
- `True`/`False` and `Assign`/`Define` need **no** special-casing: the
  former already renders identically under both conventions (a bare
  `Symbol` term's verbatim name); the latter never reach the display path
  at all once items 2/3 land (their handlers return the bound value, not
  an `Assign(...)`/`Define(...)` term) — see `runtime.rs`'s own
  `SIR_DISPLAY_DERIVE` doc comment for the full writeup.

This item has no code dependency on items 2/3 (it touches only
`toDisplayString`, a different function than `evalTerm`/`evalApply`) and
is scoped to Derive only — `derive-to-semantic-ir` is the only Stream B
frontend with an oracle corpus proving it today; Wolfram/Macsyma/Reduce/
Maple's own conventions are separate future work following the identical
recipe (one more `SIR_DISPLAY_*` flag + printer port).

### Array/matrix domain (SIR22 base cut)

The inlined `__Sir` also carries `Array` — a plain-JS port of the
published `@coding-adventures/sir-runtime-array` TypeScript package, so
the artifact stays self-contained:

| SIR                                    | JavaScript emitted                                                       |
|------------------------------------------|---------------------------------------------------------------------------|
| `ArrayLit([[1, 2], [3, 4]])`             | `__Sir.Array.fromRows([[1, 2], [3, 4]])`                                 |
| `Range(1, Some(2), 10)`                  | `__Sir.Array.range(1, 10, 2)` — note the argument ORDER: `stop` before `step` |
| `MatMul(a, b)`                           | `__Sir.Array.matmul(a, b)`                                               |
| `ElementwiseOp(Mul, a, b)`               | `__Sir.Array.elementwise("Mul", a, b)`                                   |
| `Transpose(a, conjugate: true)`         | `__Sir.Array.transpose(a, true)`                                         |
| `IndexGet(a, [Scalar(0), Whole])`        | `__Sir.Array.indexGet(a, [{ kind: "scalar", value: 0 }, { kind: "whole" }])` |
| `IndexSet(a, [Scalar(0)], v)` (a `Stmt`) | `__Sir.Array.indexSet(a, [{ kind: "scalar", value: 0 }], v);`            |

`NDArray` is `{ shape: number[], data: Float64Array }` — dense,
COLUMN-MAJOR storage (Fortran/MATLAB order), mirroring
`array_runtime::value::Array` field-for-field. `elementwise` coerces a
bare JS `number` operand into a scalar `NDArray` (`toArrayValue`) because
`matlab-to-semantic-ir`'s lowerer emits a mixed number/`NDArray` operand
pair whenever exactly one side of `.* ./ .\`/`* /` is provably scalar
(`A .* 2` passes `2` through unwrapped, not as an `ArrayLit`).

**Not implemented**: the SIR22 "APL addendum" nodes (`Reduce`/`Scan`/
`OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/
`Catenate`) — see the capability table's caveat above for why a module
using one of these fails cleanly rather than through the ordinary
feature-flag check.

**Security.** Every factory that computes an output size from
caller-supplied numbers (`matmul`'s `[m, n]`, `indexGet`'s two
independently-bounded row/column selections, `range`'s element count, ...)
validates via `checkedShapeSize`/an explicit `MAX_ELEMENTS` (2^26) check
*before* allocating a `Float64Array`, matching `matlab-runtime`'s own
`MAX_RANGE` bound — an unbounded or malformed shape fails with a
catchable `Error`, not an uncaught `RangeError` or a stalled huge
allocation.

## Output format

- 2-space indentation, semicolons always (no ASI reliance).
- `"use strict";` at the top (after the banner comment).
- Banner comment naming the source module and language.
- Trailing newline at end of file.
- **Deterministic** — the same module always produces byte-identical
  output (the runtime is fixed text; no iteration over unordered maps).

A `Block` in **function-body** position emits a flat
`{ stmts…; return value; }`; a `Block` in **expression** position emits
an IIFE `(() => { stmts…; return value; })()` so its `let` bindings stay
private. The module footer calls `_init()` (if present) then `main()`.

## Identifier sanitisation

JavaScript identifiers match `[A-Za-z_$][A-Za-z0-9_$]*` and must not be
reserved words. SIR names can carry `?`, `!`, `-`, `+`, etc., so
`sanitize_ident` rewrites anything that does not fit:

| input        | output      | rule                              |
|--------------|-------------|-----------------------------------|
| `hello`      | `hello`     | already valid → unchanged         |
| `class`      | `_$class`   | reserved word → `_$` prefix       |
| `null?`      | `_$null_3f` | invalid char → `_$` + hex-encoded |
| `""` (empty) | `_$empty`   | empty → sentinel                  |

The `_$` prefix guarantees a legal leading character and avoids
collisions between distinct invalid inputs (each non-`[A-Za-z0-9_$]`
character hex-encodes to `_<codepoint>`).

## Tests

```sh
cargo test -p semantic-ir-to-javascript
```

- Unit tests for `sanitize_ident`, string quoting, float formatting, and
  every emit arm.
- A determinism test (two compilations are byte-identical).
- End-to-end integration tests (`tests/run_with_node.rs`) that emit
  JavaScript to a unique temp file, **run it with `node`**, and assert
  stdout. Twig-lowered programs cover the v0 core (add → `3`, factorial →
  `120`, closure-adder → `8`); hand-built SIR16 modules cover float
  arithmetic promotion (`3.5`), short-circuit (rhs not evaluated), seq
  build/index/len/set, map build/get/set (missing key → nil), a `while`
  counter, a for-range accumulator (and a descending step), for-each, and
  mutable reassignment (`42`). When `node` is not on PATH the execution is
  skipped and the syntactic checks still run.
- `tests/sir23_symbolic.rs`: real `node`-execution tests for the SIR23
  symbolic domain — `replaceRepeated` reduces `Add(Add(z, 0), 0)` to the bare
  symbol `z` via `x_ + 0 -> x_`, `replaceAll`'s single-pass (no-retry)
  contract, and a head-typed blank (`x_Integer`) matching selectively.
- `tests/sir22_array.rs`: real `node`-execution tests for the SIR22
  array/matrix base cut — matrix multiplication, the bare-scalar-operand
  `elementwise` coercion fix, transpose, MATLAB-colon range semantics,
  in-place `indexSet` (including whole-column broadcast), and a
  non-conformable-matmul clean-error-exit case.
