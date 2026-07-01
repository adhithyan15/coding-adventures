# Changelog

All notable changes to `javascript-to-semantic-ir` are documented here.

## Unreleased

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## 0.7.0

**Member-method calls → `__method__` dispatch (C3 — collection methods).**
The frontend previously *deferred* every member-method call (`arr.map(fn)`,
`arr.push(x)`, `str.toUpperCase()`, …) to a positioned error, special-casing
only `console.log`.  It now lowers them to the SIR method-dispatch
convention — `BuiltinCall("__method__", [receiver, StrLit("method"),
args…])` — exactly as the Ruby frontend does.  This is the JavaScript arm of
the `sir-collection-methods` cascade; collection code (`.map`/`.filter`/
`.push`/`.reduce`/…) now lowers, validates, and executes end-to-end through
the JS backend.

### Added

- **Member-method-call lowering.**  A dotted call `recv.method(args…)` (other
  than `console.log`) lowers to `BuiltinCall("__method__", [recv,
  StrLit("method"), args…])`: the receiver at `args[0]`, the method name
  always a `StrLit` at `args[1]`, the call arguments following.  The synthetic
  method-name `StrLit` declares `Feature::Strings` (no dedicated
  `MethodDispatch` feature in v0 — `Strings` is what the validator observes).
- **Chained method calls.**  A flat call chain `xs.filter(f).map(g)` folds one
  step at a time — each `.method(args)` wraps the accumulated expression as
  the receiver of the next `__method__` dispatch.
- **Callback arguments.**  An arrow / function argument (`arr.map(x => x*2)`)
  reuses the existing closure lowering, arriving as a `MakeClosure` in the
  dispatch's argument list (the JS higher-order convention — no trailing-block
  syntax).
- Lowering-assertion tests (`push`/`pop`/`map`/`filter`/`toUpperCase`,
  chaining, `console.log` non-regression, deferred computed calls) plus node
  execution-proofs (`xs.push(4); xs.length` → `4`; `.map`+`.reduce` → `12`;
  `.filter`+`.includes` → `#t`).

### Changed

- `console.log` keeps its dedicated `print` lowering (not routed through
  `__method__`); a *computed* member call (`obj[expr](…)`) stays deferred.
- Compile-compat stub arm for the new core `semantic-ir` variant
  `Expr::KeywordArg` (KW1) — the effect-inference pass recurses into the
  keyword arg's inner `value` so purity analysis stays
  conservative-correct. Real keyword-argument lowering is pending KW2–KW8.

## 0.6.0

**Default parameters (P9).**  The frontend now *produces* default
parameters: a defaulted formal `name = <expr>` lowers to a
`Param { default: Some(<lowered expr>) }`.  Previously M4 deferred any
parameter with an initializer; this wires defaults all the way through, for
both `function` declarations and arrow functions.  The core IR and all five
backends already supported `Param.default`; this completes the JavaScript
front of that work.

### Added

- **Default-parameter lowering.**  A `formal_parameter` with the CST shape
  `[Name, "=", <assignment_expression>]` lowers to a `Param` whose `default`
  is the lowered initializer.  A plain `[Name]` keeps `default: None`.  This
  applies to `function f(a = 1) { … }` and to arrow functions
  `(a = 1) => …`.  Observes `Feature::DefaultParams` (and any feature the
  default expression itself uses, e.g. `Strings` for `a = "x"`).
- **Call-time, param-scope semantics.**  JS defaults are evaluated at the
  call site and may reference *earlier* parameters
  (`function f(a, b = a + 1)`).  We lower each default **inside the function
  frame** (after the param scope is pushed), so a reference to an earlier
  param resolves as a `Scope::Param` `VarRef` — exactly the SIR
  `Param.default` model the validator and every backend already assume.
- **Partial calls.**  A call that omits a defaulted argument (`f(5)` against
  `function f(a, b = a + 1)`) lowers to a `DirectCall` carrying *only the
  present arguments*; the omitted parameter is filled by its default at the
  call site.  The SIR validator permits such partial calls when the trailing
  params have defaults.
- **e2e (node).**  New `default_params_run_in_node` test lowers
  `function f(a, b = a + 1) { … } console.log(f(5)); console.log(f(5, 10));`
  → SIR → JavaScript (via the `semantic-ir-to-javascript` dev-dep) → `node`,
  asserting `6` then `10` — proving call-time evaluation and param-scope
  resolution end to end.

