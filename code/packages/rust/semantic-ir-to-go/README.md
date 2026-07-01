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
