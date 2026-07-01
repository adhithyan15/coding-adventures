# Changelog

## 0.7.0

### Added

- **Collection-method dispatch + runtime catalog (C5).**  The Go backend now
  EXECUTES `recv.meth(args…)` end to end.  A method call reaches the backend as
  `BuiltinCall("__method__", [recv, StrLit("meth"), …args])`; previously it fell
  through to the generic `_sir_call_builtin_by_name` fallback, which has no
  method-dispatch arm — so any collection method failed at runtime.  Now:
  - **Emit** (`emit.rs`): a `"__method__"` case in `emit_builtin_call` lowers the
    dispatch to `_sir_call_method(recv, "name", []Value{…args})`.  A trailing
    block (`MakeClosure`) rides in as the last `[]Value` element; a `&:sym` /
    `&proc` block-pass that survives on the dispatch is converted via
    `try_emit_block_pass` (`_sir_sym_to_proc(intern("sym"))` for `&:sym`, the
    proc verbatim otherwise).  A `Const`-scoped class operand on a class
    predicate (`x.is_a?(Integer)`) is passed as its name string.
  - **Runtime** (`runtime.rs`): a new inlined `_sir_call_method(recv, name, args)`
    implements the collection-method catalog by an **explicit type-switch +
    method-name switch** (Array `*Seq` / Hash `*Map` / String / Numeric / Symbol),
    **ported from the Python/TS `sir-runtime-oop` reference for behavioural
    parity** (same method names, same semantics).  Implemented:
    - **Array**: `length`/`size`/`count`, `first`, `last`, `empty?`, `include?`,
      `index`, `push`/`append`, `<<`, `pop`, `shift`, `reverse`, `sort`, `join`,
      `to_a`, plus block methods `each`, `map`/`collect`, `select`/`filter`,
      `reject`, `reduce`/`inject`, `find`/`detect`, `any?`, `all?`, `none?`.
    - **Hash**: `keys`, `values`, `has_key?`/`key?`/`include?`/`member?`,
      `has_value?`/`value?`, `size`/`length`, `empty?`, plus block methods `each`/
      `each_pair`, `map`, `select`/`filter`, `reject`.
    - **String**: `length`/`size`, `upcase`, `downcase`, `reverse`, `strip`/
      `lstrip`/`rstrip`, `empty?`, `include?`, `start_with?`, `end_with?`, `split`,
      `chars`, `to_i`, `to_f`, `to_sym`.
    - **Numeric**: `abs`, `to_i`, `to_f`, `even?`, `odd?`, `zero?`, `positive?`,
      `negative?`, `succ`/`next`, `pred`, plus the block method `times`.
    - **Symbol**: `to_s`, `to_sym`, `length`/`size`, `upcase`, `downcase`,
      `empty?`.
    - **Universal** (every receiver): `nil?`, `==`, `!=`, `class`, `to_s`,
      `itself`.
    - **`Symbol#to_proc`** (`_sir_sym_to_proc`): `&:sym` becomes a `*Closure`
      that re-enters dispatch on its first argument, so `map(&:to_s)` behaves
      exactly like `map { |x| x.to_s }`.
  - **Security (the C3 RCE lesson)**: dispatch is ONLY through the explicit
    catalog switches — there is **no reflection** on the raw method name, no
    dynamic Go method/field lookup.  The catalog switch IS the allowlist.  An
    unknown method on a known receiver falls through to `_sir_method_unknown`,
    which panics with a controlled `undefined method '<name>' for <Class>`
    message — a surfaced runtime error, never arbitrary behaviour.
  - **Capability gate** (`lib.rs`): a **pure** collection-method module (a
    `__method__` dispatch with NO class features) is now proven accepted.  This
    needs no gate change and no new `Feature` variant (the deferred C1
    `MethodDispatch` is not required): the validator observes no feature for
    `__method__`, so such a module carries only its receiver/argument features
    (`Sequences`/`Strings`/`Closures`/`Symbols`/`Maps`/`DynamicTyping`), all
    already accepted — while class-bearing modules stay rejected
    (`Feature::Classes` is not accepted).  The runtime catalog is the real gate.
  - Adds `sort` + `strings` to the emitted import block (the runtime catalog
    always references both).
  - Tests: emitted-shape unit tests (dispatch call shape, block/`&:sym` shapes,
    class-predicate name-string, catalog present in the preamble); acceptance
    tests (pure dispatch accepted, classes still rejected); and an
    **execution-proof** integration test (`compile_and_run_coll_methods.rs`) that
    runs `.map`/`.select`/`.length`/`.reduce`/`.join`/`.sort`/`.reverse`/
    `.upcase`/`.split`/`.even?`/`.abs`/`.keys`/`&:to_s` through real `go run` and
    diffs stdout against the Python/TS reference values, plus a proof that an
    unknown method (`[1].bogus_xyz`) exits non-zero with the controlled
    "undefined method" message.

