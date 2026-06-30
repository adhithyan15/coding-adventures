# Changelog

All notable changes to `semantic-ir-to-javascript` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