### Changed

- **Block builder keeps effectful superseded tail expressions.**  When a
  bare-expression statement is superseded as a block's tail value by a later
  expression, it is now flushed as an `ExprStmt` **iff it may have a side
  effect** (a `print`, a call, …) rather than always being dropped as pure.
  This fixes a latent bug where two consecutive `console.log(...)`
  statements at the same block level lost all but the last.  A new
  conservative `expr_may_have_effects` helper drives the decision (literals,
  var-refs, and pure operators over pure operands are still dropped).

### Rejected (still deferred)

- Rest `...args` and destructuring (`{a}` / `[a]`) parameters remain
  deferred — defaults are the only previously-deferred param form now
  supported.  The deferral message was updated accordingly.

### Safety

- The default initializer is an ordinary expression, lowered through the
  existing `MAX_EXPR_DEPTH`-bounded `lower_expression`, so a pathologically
  deep default becomes a positioned error rather than a native stack
  overflow (CWE-674).  No new unbounded CST walk was introduced.

## 0.5.0

SIR19 milestone **M5 (collections: arrays & objects)** — builds on M4's
functions/closures.  Adds array literals, object literals, member / dot /
subscript access (reads), and indexed / property assignment (writes).
This is the last big data-manipulation milestone for the JS frontend.

### Added

- **Array literals → `SeqLit`.**  `[e0, e1, …]` (CST `array_literal[
  LBracket, element_list, RBracket ]`) lowers each element through the
  depth-bounded expression lowerer.  `[]` is an empty `SeqLit`.  Observes
  `Feature::Sequences`.
- **Object literals → `MapLit`.**  `{ a: 1, "k": v }` (CST `object_literal`
  with `property_definition[ property_name, Colon, value ]` children) lowers
  to a `MapLit`.  Identifier keys (`a:`) and quoted keys (`"k":`) **both**
  become string map keys — the JS quoting distinction is syntactic.  `{}` is
  an empty `MapLit`.  Observes `Feature::Maps`.  A parenthesised grouping
  (`primary_expression[ (, expression, ) ]`) is now peeled, so `({a:1})` at
  statement start lowers (the `{`-as-block ambiguity is resolved by the
  source parens).
- **Member / subscript reads.**  `xs.length` → `SeqLen`; every other
  `obj.prop` (dot) → `MapGet` with a string key; `xs[i]` → `SeqIndex`;
  `obj["k"]` (string-literal subscript) → `MapGet`.  Member chains are
  **flat** in the CST (`grid[0][1]` is one `member_expression` with the
  accesses as a trailing token/node sequence), so we **fold the accesses
  left iteratively** — no per-segment CST recursion.
- **Indexed / property assignment (writes).**  `xs[i] = v` → `SeqSet`;
  `obj.prop = v` / `obj["k"] = v` → `MapSet`; chained `grid[0][1] = v`
  builds the receiver (`grid[0]`) via the read path then emits the `SeqSet`.

### The bracket-vs-dot disambiguation

The IR has both sequences and maps; JS `x[i]` is structurally ambiguous and
the IR is untyped at this layer.  Following the SIR19 collections table
(the authority), we resolve it **by access shape and key kind**, not by
value-type inference:

- dot `.prop` → `MapGet` (string key), with the single special case
  `.length` → `SeqLen`;
- bracket with a **string-literal** key (`obj["k"]`) → `MapGet` (a quoted
  subscript reads exactly like the equivalent dotted access);
- bracket with **any other** key (`xs[0]`, `xs[i]`, `xs[i+1]`) → `SeqIndex`.

Assignment targets mirror this exactly.  This default is spec-sanctioned
and documented in both the module docs and the spec's collections section.

### Recursion safety (CWE-674)

Every new CST walk is depth-bounded.  Array elements, object-property
values, member-chain receivers, and bracket indices are all lowered through
`lower_expression` at `depth + 1`, capped by `MAX_EXPR_DEPTH = 256`; the
element/property/access *iteration* is strictly non-recursive.  A
pathological `[[[[…]]]]`, `{a:{b:{c:…}}}`, or `p.p.p…` tower yields a
positioned `JsLowerError`, never a native stack overflow.  A regression test
(`deeply_nested_member_chain_is_rejected_without_crashing`) feeds a
600-deep member tower straight into `compile` and asserts a clean error.

### Deferred past M5

