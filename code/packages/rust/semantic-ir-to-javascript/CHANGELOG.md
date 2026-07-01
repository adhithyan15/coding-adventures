# Changelog

All notable changes to `semantic-ir-to-javascript` are documented here.

## Unreleased

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### Security

- **Allowlist method-dispatch names in `callMethod` to block a
  Function-constructor RCE (C3).**  `callMethod(recv, name, …args)` performed
  an unrestricted dynamic `recv[name]` lookup with an attacker-controlled
  `name`.  A translated untrusted program could therefore reach reflective
  gadgets — chiefly `constructor`, which on any function yields the global
  `Function` constructor, letting `id.constructor("return …evil…")` synthesise
  and run arbitrary code (a native higher-order method like
  `Array.prototype.map` then invokes it → remote code execution).  `apply`,
  `call`, `bind`, `__proto__`, `prototype`, and the `__define/lookup*etter__`
  pair were equally reachable.  `callMethod` now dispatches **only** through a
  fixed allowlist of known-safe Array / String / Number methods; any name
  outside it (every gadget included) throws a `TypeError` *before* the lookup.
  This is the primary, load-bearing gate — the emitted JS is what executes.
  A node execution-proof asserts `callMethod(id, "constructor", …)` throws
  instead of building a function.  `length` remains special-cased ahead of the
  allowlist as a property read.

## 0.6.0 — exception handling (try/catch/raise) + user-class ancestry (E1)

### Added

- **`Stmt::TryCatch` lowers to native `try`/`catch`/`finally` (E1).**  The
  backend previously *panicked* on any `TryCatch`.  It now emits a native
  `try { <body> } catch (__exc) { … } finally { <ensure> }`.  Because a native
  `catch` binds one variable and catches everything while Ruby has an ordered
  list of *typed* `rescue` clauses, the catch body is an if/else-if chain that
  asks `__Sir.rescueMatches(__exc, ["Foo", "Bar"])` for each clause in source
  order, binds `=> e` when present, and re-`throw`s the original exception if
  no clause matches (Ruby's "propagate when unrescued").  An empty
  `exception_types` is a bare `rescue` (catch-all).  Mirrors the TypeScript
  backend's `TryCatch` arm exactly, minus the type annotation on the binding.
- **`raise` builtin lowers to `__Sir.raiseError` (E1).**  `raise Foo, "msg"`
  (a `Const` class name + message) → `__Sir.raiseError("Foo", <msg>)`;
  `raise Foo` → `__Sir.raiseError("Foo")`; a non-`Const` first arg
  (`raise "msg"`) → `__Sir.raiseError("RuntimeError", <arg>)`; bare `raise` →
  `__Sir.raiseError()` (a generic `RuntimeError` re-raise).  Matches the TS
  backend's shape.
- **Inlined exception runtime.**  Ported the plain-JS-compatible pieces of the
  published `@coding-adventures/sir-runtime-exceptions` package into the
  backend's self-contained `__Sir` IIFE: a class-name-tagged `SirError` (a real
  `Error` subclass), `raiseError(cls, msg)`, `rescueMatches(exc, classNames)`,
  and the built-in Ruby `ANCESTRY` table (so `rescue StandardError` catches a
  `RuntimeError`/`ArgumentError`/…).  No `import`/`require`; the emitted `.js`
  still runs directly under `node`.
- **User-defined class ancestry (E2, the JS half).**  Added
  `__Sir.registerAncestry(map)`, which merges a user
  `{ childClass: superclassName }` map into the runtime's ancestry lookup.  The
  emitter collects every `Stmt::ClassDef { name, superclass: Some(_) }` pair in
  the module (recursing into nested bodies) and emits one
  `__Sir.registerAncestry({ … })` at program init — so
  `class MyErr < StandardError; raise MyErr; rescue StandardError` matches
  through the merged chain.  A `ClassDef` body's (non-`def`) statements are now
  emitted inline instead of panicking.
- **Accepts `Feature::Exceptions`, `Feature::Classes`, and `Feature::Constants`.**
  Exceptions and classes are lowered as above; `Constants` is accepted because
  `raise Foo` names its class as a `Const` `VarRef` (consumed by the `raise` arm
  as a string) — any other constant read emits its bare identifier.

### Security

- **Ancestry dispatch is by explicit table lookup, never reflection.**
  `rescueMatches` / `isAncestorOrSelf` resolve a class's superclass chain via
  `ancestry[cur]` string-map reads only — no `eval`, no dynamic code
  synthesis; class and method names are treated as pure data.  The mutable
  ancestry map is `Object.create(null)` (prototype-less), so a user class
  literally named `constructor`/`__proto__` cannot poison the lookup, and a
  malformed (cyclic) user map terminates via a `seen` guard.