## Unreleased

### Fixed

- **Reject keyword params mixed with `*rest`/`**kwrest` (unsound static
  resolution).**  KW6 resolves keyword arguments by *static* keyword→positional
  slot mapping, which is only sound for **fixed-arity** callees.  The core
  validator, however, accepts a callee that mixes a `Keyword` param with a
  variadic (its ordering rule is `Required* Rest? Keyword* KwRest?`, so Ruby's
  `def f(a, *rest, x: 1)` is well-formed), and this backend accepts
  `Feature::KeywordParams` — so such a module reached `emit_direct_call`, where
  the `*rest` slot has no fixed position for a keyword to resolve against.  The
  result was a **panic** in debug builds (`debug_assert!` in the slot loop) or a
  **silent mis-emit** in release builds (a single `_sir_missing` sentinel landed
  in the variadic slot instead of a collected sequence).  A new capability check
  (`check_no_keyword_rest_mix`, run beside the manifest gate in `compile`) now
  returns a clean `BackendError { kind: UnsupportedFeature }` for any function
  carrying BOTH a `Keyword` param AND a `Rest`/`KwRest` param, naming the
  offending function.  This becomes frontend-reachable once the Ruby frontend
  (KW7) emits keyword+splat methods.  The keyword-params-**without**-rest happy
  path (fixed arity) is unchanged and still passes all existing tests.
  Added unit tests for both the `*rest` and `**kwrest` rejections and for the
  preserved happy path.

## 0.6.0 — KW6 keyword parameters & arguments via static positional resolution

Adds `Feature::KeywordParams` to the Go backend's accepted set (see
`code/specs/sir-keyword-params.md`, §4 Go row).  Go has **no** native keyword
arguments, so the backend lowers them **directly** — no runtime library — by
resolving each keyword to a positional slot at *emit time* (a `DirectCall`'s
callee signature is statically known).  This mirrors the Rust backend's
strategy and reuses the SIR19 default-parameter machinery (the `_sir_missing`
sentinel + callee body prologue) unchanged.

### Added

- **Keyword def params are positional-ized.**  A `ParamKind::Keyword` parameter
  emits as an ordinary positional Go parameter in declared order — the
  by-name-ness is a source affordance the backend resolves at the call site.
  An *optional* keyword (`Keyword` + `default: Some`) reuses the existing
  default-param prologue: `if _sir_is_missing(name) { name = <default> }`.
- **Static keyword→positional call resolution.**  A `DirectCall` whose `args`
  contain `Expr::KeywordArg{ name, value }` elements is emitted as a plain
  positional Go call, built in the callee's declared param order:
  leading positionals fill leading slots; each `KeywordArg` fills the slot
  whose param **name** matches (source order irrelevant); every omitted
  *optional* slot is padded with `_sir_missing` (the callee prologue supplies
  the default). Worked example — `greet(greeting:, name: "world")`:
  `greet(greeting: "hi")` → `greet("hi", _sir_missing)`;
  `greet(name: "ada", greeting: "hi")` → `greet("hi", "ada")`.
- **`FN_PARAMS` signature table.**  A new per-module thread-local mapping each
  function name to its parameter shapes (name, is-keyword, has-default), in
  order, populated by `emit_module` alongside `FN_ARITY`.  The `DirectCall`
  arm consults it to reorder keywords by name — `FN_ARITY` alone knows only
  *how many* params, not their names.