Spread (`[...xs]` / `{...o}`), array elisions (`[1, , 3]`), object shorthand
(`{x}`), computed keys (`{[e]: v}`), numeric keys, object methods /
getters / setters, `.length` *assignment* (a resize with no IR node), and
array methods (`.map`/`.push`/… — these need runtime-library support per the
project mandate), plus the existing M4 deferrals (classes, `this`/`new`,
generators, `async`, destructuring, default/rest params, template literals).

### Tests

- Per-form lowering tests (array/object literals incl. empty + nested, the
  SeqLen/SeqIndex/MapGet disambiguation, string-vs-numeric subscripts, dot
  chains, all four write forms, chained writes), each with a
  `semantic_ir::validate` round-trip.
- The CWE-674 deep-member-chain regression test.
- Two new `node` end-to-end goldens (gated on `node` availability):
  `array_sum` (build/index/mutate/sum an array via a counting `for` over
  `xs.length`) → `19`; `object_get_set` (build an object, get/set
  properties) → `155`.

## 0.4.0

SIR19 milestone **M4 (functions, calls, closures)** — builds on M3's
control flow.  Adds `function` declarations, arrow functions, tail
`return`, function calls, and closures (`MakeClosure` + free-variable
capture).

### Added

- **`function` declarations → `Function`.**  A two-pass design: pass 1
  walks the whole CST collecting **every** `function` name (top-level and
  nested) so a call resolves to a `DirectCall` regardless of source order
  — forward references and (mutual) recursion all work.  Pass 2 lowers
  `function f(params){body}` to a top-level `Function { name, params,
  captures: [], body }`.  Params are simple positional names; default /
  rest / destructuring params are deferred.
- **Tail-position `return`.**  The IR has no early-return node, so a
  function/closure body is a `Block` whose `value` is the returned
  expression.  A `return` is accepted **only** in tail position — the
  body's last statement, or (recursively) the last statement of a branch
  of a tail-position `if`.  This admits the natural guard-via-`if`/`else`
  recursion shape (`if (base) { return b; } else { return rec; }`) while
  rejecting a genuine early `return` (one followed by more statements)
  with a positioned "early return not supported in v0" `JsLowerError`.  A
  body with no `return` (or a bare `return;`) yields `value = NilLit`.
- **Arrow functions → `MakeClosure`.**  `(params) => expr` and
  `(params) => { stmts; return r; }` (and the bare `a => …` and `() => …`
  forms) lift to a gensym'd top-level `__lambda_<N>` `Function` plus an
  `Expr::MakeClosure` at the source position.  An expression-bodied arrow
  has the expression as the block value; a block-bodied arrow follows the
  same tail-`return` rule as a function.
- **Nested `function` declarations → lifted closures.**  A `function`
  nested inside another function is lifted to a top-level synthesised
  `Function` (keeping its source name) and bound locally to a
  `MakeClosure`, so it can be returned / called indirectly and referenced
  by name.
- **Free-variable capture (on-resolve).**  A closure body is lowered
  inside a fresh scope frame; each name that resolves to an *enclosing*
  frame's local/param/capture (and is not a param/local of the closure,
  nor a module function/global/builtin) becomes a `Capture` on the
  synthesised `Function` (resolved as `Scope::Capture` inside the body)
  and a `CaptureValue` on the `MakeClosure` (resolved in the enclosing
  scope).  Captures thread **transitively** through nested closures.
- **Calls → `DirectCall` / `IndirectCall` / `BuiltinCall`.**  `f(args)`
  dispatches on the callee: a known module `function` name → `DirectCall`;
  `console.log(x)` → `BuiltinCall("print", [x])` (with the `MayPrint`
  effect); any other identifier that resolves to a closure *value*
  (local / param / capture) → `IndirectCall` on that value.  Zero-arg and
  multi-arg calls both lower.  Method calls other than `console.log` and
  non-identifier callees are deferred.
- **Manifest.**  Arrows / nested functions and `IndirectCall` declare
  `Feature::Closures`; an untyped param anywhere declares
  `Feature::DynamicTyping`; a genuine ≥2-cycle in the static call graph
  declares `Feature::MutualRecursion` (the validator has no node that
  *observes* mutual recursion, so it is frontend-declared — and only when
  a real cycle exists, to avoid a spurious "declared but unused"
  warning).  Pure self-recursion is **not** mutual recursion.
- **Exports.**  Every user-visible top-level `function` is exported
  alongside `main` (a module-scoped `function f` in JS is the analog of a
  Python module-level `def`).