### Tests

- Emitted-shape unit tests for the `TryCatch` else-chain, the four `raise`
  shapes, and one-shot `registerAncestry` emission (present iff a class
  inherits).
- Four `node` execution-proofs: built-in ancestry (`ArgumentError` caught by
  `rescue StandardError`), bare `rescue` catch-all, an unmatched type
  re-raising to a non-zero exit, and USER ancestry
  (`class MyErr < StandardError` caught by `rescue StandardError`).

## 0.5.0 — method dispatch (`__method__`) execution

Adds the minimal runtime support the JavaScript frontend's C3 member-method
lowering needs to **run**.  A method call `recv.meth(args…)` reaches the
backend as `BuiltinCall("__method__", [recv, StrLit("meth"), args…])`; the
emitter now routes it to a new runtime helper, `__Sir.callMethod`, which
invokes the JS-native method on the receiver (arrays' `push`/`pop`/`map`/
`filter`/`forEach`/`includes`/`reduce`/…, strings' `toUpperCase`/…) and
unwraps any `Closure` callback argument into a plain JS function.  This lets
JavaScript→SIR→JS collection programs execute end-to-end under `node`.

### Added

- `emit_builtin_call` special-cases `BuiltinCall("__method__", [recv,
  StrLit(name), args…])` → `__Sir.callMethod(recv, "name", args…)` (receiver
  first, method name second, call args after).
- Runtime `callMethod(recv, name, ...args)`: unwraps `Closure` args via
  `applyClosure`, accepts `length` as a nullary method, and dispatches to the
  native `recv[name]` method (throwing a clear `TypeError` when absent).

## 0.4.0 — KW4 (keyword-parameter & argument emission)

Replaces the KW1 compile-compat stubs with **real** keyword-parameter and
keyword-argument emission.  JavaScript has no native keyword-argument call
form, so — exactly as the TypeScript backend does (spec §4) — keyword
constructs lower to a zero-dependency **options object**.  No runtime
library is required; the lowering is direct.

### Added

- `accepts_features()` now declares `KeywordParams` (mirrors `DefaultParams`).
- **Def side.** A function's `Keyword` params (`def f(a:)` / `def f(a: 1)`)
  are folded into a single trailing options-object parameter `__kw`; the
  body prologue destructures it: `const { b, c = <default> } = __kw ?? {};`.
  A **required** keyword (`Keyword`, `default: None`) destructures bare; an
  **optional** keyword (`Keyword`, `default: Some(e)`) carries a JS
  destructuring default `name = <e>`, which fires on `undefined` exactly
  like SIR optional-keyword semantics.  The `?? {}` guard lets an
  all-optional callee be called with no options object.  When a keyword
  name is not a valid JS identifier, the prologue emits the explicit
  `{ "raw key": sanitized_local }` rename form so the object key still
  matches the call site.  `__kw` is collision-safe: `sanitize_ident` never
  produces a leading `__`, so no user parameter can sanitize to it.
- **Call side.** In a call's `args`, positionals emit as before and every
  `Expr::KeywordArg` collapses into one trailing object literal:
  `f(1, b: 2, c: 3)` → `f(1, { b: 2, c: 3 })`; a call with only keyword
  args → `f({ b: 2 })`; none → no trailing object.  `IndirectCall` routes
  the same object as the last element of its argument array.  The object
  key is the raw keyword `name`, matching the callee's destructuring
  prologue.  A new `emit_call_args` helper drives both call sites.

### Changed

- The `emit_expr` `KeywordArg` arm is now a pure defensive panic: keyword
  args are peeled off by `emit_call_args` before recursion, so reaching
  that arm signals a backend bug rather than a deferred feature.

### Tests

- Emitted-shape unit tests: trailing `__kw` object + destructuring
  prologue (required & optional keywords), keyword-only function, call-side
  object collapse (positional+keyword, keyword-only, none), and the
  `IndirectCall` object placement.
- Execution-proof through `node` (skips gracefully if absent):
  `add(5)` defaults the omitted keyword to 10 (→15) and
  `add(5, delta: 100)` supplies it (→105); a required-keyword call
  `pick(chosen: 7)` returns 7.

## 0.3.0 — P2d (default-parameter emission)

Adds **default parameters** to the JavaScript backend.  JavaScript's
native default-parameter feature has *exactly* SIR's semantics — the
default expression is evaluated **at call time**, only when the argument
is omitted, in **param scope** (so a later default may reference an
earlier parameter by name).  The lowering is therefore a direct native
inline: no runtime helper, no call-site padding.

### Added

- `accepts_features()` now declares `DefaultParams`.
- Emit: a `Param { default: Some(expr) }` lowers to a native JS default
  parameter `name = <emitted default>`.  The default expression is
  emitted with the ordinary `emit_expr`, so a default that references an
  earlier parameter (`VarRef { scope: Param }`) becomes a bare name —
  valid JavaScript, since earlier params are in scope left-to-right.
  `Rest`/`KwRest` params are unchanged; `IndirectCall` and closure
  defaults are unchanged / deferred.
- `DirectCall` documented and confirmed to emit **only the args present**
  — the SIR validator allows omitting trailing defaulted args (arity ≥
  `required_param_count`), and native JS defaults fill the omitted
  trailing params at call time.  No padding is inserted.
- Unit tests: `f(a, b = a + 1)` emits `function f(a, b = (a + 1)) {`; a
  short `DirectCall` (`f(5)`) is not padded.
- Integration test (`tests/run_with_node.rs`,
  `default_param_is_call_time_and_param_scoped`): hand-builds a module
  with `f(a, b = a + 1)` returning `b` and a `main` that calls
  `print(f(5))` then `print(f(5, 10))`, emits JavaScript, **runs it under
  `node`**, and asserts stdout `6` then `10` — proving the default is
  evaluated at call time (depends on the actual `a = 5`) and in param
  scope (references the earlier param `a`).

## 0.2.0 — D4 (completes SIR16 / v1 parity for the JS backend)

Brings the JavaScript backend to **full SIR16 / v1 parity**: the six
SIR16 features it previously deferred are now emitted and accepted.
JavaScript supports all of them natively, so each lowering is direct.

### Added

- `accepts_features()` now declares the v0 surface **plus all of SIR16**:
  `Floats`, `ShortCircuit`, `Sequences`, `Maps`, `MutableBindings`,
  `Loops`. (`accepts_intrinsics()` stays empty.)
- Emit arms for every SIR16 node:
  - `Floats` — `FloatLit` emits a native `number` literal (already wired
    in D1; the `Floats` capability is now accepted). `NaN`/`Infinity`/
    `-Infinity` spelled out; integer-valued floats keep an explicit `.0`.
  - `ShortCircuit` — `LogicalAnd`/`LogicalOr` emit a truthy-guarded arrow
    IIFE (`((__l) => __Sir.truthy(__l) ? (rhs) : __l)(lhs)` for And, the
    mirror for Or) so the rhs runs only when the lhs decides, routing the
    test through `__Sir.truthy` (only `false`/`nil` are falsy).
  - `Sequences` — `SeqLit` → `[…]`, `SeqIndex` → `(arr)[i]`, `SeqLen` →
    `(arr).length`, `SeqSet` → `(arr)[i] = v;` (native arrays).
  - `Maps` — `MapLit` → `new Map([[k, v], …])`, `MapGet` →
    `((m).get(k) ?? null)` (missing key reads as nil), `MapSet` →
    `(m).set(k, v);` (native `Map`, matching the TypeScript backend's
    representation).
  - `MutableBindings` — `Assign` (Local/Param/Capture/Global) → a plain
    `name = value;` reassignment. `let` (never `const`) is already the
    keyword for every binding, so no const→let pre-pass is needed (unlike
    the Rust/TypeScript backends).
  - `Loops` — `While` → `while (__Sir.truthy(cond)) { … }`; `ForRange` →
    a direction-aware C-style `for` with `stop`/`step` evaluated once into
    block-scoped `__sir_stop_N`/`__sir_step_N` temporaries (a per-module
    monotonic counter keeps them deterministic); `ForEach` → `for (let x
    of iter) { … }`.
- `emit_block_as_stmts` helper for loop bodies (trailing value discarded;
  a bare `nil` value is dropped).
- Unit tests for every new emit arm (floats incl. specials, short-circuit
  And/Or, seq build/index/len, map lit/get, assign, seq-set, map-set,
  while, for-range incl. distinct nested temporaries, for-each).
- Integration tests (`tests/run_with_node.rs`) that hand-build SIR16
  modules, emit JavaScript, **run it under `node`**, and assert stdout:
  float arithmetic promotion (`3.5`), short-circuit (rhs not evaluated),
  `or` first-truthy (`7`), sequence build/index/len/set, map
  build/get/set (incl. missing-key → nil), a `while` counter, a
  for-range accumulator (and a descending step), for-each over a
  sequence, and mutable reassignment (`42`).

### Still deferred (rejected at the capability check)

- String interpolation — `StrConcat` (`StringInterpolation`).
- OOP & exceptions — `ClassDef`/`ModuleDef`/`SingletonClassDef`,
  `TryCatch`, and the `Instance`/`ClassVar`/`Const` scopes (`Classes`,
  `Modules`, `InstanceVars`, `ClassVars`, `Constants`, `Exceptions`).
- `TailCalls` (V8 has no reliable TCO) and `Intrinsics` (empty
  whitelist).

The remaining `panic!` arms in `emit` cover only these unaccepted nodes,
so they are defence-in-depth (unreachable for a capability-checked
module), never reachable for an accepted feature.

## 0.1.0 — D1 (initial runnable core)

The first slice of the SIR18 JavaScript backend: the v0 expression /
statement core, emitting self-contained JavaScript that runs under
Node.js with no dependencies.

### Added

- `JavaScriptBackend` implementing `semantic_ir::Backend`:
  - `target_tag()` → `"javascript"`.
  - `accepts_features()` → the **v0 feature set** (`Closures`, `Pairs`,
    `Symbols`, `Strings`, `DynamicTyping`, `OptionalTypeAnnotations`,
    `MutualRecursion`, `Globals`).
  - `accepts_intrinsics()` → empty.
  - `compile()` → validate → capability check → reject `TailCalls` →
    lower to JavaScript.
- `compile(&module)` convenience free function.
- Inlined `__Sir` runtime (`src/runtime.rs`): an IIFE with `Sym`/`Pair`/
  `Closure` classes, symbol interning, `applyClosure`, SIR `truthy`,
  `format`/`print`, and a builtins dispatch table (arithmetic,
  comparison, pair ops, predicates, `len`, `range`). Pasted verbatim
  into every artifact, so output is fully self-contained.
- Emitter (`src/emit.rs`) for the v0 nodes:
  - Literals: `IntLit`, `FloatLit`, `BoolLit`, `NilLit`, `StrLit`,
    `SymLit`.
  - `VarRef` by scope: `Local`/`Param`/`Capture`/`Global` → bare
    identifier; `Builtin` → `__Sir.builtinClosure("name")`.
  - `If` → SIR-truthy ternary.
  - `Block`: function-body form (flat `{ …; return v; }`) and
    expression form (IIFE).
  - `DirectCall`, `IndirectCall` (`__Sir.applyClosure`), `BuiltinCall`
    (native-infix specialisation for `+ - * / % = != < > <= >=`,
    `not`/`neg`/`len`, `__Sir.print`; everything else via
    `__Sir.callBuiltin`).
  - `Function` declarations (captures prepended before params; native
    `...rest` for rest params).
  - `LetBinding`/`LetStarBinding`/`ExprStmt`; the `_init` `global_set`
    pattern renders as a direct assignment.
  - `MakeClosure` → `new __Sir.Closure((..._a) => fn(caps…, ..._a))`.
  - Module wrapping: banner comment, `"use strict";`, inlined runtime,
    module globals, function declarations, then `_init()` and `main()`.
- `sanitize_ident` (reserved words → `_$` prefix; invalid chars →
  `_$<hex>`; empty → `_$empty`), JS string escaping, and float
  formatting (explicit decimal point; `NaN`/`Infinity` handled).
- Tests: unit coverage for `sanitize_ident` and each emit arm, a
  determinism test, and an end-to-end integration test
  (`tests/run_with_node.rs`) that lowers Twig → SIR → JS and **executes
  the result under `node`** (add → `3`, factorial → `120`,
  closure-adder → `8`), skipping execution when `node` is absent.
- Package scaffolding: `Cargo.toml`, `README.md`, this changelog, and
  `BUILD` / `BUILD_windows`. Registered in the Rust workspace
  (`code/packages/rust/Cargo.toml`).

### Deferred

The following are intentionally **not** implemented in this milestone and
are **rejected at the capability check** (their `Feature`s are absent
from `accepts_features()`), so a module that uses them is turned away
rather than mis-compiled:

- Collections — `SeqLit`/`SeqIndex`/`SeqLen`, `MapLit`/`MapGet`
  (`Sequences`, `Maps`).
- Loops — `While`/`ForRange`/`ForEach` (`Loops`).
- Mutation — mutable `Assign`, `SeqSet`/`MapSet` (`MutableBindings`).
- Short-circuit — `LogicalAnd`/`LogicalOr` (`ShortCircuit`).
- Floats as a declared feature (`FloatLit` *emission* is implemented,
  but the `Floats` capability is not yet accepted).
- String interpolation — `StrConcat` (`StringInterpolation`).
- OOP & exceptions — `ClassDef`/`ModuleDef`/`SingletonClassDef`,
  `TryCatch`, and the `Instance`/`ClassVar`/`Const` scopes
  (`Classes`, `Modules`, `InstanceVars`, `ClassVars`, `Constants`,
  `Exceptions`).
- `TailCalls` (V8 has no reliable TCO) and `Intrinsics` (empty
  whitelist) — fundamentally unsupported / out of scope for v0.