### Tests

- Emitted-shape unit tests: positional-ized keyword def with optional-keyword
  default prologue; keyword call reordered to declared order (source order
  scrambled); omitted optional keyword padded with the sentinel; mixed
  positional + keyword call.
- Execution proof (`tests/compile_and_run_keyword_params.rs`): a
  `greet(greeting:, name: "world")` module compiled and run through `go run`,
  asserting `greet(greeting: "hi")` prints `(hi world)` (default filled) and
  `greet(greeting: "hi", name: "ada")` prints `(hi ada)` (supplied). Skips
  gracefully if `go` is absent.

### Deferred (spec §Out of scope)

- **Indirect/closure keyword calls.**  An `IndirectCall`/`MakeClosure` cannot
  resolve keywords by name (the callee signature is not statically known); the
  frontends do not emit such calls, so a `KeywordArg` reaching that path
  panics with a documented deferral message rather than mis-emitting.

## 0.5.0 — SIR19 default parameters (P2f) via missing-sentinel runtime-mimic

Adds `Feature::DefaultParams` to the Go backend's accepted set.  Go has no
native optional/default parameters and emitted functions are *fixed-arity*
over `Value`, so the backend uses a **runtime-mimic** strategy: a unique
package-level MISSING sentinel flows through the ordinary `Value` channel.

Semantics are **call-time, param-scope**: a default expression is evaluated
each call, in the callee, where the *earlier* parameters are already bound
(so a later default may reference an earlier param — `def f(a, b = a + 1)`).

### Added

- **Runtime MISSING sentinel.**  A distinct, otherwise-empty `_missingMarker`
  struct type plus the single shared instance `var _sir_missing Value =
  &_missingMarker{}`.  A program can never construct one itself (no IR node
  lowers to it), so pointer identity makes the new
  `func _sir_is_missing(v Value) bool` predicate exact and total.
- **Caller-side padding.**  A `DirectCall` that omits trailing defaulted
  arguments pads the call up to the callee's full (fixed) param count with
  `_sir_missing`, e.g. `f(5)` for `f(a, b = …)` emits
  `f(Value(int64(5)), _sir_missing)`.  The full param count comes from the
  module's function table (`FN_ARITY`, populated by `emit_module` before any
  body is walked).
- **Callee body prologue.**  Each defaulted parameter gets a guard at the top
  of the function body, in declaration order:
  `if _sir_is_missing(<name>) { <name> = <emitted default expr> }`.  Ordering
  is what makes a later default see an earlier param's already-resolved
  value.  Reassigning a parameter is ordinary Go (parameters are mutable
  locals) and the guard itself "uses" the param, so Go's strict
  unused-variable rule is satisfied even when the body never reads it.

### Changed

- **`_sir_format` / `_sir_value_eq`** defensively handle the sentinel — it
  never reaches a print or `=` path in a well-formed program (a defaulted
  param is always replaced before use), but `_sir_format` renders a stray
  sentinel as `<missing>` and `_sir_value_eq` treats two sentinels as equal
  and a sentinel as equal to nothing else, so it can never masquerade as a
  user value.

### Tested

- Unit tests assert the emitted shape: the body prologue (`if
  _sir_is_missing(b) { b = _sir_plus([]Value{a, Value(int64(1))}) }`), that a
  required param emits no guard, and that `DirectCall` padding appends the
  right number of sentinels.  Runtime tests assert the sentinel type, the
  `_sir_is_missing` helper, and the defensive format/eq guards.
- New `go run` integration test (`compile_and_run_default_params.rs`):
  module `f(a, b = a + 1)` returning `b`, `main` prints `f(5)` then
  `f(5, 10)`; the emitted Go is compiled and run under the real Go toolchain
  and stdout is asserted to be `6` then `10` (the default ran and saw
  `a = 5`; a supplied argument suppressed it).  The four existing `go run`
  tests (floats / loops / seq+maps / cyclic) still pass.

### Housekeeping

- Fixed three pre-existing `clippy` lints in `emit.rs` (a `write!`-with-
  trailing-newline, a needless lifetime on `pick_global_set`, and a
  `len() >= 1`) so the crate is clippy-clean under `--all-targets`.