- **Recursion bound.**  Function/closure/`if` body nesting reuses the
  `MAX_STMT_DEPTH = 256` guard (and operand recursion stays bounded by
  `MAX_EXPR_DEPTH`), so a pathologically deep nest of functions, calls, or
  tail `if`s yields a positioned error rather than a stack-overflow abort.
  The two **pre-lowering recursive CST walks** added in M4 —
  `collect_function_names` (pass-1 name collection, run from `compile`
  before the guarded lowering) and `reject_returns` (early-return
  detection) — are likewise depth-bounded by `MAX_STMT_DEPTH`, closing a
  CWE-674 unbounded-recursion / stack-overflow vector reachable from the
  public `compile` on adversarially deep input.
- 31 new unit tests (86 total): function declaration shape, tail return,
  no-return / bare-return → nil, empty body, early-return rejection
  (top-level and inside a non-tail `if`), tail `if`/`else` folding to
  `Expr::If`, `DirectCall`, forward-reference call, `IndirectCall`,
  `console.log` → print, zero-arg call, expression / block / no-param /
  bare-identifier arrows, capture (single and transitive), nested-function
  lift + capture, self-recursion vs mutual-recursion manifest, param /
  unresolved-name resolution, deferred default-param and method-call,
  depth-bound regressions for both pre-lowering CST walks (deep towers
  yield a clean "too deep" error, no crash — including a public-`compile`
  integration test on a synthetic 600-deep tower), and
  a functions+closures validate round-trip.
- **End-to-end `node` execution tests** (`tests/e2e_node.rs`, gated on
  `node` availability): factorial → `120`, fibonacci → `55`,
  closure-adder → `8`, mutual-recursion `isEven(10)` → `#t`, each lowered
  JS → SIR, emitted back to JavaScript with the merged
  `semantic-ir-to-javascript` backend (a new dev-dependency), and
  **executed with `node`**.

### Changed

- Minor version bump `0.3.0 → 0.4.0` (additive, backward-compatible).
- The lowerer's single flat `declared_locals` set is replaced by a
  **scope-frame stack** (`FnScope` per function: params, captures, locals)
  so name resolution can distinguish `Local` / `Param` / `Capture` and
  discover free variables across function boundaries.  M1–M3 behaviour is
  preserved (`main` is the bottom frame; top-level bindings are its
  locals).
- Re-assignment now preserves the target's resolved scope (`Assign` to a
  `Param` / `Capture`, not only `Local`).

### Spec sync

- Matches SIR19 "Return statement", "Arrow functions vs `function`
  declarations", "Closures", and the function/call coverage rows.  One
  refinement over a literal reading of the spec's "trailing `return`":
  a `return` is also accepted as the tail of a branch of a *tail* `if`,
  which is what makes the guard-style recursive goldens (factorial /
  fibonacci) expressible without an early-return node.  The spec's
  "Manifest computation = same trigger map as SIR17" is honoured, with
  `MutualRecursion` declared only on a genuine call-graph cycle.

### Deferred

Still **out of scope after M4** and returning a clear positioned
`JsLowerError`:

- **M5 — collections:** array literals (`SeqLit`), indexing (`SeqIndex`),
  `.length` (`SeqLen`), object literals (`MapLit`), member / `[]` access
  (`MapGet`), and method calls other than `console.log`.
- **Classes / `this` / `new`**, generators, `async`/`await`, default /
  rest parameters, destructuring, spread, and template literals.
- **Early `return`** mid-function (non-tail) — rejected, awaiting a
  control-flow lift in a future version.
- The remaining within-M2/M3 gaps (compound assignment outside the
  loop-update position, member/index assignment targets, multi-binding
  declarations, uninitialised bindings, bitwise/shift/exponentiation/
  nullish operators, other prefix unaries, non-decimal numeric forms,
  `switch`/`try`/`do-while`/labeled/`break`/`continue`) — unchanged.

## 0.3.0

SIR19 milestone **M3 (control flow)** — builds on M2's variables and
operators.  Adds `if`/`else`, `while`, the canonical counting C-style
`for`, `for … of`, and bare `{ … }` blocks.

### Added

