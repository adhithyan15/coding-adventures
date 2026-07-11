# semantic-ir-to-go

Fourth backend for the narrow-waist Semantic IR.  Lowers
[semantic-ir](../semantic-ir/) modules into **self-contained** Go source
code — every emitted `.go` file is `package main` with inlined runtime
helpers; no `go.mod` external dependencies.  `go build <file>.go` builds
a working binary.

Implements [SIR15](../../../specs/SIR15-semantic-ir-to-go.md).

## Public API

```rust
use semantic_ir_to_go::{compile, GoBackend};
use semantic_ir::Backend;

let artifact = compile(&sir_module)?;
let backend = GoBackend::new();
let artifact = backend.compile(&sir_module)?;
```

## Capability declaration

Accepts the full v0 feature set minus `TailCalls` (Go has no TCO) and
`Intrinsics` (empty whitelist in v0).

### SIR16 (v1) — complete

The Go backend accepts **all six** SIR16 features below, reaching **full
SIR-v1 parity**.  Go is the fifth and last backend to reach v1.


- **`Floats`** — the runtime `Value` gains a `float64` arm.  Arithmetic
  stays on the exact int64 path while every operand is an integer and
  promotes to `float64` once any operand is a float ("int op float ⇒
  float").  `number?` covers floats; `=` is cross-type for numbers
  (`1 == 1.0` is true, `NaN != NaN`); `<` / `>` compare numerically.
  Float display keeps a trailing `.0` on integral floats (`3.0`, not
  `3`) and prints non-finite values as `NaN` / `inf` / `-inf`.
- **`ShortCircuit`** — `and` / `or` lower to a truthy-guarded
  immediately-invoked func literal that returns the operand value
  (`a and b ⇒ b` if `a` is truthy else `a`), evaluating the left side
  exactly once.
- **`MutableBindings`** — `Assign` re-binds an already-declared name.
  Go has no const/mut distinction, so a Local/Param/Capture reassignment
  is just `<name> = <value>` (the matching `LetBinding`/param already
  declared the name with `:=`).  No `let mut` pre-pass is needed the way
  the Rust backend needs one.  A `Global` assignment writes through the
  runtime global store.
- **`Loops`** — maps SIR's three loop forms onto Go's native `for`:
  - `While { cond, body }` → `for _sir_truthy(<cond>) { <body> }`
    (Go's `for` is its `while`; the test routes through SIR truthiness).
  - `ForRange { var, start, stop, step, body }` → a native three-clause
    `for`.  `stop`/`step` are cached **once** into `int64` temporaries
    (re-evaluating Python's `range` bounds each turn would be wrong);
    the continue test is direction-aware via `_sir_range_cont`, so a
    negative `step` counts down.  `var` is re-bound each iteration as a
    fresh `Value(int64(...))`.
  - `ForEach { var, iter, body }` → `for _, <var> := range _sir_seq_iter(<iter>)`.
    The runtime `_sir_seq_iter` flattens a cons-list (a `Pair`-chain
    ending in `nil`) into a `[]Value` (Sequences land in a later PR, so
    a "sequence" is still the classic cons-list).
  Loop bodies emit in statement context: a body's trailing non-`nil`
  value becomes `_ = <value>` so side effects still fire, and every
  introduced loop variable gets a `_ = <var>` guard so Go's strict
  unused-variable rule never rejects a body that ignores it.
- **`Sequences`** — the runtime gains a pointer-backed `*Seq`
  (`Seq{ Items []Value }`) with **shared mutable** semantics: a `SeqSet`
  (`xs[i] = v`) mutates the very sequence the caller holds, and aliasing
  bindings observe the write (Python-list / JS-array reference
  semantics).  `SeqLit` → `_sir_seq_lit`, `SeqIndex` → `_sir_seq_index`,
  `SeqLen` → `_sir_seq_len`, `SeqSet` → `_sir_seq_set`.  Indexing is
  strict — out-of-range reads/writes panic.  `ForEach` over a `SeqLit`
  works end-to-end: `_sir_seq_iter` now snapshots a `*Seq` as well as
  walking a cons-list.  Display: `[1, 2, 3]`.
- **`Maps`** — the runtime gains a pointer-backed `*Map`
  (`Map{ Entries []MapEntry }`), an *insertion-ordered* association list.
  Go's native `map` can't key on an arbitrary `Value`, so keys are
  compared with the runtime's structural equality (`_sir_value_eq`, a
  linear scan — shared by `=`).  A missing key reads as `nil`.
  `MapLit` → `_sir_map_lit` (keys/values emitted as two parallel slices),
  `MapGet` → `_sir_map_get`, `MapSet` → `_sir_map_set` (insert appends in
  order; existing key overwrites in place).  Display: `{a: 1, b: 2}`.

With all six SIR16 features wired up, `accepts_features` is in lockstep
with emit — every declared feature has a real (non-panicking) emit path.

### SIR19 — default parameters (`DefaultParams`)

Go has no native optional/default parameters and emitted functions are
*fixed-arity* over `Value`, so default parameters use a **runtime-mimic**
strategy built on a unique MISSING sentinel:

- **Runtime sentinel.** A distinct `_missingMarker` struct and a single
  shared `var _sir_missing Value = &_missingMarker{}`, with an exact
  `_sir_is_missing(v Value) bool` (pointer-identity) predicate.  A program
  cannot construct one (no IR node lowers to it).  `_sir_format` /
  `_sir_value_eq` guard it defensively (a stray sentinel prints as
  `<missing>`), so it never masquerades as a user value.
- **Caller padding.** A `DirectCall` that omits trailing defaulted arguments
  pads up to the callee's full param count (read from the module's function
  table) with `_sir_missing` — `f(5)` for `f(a, b = …)` emits
  `f(Value(int64(5)), _sir_missing)`.
- **Callee prologue.** At the body top, each defaulted param gets a guard in
  declaration order: `if _sir_is_missing(b) { b = <default expr> }`.  The
  default runs **call-time, in param-scope** — a later default sees the
  *earlier* params already bound (`def f(a, b = a + 1)` ⇒ `f(5)` yields
  `b == 6`).  Reassigning a parameter is ordinary mutable-local Go, and the
  guard "uses" the param, so Go's strict unused-variable rule is satisfied.

### KW6 — keyword parameters & arguments (`KeywordParams`)

Go has **no** native keyword arguments, so — like the Rust backend — this
backend resolves keywords to positions **statically at emit time** (a
`DirectCall`'s callee signature is known), producing a plain positional Go
call. No runtime library is added; the SIR19 sentinel/prologue machinery
above is reused unchanged.

- **Def side.** A `ParamKind::Keyword` param emits as an ordinary positional
  Go parameter in declared order. An *optional* keyword (`Keyword` +
  `default: Some`) fills its default via the same prologue guard as a
  positional default.
- **Call side.** For each callee param slot in declared order: a leading
  positional arg fills it; a `KeywordArg` whose name matches fills it
  (regardless of source order); an omitted *optional* slot is padded with
  `_sir_missing` (the prologue supplies the default). A *required* keyword
  left out is a validation error, so it never reaches emit.

Worked example — `def greet(greeting:, name: "world")`:

```text
  greet(greeting: "hi")              →  greet("hi", _sir_missing)   // name → "world"
  greet(greeting: "hi", name: "ada") →  greet("hi", "ada")
  greet(name: "ada", greeting: "hi") →  greet("hi", "ada")         // matched by name
```

Indirect/closure keyword calls are **deferred** (spec §Out of scope): the
callee signature is not statically known, and the frontends do not emit them.

### C5 — collection-method dispatch (runtime catalog)

`recv.meth(args…)` reaches the backend as
`BuiltinCall("__method__", [recv, StrLit("meth"), …args])` (receiver at
`args[0]`, method name always a `StrLit` at `args[1]`, an optional trailing
block surviving as a `MakeClosure`).  Go has no native method dispatch, so —
like the Python/TS backends' `sir-runtime-oop` — the backend ships an **inlined
runtime catalog** and emits every dispatch to it:

```text
  [1, 2, 3].map { |x| x * 2 }
    → _sir_call_method(<seq>, "map", []Value{_sir_make_closure(…)})
    → [2, 4, 6]
```

- **Emit.**  `emit_builtin_call` has a `"__method__"` case producing
  `_sir_call_method(recv, "name", []Value{…args})`.  A trailing block rides in
  as the last `[]Value` element; a `&:sym` / `&proc` block-pass that survives on
  the dispatch is converted (`_sir_sym_to_proc(intern("sym"))` for `&:sym`, the
  proc verbatim otherwise).  A `Const`-scoped class operand on a class predicate
  (`x.is_a?(Integer)`) is passed as its name string.
- **Runtime catalog** (`_sir_call_method`).  An **explicit type-switch +
  method-name switch** over the receiver — Array (`*Seq`), Hash (`*Map`),
  String, Numeric (`int64`/`float64`), Symbol — **ported from the Python/TS
  reference for behavioural parity** (same names, same semantics).  Block-taking
  methods (`each`/`map`/`select`/`reduce`/`times`/…) detect a trailing `*Closure`
  and apply it via `_sir_apply`; `Symbol#to_proc` (`_sir_sym_to_proc`) makes
  `map(&:to_s)` behave exactly like `map { |x| x.to_s }`.
- **Security — the catalog is the allowlist.**  Dispatch is **only** through the
  explicit switches; there is **no reflection** on the raw method name and no
  dynamic Go method/field lookup.  An unknown method on a known receiver hits
  `_sir_method_unknown`, which panics with a controlled
  `undefined method '<name>' for <Class>` — a surfaced runtime error, never
  arbitrary behaviour (the C3 RCE lesson).
- **No new feature gate.**  A pure collection-method module observes no
  `Feature::MethodDispatch` (the validator marks nothing for `__method__`), so
  it carries only its receiver/argument features — all already accepted — and is
  accepted **without dragging in class semantics**.  (`Feature::Classes` is
  accepted post-E3 only for exception subclasses — see below — never for general
  OOP, which `check_exception_soundness` still rejects.)  The runtime catalog is
  the real gate.

Catalog coverage (v0): **Array** `length`/`size`/`count`, `first`, `last`,
`empty?`, `include?`, `index`, `push`/`append`, `<<`, `pop`, `shift`, `reverse`,
`sort`, `join`, `to_a`, `each`, `map`/`collect`, `select`/`filter`, `reject`,
`reduce`/`inject`, `find`/`detect`, `any?`, `all?`, `none?`; **Hash** `keys`,
`values`, `has_key?`/`key?`/`include?`/`member?`, `has_value?`/`value?`, `size`/
`length`, `empty?`, `each`/`each_pair`, `map`, `select`/`filter`, `reject`;
**String** `length`/`size`, `upcase`, `downcase`, `reverse`, `strip`/`lstrip`/
`rstrip`, `empty?`, `include?`, `start_with?`, `end_with?`, `split`, `chars`,
`to_i`, `to_f`, `to_sym`; **Numeric** `abs`, `to_i`, `to_f`, `even?`, `odd?`,
`zero?`, `positive?`, `negative?`, `succ`/`next`, `pred`, `round` (with optional
`ndigits`), `divmod`, `fdiv`, `clamp`, `between?`, `times`; **Symbol**
`to_s`, `to_sym`, `length`/`size`, `upcase`, `downcase`, `empty?`; **universal**
`nil?`, `==`, `!=`, `class`, `to_s`, `itself`.

### E3 — exception handling (`Exceptions`, panic/recover)

Go has **no native try/catch** — it unwinds with `panic` and a deferred
`recover`.  The backend maps SIR's `begin/rescue/ensure` (`Stmt::TryCatch`) onto
an **immediately-invoked func** whose deferred closure recovers the panic and
dispatches to the matching rescue clause; `raise` maps onto `panic`:

```go
func() {
  defer func() { <ensure body> }()             // registered FIRST ⇒ runs LAST
  defer func() {
    if r := recover(); r != nil {
      if _sir_rescue_matches(r, []string{"ArgumentError"}) { e := _sir_exc_value(r); /* body */ } else
      if _sir_rescue_matches(r, []string{"TypeError"}) { /* body */ } else { panic(r) } // re-raise
    }
  }()                                          // registered SECOND ⇒ runs FIRST
  /* try body — may panic(_sir_new_error("ArgumentError", "msg")) */
}()
```

- **`raise Foo, "m"`** → `panic(_sir_new_error("Foo", <msg>))`; **`raise "m"`** →
  an implicit `RuntimeError`; **bare `raise`** → a generic `RuntimeError` (SIR v0
  does not thread the in-flight exception, matching the TS/Python backends).
- **`ensure` ordering.**  Deferred funcs run **LIFO**, and `ensure` must run
  LAST on every path — so its `defer` is registered **first** (deferred earliest
  ⇒ runs last).  The recover/dispatch defer, registered second, runs first; when
  no clause matches it re-`panic`s, which still unwinds through the ensure defer,
  so `ensure` runs on the propagating path too.
- **Ancestry (typed `rescue`).**  `_sir_rescue_matches` walks a built-in Ruby
  ancestry table (`StandardError → Exception`, `ArgumentError`/`TypeError`/… →
  `StandardError`, `NoMethodError → NameError`, `KeyError → IndexError`, …),
  ported from the TS/Python `sir-runtime-exceptions` reference for parity.  A
  bare `rescue` (empty class list) is catch-all; `Exception` matches anything.
- **User subclasses.**  A `class MyErr < StandardError` contributes one
  `subclass → superclass` edge; all such edges are collected and registered
  **once at program init** via `_sir_register_ancestry`, so `rescue StandardError`
  catches a raised `MyErr`.
- **Security.**  Rescue matching is an **explicit string-map lookup**, never
  reflection on a Go type name; the ancestry walk carries a `seen` set so a
  cyclic user hierarchy (`class A<B; class B<A`) terminates.  The
  `_sir_ancestry` table this builds is **shared** with the O4 OOP class
  hierarchy below — one hierarchy for the whole runtime.

## O4 — user-defined class OOP (`InstanceVars` / `ClassVars` + `Classes`)

The backend EXECUTES real user-defined classes — method dispatch, `Foo.new`,
`self`, `super`, `@ivar`/`@@cvar` — not just exception subclasses.  Because the
Ruby frontend HOISTS every method to a detached top-level function, the
method↔class association is recovered at **runtime** with explicit tables.

- **Instances.**  `SirInstance { Class string; Ivars map[string]Value }`; an
  `@ivar` reads/writes the *current self* (a single-threaded self-stack top,
  with a default-self so top-level `@x` never panics).  This is the documented
  v0 model; true per-object/per-thread `self` is out of scope for v0.
- **Method tables.**  Instance and class methods live in `map[string]Value`
  keyed by a NUL-joined `class + "\x00" + method` string, populated by emitted
  `__def_method__` / `__def_class_method__` registrations.  The stored value is
  the hoisted top-level function as a `*Closure`, invoked via `_sir_apply`.
- **`new` / `super` / `self`.**  `Foo.new(args)` (`__new__`) allocates, pushes
  self, resolves an inherited `initialize` (walking the shared `_sir_ancestry`,
  seen-guarded), applies it, and pops self via `defer`; `super` (`__super__`)
  walks from the superclass with the **current** self still bound; `self`
  (`__self__`) returns the self-stack top.
- **Dispatch.**  `recv.m(args)` reaches `_sir_call_method`; a `*SirInstance`
  receiver resolves the user table walking ancestry (a miss falls through to the
  universal Object methods, else the NoMethodError floor).  Non-instance
  receivers reach the C5 collection catalog **unchanged**.
- **Security.**  Dispatch is an **explicit `(class, method)` map lookup** —
  never Go `reflect`/`MethodByName` on a source-derived name.  A class/method
  named `constructor`/`__proto__` is just a map key (a miss → the clean
  NoMethodError floor).  Ancestry walks are cycle-guarded; self-stack pops go
  through `defer` so a panic still unwinds.  `Feature::Modules` stays
  **unaccepted** (no mixin/MRO runtime in v0), and a general `Const` used as a
  value / a `Const` assignment / a `ModuleDef` are still rejected cleanly by the
  soundness gate — the widened acceptance never admits those.

## Value model

```go
type Value interface{}
type Symbol   struct { Name string }
type Pair     struct { Car, Cdr Value }
type Closure  struct { Fn func(args []Value) Value }
type Seq      struct { Items []Value }              // held by *Seq (shared, mutable)
type MapEntry struct { Key, Val Value }
type Map      struct { Entries []MapEntry }         // held by *Map (insertion-ordered assoc list)
```

Single-threaded; symbol interning + globals in module-level maps.
`*Seq` / `*Map` give sequences and maps reference (shared-mutable)
semantics; maps key on structural value-equality (`_sir_value_eq`).

## Block-as-expression

Go has no expression-position blocks, so non-trivial `Block`s lower
to an immediately-invoked function expression:

```go
func() Value {
    x := someExpr
    return _sir_plus([]Value{x, intLit2})
}()
```

## `main` collision

SIR's synthesised `main` is renamed to `_sir_user_main`; the emitter
generates the real `func main()` that calls `_init()` (if present)
then `_sir_user_main()`.

## Related crates

- [`semantic-ir`](../semantic-ir/) — the IR
- [`twig-to-semantic-ir`](../twig-to-semantic-ir/) — first frontend
- Sister backends: [`semantic-ir-to-typescript`](../semantic-ir-to-typescript/),
  [`semantic-ir-to-rust`](../semantic-ir-to-rust/),
  [`semantic-ir-to-python`](../semantic-ir-to-python/)