## 0.4.1 — harden emitted Go runtime against cyclic Seq/Map

`*Seq`/`*Map` are shared, *mutable* handles, so an emitted Go program can
build a cyclic structure (`xs = [0]; xs[0] = xs`).  Before this release the
emitted runtime walked such values structurally with no cycle protection,
so a cyclic value would make **`_sir_format`** recurse forever and overflow
the stack while printing, and make **`_sir_value_eq`** recurse forever when
comparing two *distinct* cyclic structures (a self-cycle was already short-
circuited by the same-pointer fast path, but distinct cyclic operands were
not).  This mirrors the Rust backend's `0.4.1` cyclic-guard.

This is a robustness fix only — the public runtime API and the printed form
of every *non-cyclic* value are byte-identical (all existing tests pass
unchanged).

### Fixed

- **`_sir_format` / `_sir_format_seq` / `_sir_format_map`** now thread a
  visited-pointer set through a new `_sir_format_d(v, visited)` variant.
  The set is a `map[Value]bool` keyed on the Seq/Map **pointer** — a
  `*Seq`/`*Map` boxed in the `Value` (`interface{}`) compares by pointer
  identity, the idiomatic Go way to key on handle identity.  A handle is
  inserted on entry and removed on exit, so it is only "seen" along the
  *current* path: a true cycle re-entering a handle within its own subtree
  prints a placeholder (`[...]` for a seq, `{...}` for a map) and returns
  instead of recursing, while a value reached twice by sibling (non-cyclic)
  paths still prints in full.  `_sir_format_pair` threads the set too (a
  pair can hold a cyclic seq/map).  The public `_sir_format(Value) string`
  signature is unchanged — it allocates a fresh visited set and delegates.
- **`_sir_value_eq`** keeps the same-pointer (`as == bs`) identity fast
  path and adds a co-inductive `pending` set of handle-pairs currently
  being compared (a `map[[2]Value]bool` keyed on the two boxed pointers)
  via a new `_sir_value_eq_d(a, b, pending)` variant: re-encountering a
  pair already in flight (a cycle matched in lock-step) is treated as
  equal, bounding the deep comparison of two distinct cyclic operands so it
  always terminates.
- **`_sir_map_get` / `_sir_map_set` / `_sir_map_put`** need no
  restructuring: Go has no `RefCell`-style aliasing-borrow check (the Rust
  backend's "already mutably borrowed" panic on a self-referential key has
  no Go analogue), and the remaining hazard — a cyclic key making
  `_sir_value_eq` recurse forever — is now handled by that function's
  co-inductive guard.  A comment on `_sir_map_put` records this.

### Tests

- New `tests/compile_and_run_cyclic.rs` integration test: hand-builds a
  module that constructs a cyclic seq (`xs = [0]; xs[0] = xs; print(xs)`),
  emits Go, `go run`s it (gated on `go` availability), and asserts the
  program *terminates* and prints the `[[...]]` placeholder.  It also
  checks that `_sir_value_eq` terminates on both a self-cyclic operand (via
  the same-pointer fast path) and two *distinct* cyclic structures (via the
  co-inductive guard), both `#t`.
- Two new runtime unit tests assert the cycle-guard plumbing is present in
  the emitted runtime string (`_sir_format_d` / `_sir_value_eq_d` and the
  placeholder literals).

## 0.4.0 — SIR16 Sequences + Maps — completes Go v1 parity (A6)

The final two SIR16 (v1) features land in the Go backend.  With them the
Go backend accepts **all six** SIR16 features (Floats, ShortCircuit,
MutableBindings, Loops, Sequences, Maps) — reaching **full SIR-v1
parity**.  Go is the **fifth and last backend to reach v1**, completing
the backend fleet (joining TypeScript, Rust, Python, and the others).
Before this release a module using `SeqLit` / `SeqIndex` / `SeqLen` /
`MapLit` / `MapGet` / `SeqSet` / `MapSet` was rejected at the capability
check and those emit arms were unreachable `panic!`s; this release wires
them up end-to-end.

### Added