- **`if`/`else` → `Expr::If`.**  A JS `if` *statement* lowers to a
  `Stmt::ExprStmt` wrapping an `Expr::If` (the IR's conditional is an
  expression; there is no statement-level `if`).  The `then`/`else`
  branches are `Block`s — either a `{ … }` block or a single unbraced
  statement (`if (c) x = 1;`).  A missing `else` becomes a synthetic
  empty nil-valued `Block`.  **Else-if chains** (`else if (…)`) fall out
  of the grammar for free: the parser nests another `if_statement` inside
  the `else` statement, so it recurses into a *nested* `Expr::If` living
  in the outer `else_branch`'s tail value.  `Expr::If` is *not* gated by
  any `Feature` in SIR v0 (the validator observes none), so no manifest
  entry is added for conditionals.
- **`while (c) { body }` → `Stmt::While`.**  Lowers the condition
  expression and the body block.
- **C-style `for` → `Stmt::ForRange`** — accepted **only** for the
  canonical half-open counting shape, from which `var`/`start`/`stop`/
  `step` are extracted:

  - **init** must be `let i = <start>` (a single `let`/`const`/`var`
    binding of one variable);
  - **cond** must be `i < <stop>` or `i <= <stop>` on the *same* `i`
    (`<=` is rewritten half-open by bumping `stop` to
    `BuiltinCall("+", [stop, IntLit(1)])`);
  - **update** must increment `i` by a constant `step` in one of
    `i = i + <step>`, `i += <step>`, or `i++` (step `IntLit(1)`).

  Any **non-canonical** loop — a different variable across clauses, a
  decrement (`i--`), a multiplicative step (`i = i * 2`), a missing
  clause, or a multi-variable init — is rejected with a positioned
  `JsLowerError` (deferred) rather than silently mangled.
- **`for (const x of xs) { body }` → `Stmt::ForEach { var: x, iter: xs }`.**
  Only the single-identifier binding is supported; destructuring
  (`for (const [a, b] of …)`) is deferred.
- **Bare `{ … }` blocks** lower to `Expr::Block`; their statements run
  for effect.
- **Block-scoped names.**  Names bound inside any control-flow body or a
  bare block are block-scoped: `declared_locals` is snapshotted before a
  body and restored after, so an inner `let` does not leak outward.  This
  mirrors the SIR validator, which marks/rewinds its `LocalEnv` around
  each `Block`.  The **loop variable** is bound into the loop body scope
  *only* (visible in the body, unresolved after the loop) — again
  matching the validator.
- **Manifest.**  `while`/`for`/`for-of` declare `Feature::Loops` (the
  feature the validator observes for every loop statement); `if` declares
  nothing.
- **Bounded statement-block nesting.**  Control-flow bodies recurse with
  `depth + 1`, capped at `MAX_STMT_DEPTH = 256` (the statement-side twin
  of `MAX_EXPR_DEPTH`), so adversarial deep nesting yields an ordinary
  positioned error rather than a stack-overflow abort.
- 21 new unit tests (55 total): if/else with both branches, missing-else
  synthetic block, else-if nesting, single-statement if body, comparison
  conditions, `while`, all three C-`for` update forms (`i = i + s`,
  `i++`, `i += s`), `<=` half-open stop bump, literal stop, three
  non-canonical-`for` rejections (decrement, wrong cond variable,
  multiplicative step), `for-of`, loop-var and for-of-var
  non-leakage, block-scoped `let` non-leakage, bare block lowering,
  nested control flow, and an all-forms validate round-trip.

### Changed

- Minor version bump `0.2.0 → 0.3.0` (additive, backward-compatible).
- `lower_program` now delegates to a shared `lower_stmt_seq` routine that
  also lowers `{ … }` block / control-flow bodies, threading a
  statement-nesting `depth`.

### Deferred

Still **out of scope after M3** and returning a clear positioned
`JsLowerError`:

- **M4 — functions & closures:** `function` declarations, arrow
  functions (`MakeClosure`), `return`, calls (`DirectCall` /
  `IndirectCall`), `console.log` → `BuiltinCall("print", …)`.
- **M5 — collections:** array literals (`SeqLit`), indexing
  (`SeqIndex`), `.length` (`SeqLen`), object literals (`MapLit`),
  member/`[]` access (`MapGet`).
- **Other control-flow constructs:** `switch`, `try`/`catch`, `do … while`,
  labeled statements, and `break`/`continue` (the IR has no early-exit
  node) — all positioned `JsLowerError`.
- **Within-M2 operator/assignment gaps** (compound assignment outside the
  loop-update position, member/index assignment targets, multi-binding
  declarations, uninitialised bindings, bitwise/shift/exponentiation/
  nullish operators, other prefix unaries) and template literals,
  non-decimal numeric forms — unchanged from M2.

## 0.2.0

SIR19 milestone **M2 (variables, assignment, operators)** — builds on
M1's literal lowering.

### Added

- **Variable references.**  A bare identifier resolves to
  `Expr::VarRef { name, scope }`.  M2 has a single flat scope (everything
  lives in the synthetic `main`), so a declared name resolves to
  `Scope::Local`.  `undefined` continues to collapse to `NilLit`; any
  other *undeclared* identifier is a positioned
  `JsLowerError { message: "unresolved name reference `…`" }`.
- **Bindings.**  `let x = e;`, `const x = e;`, and `var x = e;` all lower
  to a binding statement.  We emit **`Stmt::LetStarBinding`** (sequential
  `let*`), not `Stmt::LetBinding`: the validator treats a run of
  consecutive `LetBinding`s as a *parallel* group whose RHSs cannot see
  one another, but JS `let`/`const` are sequentially scoped, so a
  perfectly ordinary `let x = 1; const y = x + 1;` must validate.  `let*`
  matches JS exactly.  `const`/`let`/`var` are not distinguished in v0
  (the IR models no immutability constraint); `var` hoisting is not
  modelled (binding emitted at its source position).
- **Re-assignment.**  `x = e;` to an already-declared name lowers to
  `Stmt::Assign { scope: Local }` (declares `Feature::MutableBindings`).
  A first bare `x = e;` with no prior declarator still binds (JS
  implicitly creates the binding), matching the resolution model.
- **Binary operators** (flat left-associative chains fold left into
  nested `BuiltinCall`s):
  - arithmetic `+ - * / %` → `BuiltinCall("+"/"-"/"*"/"/"/"%")`;
  - comparison `< > <= >=` → `BuiltinCall("<"/">"/"<="/">=")`.