- `Feature::Sequences` and `Feature::Maps` join the backend's
  `ACCEPTED_FEATURES`, so a module declaring them is no longer rejected
  by the capability check.
- **Sequences** — the inlined Go runtime gains a `*Seq` value (a struct
  `Seq{ Items []Value }` held by pointer).  The pointer is the crux: a
  `SeqSet` (`xs[i] = v`) mutates the very sequence the caller holds, and
  two bindings that alias the same literal observe each other's writes —
  the reference semantics of a Python list / JS array.  Copying a `Value`
  that holds a `*Seq` copies the handle, not the backing slice.
  - `SeqLit` → `_sir_seq_lit([]Value{...})` builds a fresh shared seq.
  - `SeqIndex` → `_sir_seq_index(seq, i)` (strict bounds; out-of-range
    panics, like `car`/`cdr`).
  - `SeqLen` → `_sir_seq_len(seq)` returns the element count as `int64`.
  - `SeqSet` → `_ = _sir_seq_set(seq, i, v)` mutates in place (no
    auto-grow; out-of-range panics).
- **Maps** — the runtime gains a `*Map` value (a struct
  `Map{ Entries []MapEntry }`, an *insertion-ordered* association list).
  Go's native `map` can't key on an arbitrary `Value` (floats, closures,
  nested seqs/maps aren't usable keys), so — mirroring the Rust backend —
  keys are compared with the runtime's structural value-equality
  (`_sir_value_eq`, a linear scan).  A missing key reads as `nil`.
  - `MapLit` → `_sir_map_lit([]Value{keys...}, []Value{vals...})` (keys
    and values emitted as two parallel slices since Go has no tuple
    literal); last-write-wins on duplicate keys, first-seen order kept.
  - `MapGet` → `_sir_map_get(map, key)` (missing key ⇒ `nil`).
  - `MapSet` → `_ = _sir_map_set(map, key, v)` inserts (appends, order-
    preserving) or overwrites in place.
- **Structural value-equality** — `_sir_eq` now routes through a new
  `_sir_value_eq` that handles the whole value tower (numbers cross-type,
  symbols, pairs, and now seqs/maps element-wise / entry-wise, with
  identical-handle short-circuit).  This is the single source of truth
  shared by `=` and map-key lookup.
- **ForEach reconciliation** — `_sir_seq_iter` (the A5 cons-list
  flattener used by `ForEach`) now *also* snapshots a real `*Seq`, so
  `for x in [1, 2, 3]` (a `SeqLit`) iterates end to end while
  `ForEach`-over-cons-list keeps working.  A `*Seq` is copied element-wise
  into a fresh `[]Value` so the loop body sees a stable view even if it
  mutates the underlying sequence.
- **Display** — `_sir_format` renders a seq as a bracketed list
  (`[1, 2, 3]`) and a map as a brace-wrapped, insertion-ordered entry
  list (`{a: 1, b: 2}`).
- New integration test `tests/compile_and_run_seq_maps.rs` — hand-builds
  a module that exercises a sequence (lit/index/len/set + aliasing), a
  map (lit/get/set + missing-key ⇒ nil), and a `for x in [10,20,30]`
  ForEach accumulation; emits Go, `go run`s it (gated on `go`
  availability), and asserts stdout (`99 / 3 / 99 / 2 / 3 / nil / 60`).
  This is the only check that catches Go's `:=`-vs-`=` and
  unused-variable strictness.

### Notes

- `accepts_features` is now in lockstep with emit for **all six** SIR16
  features: every declared feature has a real (non-panicking) emit path.
  The only remaining `panic!` reject arms cover SIR17/18 nodes
  (classes / module-defs / exceptions / `StrConcat`) whose features stay
  unaccepted, so they remain strictly unreachable.

## 0.3.0 — SIR16 MutableBindings + Loops (A5)

The next two SIR16 (v1) features land in the Go backend, mirroring the
merged Rust backend equivalent.  Before this release the Go backend
accepted only `Floats` + `ShortCircuit`, so every `Assign` / `While` /
`ForRange` / `ForEach` IR node hit a `panic!` reject arm.  This release
wires up mutation and the three loop forms end-to-end onto Go's native
`for`.

### Added

- `Feature::MutableBindings` and `Feature::Loops` join the backend's
  `ACCEPTED_FEATURES`, so a module declaring them is no longer rejected
  by the capability check.
- **MutableBindings** — `Stmt::Assign` to a Local/Param/Capture emits a
  plain `<name> = <value>`.  Go has no const/mut distinction, so unlike
  the Rust backend (which needs a `let mut` pre-pass) reassignment just
  works against the name already declared by the matching `LetBinding`
  (`:=`) or parameter.  A `Global` assignment writes through the runtime
  global store (`_sir_globals[<key>] = <value>`).
- **Loops** — `Stmt::While` / `ForRange` / `ForEach` map onto Go's
  native `for`:
  - `While` → `for _sir_truthy(<cond>) { <body> }` (Go's `for` is its
    `while`; the test routes through SIR truthiness, never Go `bool`).
  - `ForRange` → a native three-clause `for` whose `stop`/`step` bounds
    are cached **once** into `int64` temporaries (re-evaluating Python's
    `range` bounds each turn would be wrong).  A direction-aware
    continue test (`_sir_range_cont`) lets a negative `step` count down.
    The loop variable is re-bound each turn as a fresh `Value(int64(…))`
    and guarded with `_ = <var>` so an unused loop var still compiles.
  - `ForEach` → `for _, <var> := range _sir_seq_iter(<iter>)`.  The new
    runtime `_sir_seq_iter` flattens a cons-list (`Pair`-chain ending in
    `nil`) into a `[]Value` (Sequences land in a later PR, so a
    "sequence" is still the classic cons-list).
- Loop bodies emit in statement context: a body's trailing non-`nil`
  value becomes `_ = <value>` (so side effects fire), and introduced
  loop variables get a `_ = <var>` guard — satisfying Go's strict
  unused-variable rule even when the body ignores them.
- New runtime helpers `_sir_range_cont` and `_sir_seq_iter`.  (`ForRange`
  reuses the existing `_sir_as_int` from the Floats release for its
  bound extraction.)
- New integration test `tests/compile_and_run_loops.rs` — hand-builds a
  module using a mutable accumulator, a `for`-range, and a `while`
  countdown, emits Go, `go run`s it (gated on `go` availability), and
  asserts stdout (`sum 0..5 = 10`, countdown to `0`, reassign to `99`).
  This is the only check that catches Go's `:=`-vs-`=` and
  unused-variable strictness.

### Notes

- Only two SIR16 features remain undeclared (`Sequences`, `Maps`); their
  `SeqLit` / `MapLit` / `SeqSet` / `MapSet` nodes still hit `panic!`
  reject arms, kept strictly unreachable by the capability check until a
  later PR.  `accepts_features` stays in lockstep with emit: every
  declared feature has a real (non-panicking) emit path.

## 0.2.0 — SIR16 Floats + ShortCircuit (A4)

First two SIR16 (v1) features land in the Go backend, mirroring the
just-merged Rust backend equivalent.  Before this release the Go backend
declared *none* of the six SIR16 features, so every SIR16 IR node hit a
`panic!` reject arm.  This release wires up two of them end-to-end.

### Added

- `Feature::Floats` and `Feature::ShortCircuit` join the backend's
  `ACCEPTED_FEATURES`, so a module declaring them is no longer rejected
  by the capability check.
- **Floats** — the inlined Go runtime's `Value` (`interface{}`) now
  accepts a `float64` arm:
  - New helpers `_sir_as_float`, `_sir_any_float`, `_sir_is_number_val`,
    and `_sir_format_float`.
  - Arithmetic (`+ - * /`) keeps the exact int64 fast-path while every
    operand is an integer, and promotes the whole fold to `float64` the
    moment any operand is a float ("int op float ⇒ float").  Integer
    division keeps its divide-by-zero panic; float division follows
    IEEE-754 (`1.0/0.0 ⇒ +Inf`).
  - `=` is cross-type for numbers (`1 == 1.0` is true) and uses IEEE
    equality for floats (`NaN != NaN`).  `<` / `>` compare numerically,
    staying on the int path when both operands are int64.
  - `number?` is true for both integers and floats.
  - `FloatLit` emits `Value(float64(<lit>))`; integral values spell out
    `3.0` (never `3`) so the runtime type-switch hits the float arm.
    Non-finite values route through `math.NaN()` / `math.Inf(±1)` since
    Go has no float literal for them.
  - Display: `_sir_format_float` prints integral floats with a trailing
    `.0` (`3.0`, not Go's default `%v`-style `3`), fractional values via
    `strconv.FormatFloat(x, 'g', -1, 64)`, and non-finite values as
    `NaN` / `inf` / `-inf` — matching the Rust backend's intent.
- **ShortCircuit** — `LogicalAnd` / `LogicalOr` emit a truthy-guarded
  immediately-invoked func literal:
  `func() Value { __l := <lhs>; if _sir_truthy(__l) { return <rhs> } else { return __l } }()`
  (and the mirror for `or`).  The operand value is returned (not a
  coerced bool), `lhs` is evaluated exactly once, and each IIFE scopes
  its own `__l` so nesting never collides.  Pure emit — no runtime
  change.
- The emitter now imports `"math"` (alongside `"fmt"` and `"strconv"`);
  the runtime always references it via the float `NaN`/`Inf` checks, so
  Go's unused-import rule stays satisfied.
- Integration test `tests/compile_and_run_floats.rs`: hand-builds a SIR
  module exercising floats, short-circuit, and cross-type equality;
  emits Go, runs it with `go run`, and asserts stdout
  (`4.0 / 4.0 / 5 / 7 / #f / #t`).  Gated on `go version` — skips with a
  log line if the Go toolchain is absent.

### Notes

- The remaining four SIR16 features (MutableBindings, Loops, Sequences,
  Maps) are still **not** declared, so the corresponding emit arms
  (`SeqLit`, `MapLit`, `Assign`, `While`, …) remain reachable only as
  internal-bug `panic!`s — the capability check rejects such modules
  before emit.  They land in later Go PRs.

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

## 0.1.0 — initial release (SIR15 v0)

Fourth backend for the narrow-waist Semantic IR.  Emits
self-contained Go source from a `semantic_ir::Module`.

### Added

- `GoBackend` implementing `semantic_ir::Backend` with
  `target_tag = "go"`; accepts the v0 feature set minus
  `TailCalls` and `Intrinsics`.
- Per-node lowering per SIR15.  Notable Go-isms:
  - `If` and non-trivial `Block` lower to immediately-invoked
    function expressions (`func() Value { ... }()`) since Go has
    no expression-position blocks.
  - `MakeClosure` emits an adapter `func([]Value) Value` that
    splats the runtime args into the synthesised lambda's
    positional parameters; the per-function arity table is
    threaded through TLS so the splat is sized correctly.
  - `LetBinding` emits `name := value` followed by a defensive
    `_ = name` so unused bindings don't break Go's strict
    unused-variable rule.
  - `ExprStmt` emits `_ = expr` for the same reason.
- Inlined Go runtime (~280 lines) covering `Value` (`interface{}`),
  `Symbol`, `Pair`, `Closure`, all 15 Twig builtins, symbol
  interning, module globals, `_sir_format` and `_sir_truthy` and
  `_sir_apply` and `_sir_make_closure`, plus a `_sir_call_builtin_by_name`
  dispatch table for `VarRef Builtin`.
- Identifier sanitisation handles Go keywords (`for`, `func`,
  `chan`, etc.) and predeclared builtins (`int`, `string`,
  `print`, `len`, etc.) by appending `_`.  Other invalid chars
  encode as `_<hex>`.  Empty → `_sir_empty`.  SIR's `main` is
  renamed to `_sir_user_main` so the emitter's own `main()`
  doesn't collide.
- `sanitize_comment` strips line terminators from external
  strings written into `//` comments — same defence as SIR12 /
  SIR13 / SIR14.
- Pre-lowering validation via `semantic_ir::validate`; capability
  check via `Backend::check_module`.

### Notes

- The runtime always imports both `"fmt"` and `"strconv"` — both
  are referenced inside the runtime block, so Go's strict
  unused-import rule never fires regardless of what the user
  module uses.