- **Equality normalisation.**  Both `==` *and* `===` → `BuiltinCall("=")`,
  and both `!=` *and* `!==` → `BuiltinCall("!=")`.  This is a **deliberate
  semantic change**: the IR has only the strict-shaped comparison, so the
  loose-equality coercion cases (e.g. `null == undefined`, `true` in JS)
  become `false`.  Spec-sanctioned for v0 (SIR19 "Equality
  normalisation").
- **Logical operators.**  `&&` → `Expr::LogicalAnd`, `||` →
  `Expr::LogicalOr` — short-circuit nodes (declares
  `Feature::ShortCircuit`), **not** builtins (a builtin would eagerly
  evaluate both operands).
- **Unary operators.**  `!x` → `BuiltinCall("not", [x])`; `-x` →
  `BuiltinCall("neg", [x])`.  `-<numeric literal>` is **constant-folded**
  into a negative literal (`-5` → `IntLit(-5)`, `-3.25` →
  `FloatLit(-3.25)`), keeping the spec's `-7 → IntLit` row exact.
- **Bounded operand recursion.**  The precedence-spine peel stays
  iterative; only genuine operand descent recurses, capped at
  `MAX_EXPR_DEPTH = 256` so an adversarial deeply-nested expression
  yields an ordinary positioned error rather than a stack-overflow abort.
- 17 new unit tests (34 total): variable resolution incl. let-then-
  reassign and first-bare-assignment, unresolved-name error, every
  arithmetic/comparison operator, equality strict-normalisation,
  left-associativity and precedence, `&&`/`||` short-circuit nodes (and a
  guard that they are *not* builtins), unary `!`/`-`/`-literal` fold, a
  cross-referencing-bindings validation test (the parallel-`let` trap),
  and mixed-program validate round-trips.

### Changed

- Minor version bump `0.1.0 → 0.2.0` (additive, backward-compatible).
- The two M1 error-path tests that asserted operators and bare
  identifiers were *rejected* now assert they lower correctly.

### Deferred

Still **out of scope after M2** and returning a clear positioned
`JsLowerError`, tracked against the SIR19 spec:

- **M3 — control flow:** `if`/`else`, `while`, `for` (`ForRange`),
  `for-of` (`ForEach`).
- **M4 — functions & closures:** `function` declarations, arrow
  functions (`MakeClosure`), `return`, calls (`DirectCall` /
  `IndirectCall`), `console.log` → `BuiltinCall("print", …)`.
- **M5 — collections:** array literals (`SeqLit`), indexing
  (`SeqIndex`), `.length` (`SeqLen`), object literals (`MapLit`),
  member/`[]` access (`MapGet`).
- **Operator/assignment gaps within M2's families:** compound assignment
  (`+=`, `-=`, …), assignment to a member/index target (`obj.x = …`,
  `xs[i] = …`), multi-binding declarations (`let a = 1, b = 2;`),
  uninitialised bindings (`let x;`), bitwise / shift / exponentiation /
  nullish-coalescing operators, and other prefix unaries (`+`, `~`,
  `typeof`, `void`, `delete`).
- **Template literals** (backtick strings) — distinct token/rule; will
  desugar to concatenation in a later milestone.
- **Non-decimal numeric forms:** hex (`0x…`), octal (`0o…`), binary
  (`0b…`), and `BigInt` (`10n`) literals remain rejected.
- Everything in the SIR19 spec "Out of scope (deferred)" section:
  classes, exceptions, generators, `async`/`await`, destructuring,
  spread/rest, default parameters, ES modules, `eval`, regex.

## 0.1.0

First release — SIR19 milestone **M1 (crate skeleton + literal
lowering)**.

### Added

- Crate skeleton: `Cargo.toml` (path deps on `semantic-ir`,
  `coding-adventures-javascript-parser`, `parser`, `lexer`), `BUILD` /
  `BUILD_windows`, `README.md`, and this changelog.
- Public API:
  - `compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, JsLowerError>`
    — lower an already-parsed JavaScript CST.
  - `compile_source(source: &str, module_name: &str) -> Result<Module, JsLowerError>`
    — parse (with the `es2020` grammar) then lower, surfacing parse
    failures as `JsLowerError`s with a best-effort `line:column`.
  - `JsLowerError { message, line, column }`
    (`Debug`/`Clone`/`PartialEq`/`Eq`, plus `Display` + `std::error::Error`).
- Literal lowering from the **generic** `GrammarASTNode`:
  - integer-shaped number literal → `IntLit`;
  - decimal / exponent number literal → `FloatLit`;
  - `true` / `false` → `BoolLit`;
  - `null` **and** `undefined` → `NilLit` (the JS distinction is
    intentionally lost in v0);
  - string literal (single- or double-quoted) → `StrLit`.
- Synthesises an exported `main` function whose body's tail value is the
  final top-level literal (or `NilLit` for empty source).
- Stamps `metadata.source_language = "javascript"` and
  `metadata.sir_version = CURRENT_SIR_VERSION`, and emits a feature
  manifest declaring exactly the observed features (`Strings`,
  `Floats`), so every produced module passes `semantic_ir::validate`.
- 17 unit tests: one per literal kind, `compile_source` structural
  checks, a validate round-trip, and error paths (operator expression,
  bare identifier, parse failure, position extractor).

### Deferred

The following are explicitly **out of scope for M1** and currently
return a clear `JsLowerError`. They are scheduled for later milestones,
tracked against the SIR19 spec
(`code/specs/SIR19-javascript-to-semantic-ir.md`):

- **M2 — variables & operators:** variable references (`VarRef`),
  `let`/`const`/`var` (`LetBinding`), re-assignment (`Assign`),
  arithmetic / comparison / logical operators, unary `!`/`-`, loose vs.
  strict equality normalisation.
- **M3 — control flow:** `if`/`else`, `while`, `for` (`ForRange`),
  `for-of` (`ForEach`).
- **M4 — functions & closures:** `function` declarations, arrow
  functions (`MakeClosure`), `return`, calls (`DirectCall` /
  `IndirectCall`), `console.log` → `BuiltinCall("print", …)`.
- **M5 — collections:** array literals (`SeqLit`), indexing
  (`SeqIndex`), `.length` (`SeqLen`), object literals (`MapLit`),
  member/`[]` access (`MapGet`).
- **Template literals** (backtick strings, e.g. `` `a ${x} b` ``):
  deferred — these are a distinct token/rule from plain strings and will
  desugar to `+`-concatenation in a later milestone.
- **Non-decimal numeric forms:** hex (`0x…`), octal (`0o…`), binary
  (`0b…`), and `BigInt` (`10n`) literals are rejected in M1.
- Everything in the SIR19 spec "Out of scope (deferred)" section:
  classes, exceptions, generators, `async`/`await`, destructuring,
  spread/rest, default parameters, ES modules, `eval`, regex.
