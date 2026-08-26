# SIR16 — IR extensions for Python ↔ JavaScript interop

## Motivation

SIR v0 ([SIR10](SIR10-narrow-waist-semantic-ir.md)) was Twig-shaped: the
node taxonomy matched what a Lisp-precursor needed (atoms, `let`/`if`/
`lambda`/`apply`, `cons`/`car`/`cdr`).  That surface is *insufficient*
for mainstream dynamic languages like Python and JavaScript, which
depend on:

- **Loops** (`while`, `for x in range`, `for x in items`)
- **Mutable bindings** (`x = 1; x = x + 1`)
- **Sequence types** with literal syntax (`[1, 2, 3]`, `["a", "b"]`)
- **Map types** with literal syntax (`{"a": 1, "b": 2}`)
- **Floating-point numbers**
- **Short-circuit logical operators** (`and`/`or` / `&&`/`||`)

Both languages are **loose / dynamic** (no type inference required) and
sit on the same side of the strict-loose axis, so a clean bidirectional
translation is achievable via the SIR — the easiest cross-language
direction the architecture can support.

This spec lists the additions to the IR needed to make
Python ↔ JavaScript work for a focused MVP subset.  The additions are
**purely additive** — every v0 module remains a valid module under the
extended IR; the extended IR adds new node kinds, new `SirType`
variants, new manifest features, and new builtins.

## Scope

This is the **MVP subset**, deliberately small:

**In scope:**

- Integer + float + string + boolean + nil literals
- Variable binding (declare-and-assign) and mutation (reassign)
- Arithmetic: `+`, `-`, `*`, `/`, `%`, unary `-`
- Comparison: `=`, `!=`, `<`, `>`, `<=`, `>=`
- Logical: `and`/`or`/`not` with short-circuit semantics
- `if`/`else` (chained `elif` lowers to nested `if`)
- `while` loops
- `for` over an integer range (`for x in range(...)`)
- `for` over a sequence (`for x in some_list`)
- Function definitions, calls, return, closures
- Anonymous functions (Python `lambda`, JS arrow `(x) => ...`)
- Sequence literals + indexing + length: `[1,2,3]`, `xs[0]`, `len(xs)`
- Map literals + indexing + set: `{"k": v}`, `d["k"]`, `d["k"] = v`
- `print(x)` (lowered to native print on each target)
- String concatenation via `+`

**Explicitly out of scope (deferred to future SIR versions):**

- Classes / `class` declarations / inheritance / methods on user types
- Exceptions (`try` / `except` / `raise` / `throw` / `catch`)
- Generators / `yield` / iterators (beyond `for-in`)
- `async` / `await`
- Comprehensions (list / dict / set / generator)
- Decorators
- Multiple assignment / destructuring (`a, b = 1, 2`; `[a, b] = arr`)
- Slicing (`xs[1:3]`)
- String methods (`.split`, `.join`, `.replace`)
- Modules / `import` (multi-file)
- Spread / rest (`*args`, `**kwargs`, `...rest`)
- Default + keyword arguments
- `with` / `using` statements
- Set type, tuple type (use sequence as proxy)

The MVP is sufficient for non-trivial programs: factorial, fibonacci,
recursion, basic list/dict manipulation, simple algorithms.  It's not
sufficient for "real" Python or JS codebases.

## New SIR node kinds

Added to the `Expr` enum and `Stmt` enum in `semantic-ir::nodes`:

### Atoms

```text
Expr::FloatLit { value: f64, span }
```

### Mutation

```text
Stmt::Assign { name: String, scope: Scope, value: Expr, span }
```

`Assign` is a *re-binding* — the named variable must already exist in
the surrounding scope.  Frontends use `LetBinding` for first-occurrence
binding and `Assign` for subsequent reassignments.  Backends handle the
distinction natively:

- Python: both lower to `name = value` (Python has no declaration form)
- JavaScript: `LetBinding` emits `let name = value`, `Assign` emits `name = value`
- Rust: `LetBinding` emits `let mut name = value`, `Assign` emits `name = value`
- Go: `LetBinding` emits `name := value`, `Assign` emits `name = value`
- TypeScript: like JavaScript, with the SIR-typed `__Sir.Val` annotation

`scope` on `Assign` matches the same `Scope` enum used by `VarRef` — the
frontend has already committed to whether `name` is local, param,
capture, or global.

### Looping

```text
Stmt::While { cond: Expr, body: Block, span }

Stmt::ForRange {
    var: String,      // loop variable name (binding fresh in body)
    start: Expr,
    stop: Expr,       // exclusive upper bound
    step: Expr,       // typically IntLit(1)
    body: Block,
    span: Span,
}

Stmt::ForEach {
    var: String,
    iter: Expr,       // expression yielding a Seq
    body: Block,
    span: Span,
}
```

The `var` in `ForRange` / `ForEach` is a fresh `LetBinding`-shaped
binding scoped to the body.  Backends lower:

- Python `ForRange`: `for var in range(start, stop, step): body`
- Python `ForEach`: `for var in iter: body`
- JavaScript `ForRange`: `for (let var = start; var < stop; var += step) { body }`
- JavaScript `ForEach`: `for (const var of iter) { body }`
- Rust `ForRange`: `for var in (start..stop).step_by(step as usize) { body }`
- Rust `ForEach`: `for var in iter.iter() { body }`
- Go `ForRange`: `for var := start; var < stop; var += step { body }`
- Go `ForEach`: `for _, var := range iter { body }`

Loops are *statements*, not expressions — they have no value.  The
frontend uses an explicit `NilLit` as the value of a block whose final
content is a loop.

### Loop control (addendum)

```text
Stmt::Break { span: Span }
Stmt::Continue { span: Span }
```

Bare (unlabeled) only — SIR v0 has no loop-label vocabulary, so a source
language's labeled `break`/`continue` (Java `break outer;`) has no
lowering yet; a frontend encountering one rejects cleanly rather than
mis-targeting the wrong loop. Gated by `Feature::LoopControl`, deliberately
split from `Feature::Loops` (every existing backend already declares
`Loops` for plain `While`/`ForRange`/`ForEach`, and none of them implement
`break`/`continue` emission yet — folding this into `Loops` would either
force every backend to gain, or fake, that support overnight, or silently
redefine what `Loops` has always promised).

**Validator enforcement**: a `Break`/`Continue` with no enclosing loop at
all is an error (`"break"/"continue" outside a loop`). More subtly, one
whose *nearest* enclosing loop is a `ForRange` is *also* an error, even
though `ForRange` shares `Feature::Loops` with `While`/`ForEach` — this
restriction exists because, as implemented today, several backends lower
`ForRange` to a `while` loop with the step increment appended *after* the
body (so a bare `continue` would skip it — `semantic-ir-to-{c,rust,
typescript,javascript}`), while `semantic-ir-to-ruby` instead hoists the
body into a `->(){ }` lambda where a bare `break` doesn't propagate to the
enclosing loop at all (Ruby `break` inside a `lambda`, unlike inside a
`proc`/block, only returns from that lambda's own `.call`). Those two
failure modes don't even agree on which statement is unsafe, so the
validator rejects *both* uniformly whenever the nearest loop is a
`ForRange`, rather than making its own correctness depend on a specific
backend's lowering strategy. A `Break`/`Continue` nested inside a `While`/
`ForEach` that itself sits inside an outer `ForRange` is fine — only the
*nearest* enclosing loop's kind matters (`while (...) { for i in 0..10 {
break; } }`-shaped nesting is unaffected).

Only the syntactic nearest-enclosing-loop matters, not a lexically-outer
one across a statement-flow boundary: a `Break`/`Continue` inside a
`Stmt::MethodDef` body or a `MakeClosure`-hoisted top-level `Function`
never resolves against a loop the *declaration* happens to be nested in —
each of those bodies is validated with its own fresh loop-tracking state.

**Backend status (as of this addendum)**: no backend accepts
`Feature::LoopControl` yet — this addendum lands the IR/validator surface
only (mirrors SIR29's own Slice 0), so every backend's `ACCEPTED_FEATURES`
list omits it and a module using `Break`/`Continue` is cleanly rejected at
the capability check on every target. Real per-backend emission (a
mechanical `break;`/`continue;` on every language surveyed here, all of
which have a native equivalent) is a follow-up, per-backend, once a real
consumer frontend exists.

### Sequences

```text
Expr::SeqLit { items: Vec<Expr>, span }
Expr::SeqIndex { seq: Box<Expr>, index: Box<Expr>, span }
Expr::SeqLen { seq: Box<Expr>, span }
Stmt::SeqSet {
    seq: Expr,
    index: Expr,
    value: Expr,
    span: Span,
}
```

Sequences are 0-indexed everywhere.  Length returns a non-negative
`Int`.  Out-of-bounds access is the target language's responsibility
(Python raises `IndexError`, JS returns `undefined`, Rust panics, Go
panics) — the SIR does not normalize the failure mode.

### Maps

```text
Expr::MapLit { entries: Vec<MapEntry>, span }
Expr::MapGet { map: Box<Expr>, key: Box<Expr>, span }
Stmt::MapSet {
    map: Expr,
    key: Expr,
    value: Expr,
    span: Span,
}

MapEntry { key: Expr, value: Expr }
```

Keys may be any value; v0 frontends emit only string-keyed maps but the
IR doesn't enforce that.  Missing-key behaviour follows the target
(Python `KeyError`, JS `undefined`, etc.).

### Short-circuit logical

```text
Expr::LogicalAnd { lhs: Box<Expr>, rhs: Box<Expr>, span }
Expr::LogicalOr  { lhs: Box<Expr>, rhs: Box<Expr>, span }
```

These are **distinct from `BuiltinCall("and", ...)`** because:

- `BuiltinCall` evaluates all arguments before invoking the helper —
  not short-circuit.
- Real `and`/`or` short-circuit: `lhs and rhs` returns `lhs` if `lhs` is
  falsy without evaluating `rhs`.

Both languages share the same short-circuit semantics; the IR captures
this as its own node kind so backends emit native `&&`/`||` /
Python `and`/`or` and inherit the behaviour.

Unary `not` is still a `BuiltinCall("not", arg)` — no short-circuit
needed.

### Comparison: keep using `BuiltinCall`

`<`, `>`, `<=`, `>=`, `=`, `!=` remain as `BuiltinCall` nodes with the
operator name.  Backends emit the native equivalent.  Two-argument
arity validated at lowering time.

## New `SirType` variants

```text
SirType::Float
SirType::Seq        // sequence-of-Any
SirType::Map        // map-of-Any-keys to Any-values
```

`SirType::Seq` and `SirType::Map` are intentionally non-parametric in
v1 — every sequence is `Seq<Any>` and every map is `Map<Any, Any>`.
Parametric `Seq<T>` is a future SIR version.

## New `Feature` variants

Added to `Feature`:

```text
Feature::Floats
Feature::MutableBindings
Feature::Loops
Feature::Sequences
Feature::Maps
Feature::ShortCircuit
```

Frontends declare these features in the module manifest based on what
nodes they emit.  Backends declare which they accept.

### Capability matrix (v1 release)

| Feature             | TS  | Rust | Python | Go  | JS  |
|---------------------|-----|------|--------|-----|-----|
| Floats              | ✅  | ✅   | ✅     | ✅  | ✅  |
| MutableBindings     | ✅  | ✅   | ✅     | ✅  | ✅  |
| Loops               | ✅  | ✅   | ✅     | ✅  | ✅  |
| Sequences           | ✅  | ✅   | ✅     | ✅  | ✅  |
| Maps                | ✅  | ✅   | ✅     | ✅  | ✅  |
| ShortCircuit        | ✅  | ✅   | ✅     | ✅  | ✅  |

Phase-1 implementation goal: every existing and new backend accepts
the full SIR-v1 feature set.  The "narrow waist works" claim depends
on this.

## New builtins

Added to the canonical builtin name set (matching all backends'
dispatch tables):

| Name      | Arity     | Effects       | Notes                                       |
|-----------|-----------|---------------|---------------------------------------------|
| `!=`      | 2         | Pure          | Inverse of `=`                              |
| `<=`      | 2         | Pure          | Less-or-equal                                |
| `>=`      | 2         | Pure          | Greater-or-equal                             |
| `%`       | 2         | Pure          | Modulo (truncating, language-native sign)    |
| `not`     | 1         | Pure          | Logical negation                              |
| `neg`     | 1         | Pure          | Unary minus                                   |
| `range`   | 1, 2, 3   | MayAllocate   | Sequence-producing range; arity 1 = `range(stop)`, 2 = `(start, stop)`, 3 = `(start, stop, step)` |
| `len`     | 1         | Pure          | Length of a `Seq` or `Str`                    |
| `str`     | 1         | MayAllocate   | Convert any value to its `Str` representation |
| `int`     | 1         | MayThrow      | Parse `Str` to `Int`, may fail                |
| `float`   | 1         | MayThrow      | Parse `Str` to `Float`, may fail              |
| `bool`    | 1         | Pure          | Truthiness coercion                           |

Existing v0 builtins (`+`, `-`, `*`, `/`, `=`, `<`, `>`, `cons`, `car`,
`cdr`, `null?`, `pair?`, `number?`, `symbol?`, `print`, `global_set`,
`global_get`) remain.  `print` is the unifying name across all
languages — backends map it to `console.log` / `print` / `println!` /
`fmt.Println`.

## Validation updates

The `semantic_ir::validate` function extends to enforce:

1. **`Assign`** to an undeclared name is a hard error.  Frontends must
   emit a `LetBinding` before any `Assign` on the same name in the
   same scope.
2. **`SeqLit` / `MapLit`** trigger `Feature::Sequences` / `Feature::Maps`
   observation; missing-from-manifest is an error as before.
3. **`While` / `ForRange` / `ForEach`** trigger `Feature::Loops`.
4. **`LogicalAnd` / `LogicalOr`** trigger `Feature::ShortCircuit`.
5. **`FloatLit`** triggers `Feature::Floats`.
6. **`ForRange` / `ForEach` body** opens a fresh local scope containing
   only the loop variable.
7. **Depth cap** continues to apply to recursive descent through new
   node kinds (`MAX_IR_DEPTH = 1024`).

## Backwards compatibility

A SIR v0 module (Twig-emitting frontend, no new features in manifest)
remains valid under the extended IR.  No existing tests break.

A backend that hasn't been updated for the new node kinds will fail
to compile (Rust exhaustive-match rule).  Phase 1 of the implementation
updates **all** existing backends to handle the new nodes.

## Backend pattern: new node → emit

For each backend (TS, Rust, Python, Go, JS):

- **`FloatLit`** → native float literal (`3.14`, `3.14`, `3.14`, `3.14`, `3.14`).
- **`Assign`** → native re-assignment (`x = ...`).
- **`While`** → native while loop.
- **`ForRange`** → idiomatic for-range; Python uses native `range`,
  JS / TS use C-style for, Rust uses `(a..b).step_by`, Go uses C-style.
- **`ForEach`** → native iteration.
- **`SeqLit`** → native array/list literal.
- **`SeqIndex`** → native indexing.
- **`SeqLen`** → `len(...)` / `....length` / `....len()`.
- **`SeqSet`** → native indexed assignment.
- **`MapLit`** → native object/dict.
- **`MapGet` / `MapSet`** → native indexed access / assignment.
- **`LogicalAnd` / `LogicalOr`** → native short-circuit operators.

This keeps generated code idiomatic in every target.

## Frontend pattern: language feature → SIR

Frontend lowering rules (Python and JavaScript) are detailed in
SIR17 and SIR19.  At a high level:

- Numeric literals: emit `IntLit` if integer-shaped, `FloatLit` otherwise.
- Variable binding: track first-occurrence vs re-assignment;
  first → `LetBinding`, subsequent → `Assign`.
- `for x in range(n)` → `ForRange { var: x, start: 0, stop: n, step: 1 }`.
- `for x in xs` → `ForEach { var: x, iter: xs }`.
- `while c: body` → `While { cond: c, body }`.
- `a and b` → `LogicalAnd { lhs: a, rhs: b }`.
- f-strings / template literals → desugar to sequence of `+`-concat
  in the frontend (no new IR node).

## Versioning

The SIR major version stays at `0` for now — these are additive
changes.  Existing v0 modules round-trip without modification.  A
future spec revision may bump to `1` when a fundamentally
incompatible change (e.g. removing a node kind) is needed.

## Implementation order

1. **Phase 1** — `semantic-ir` crate update + all four existing
   backends updated (TS, Rust, Python, Go).
2. **Phase 2** — `python-to-semantic-ir` crate (SIR17).
3. **Phase 3** — `semantic-ir-to-javascript` crate (SIR18).
4. **Phase 4** — `javascript-to-semantic-ir` crate (SIR19).
5. **Phase 5** — extend `semantic-ir-to-python` for full SIR-v1
   support (SIR20) — covers the JS → Python direction end-to-end.

After Phase 3, Python → SIR → JavaScript runs end-to-end.  After
Phase 5, JavaScript → SIR → Python also runs.  Bidirectional MVP
complete.

## What this does NOT promise

- Idiomatic-quality output.  Generated code is *correct* and readable,
  but a human Python programmer would write Python differently than
  what comes out of JS → SIR → Python.  Style transfer is not a goal.
- Performance parity.  The generated code has runtime overhead from
  the inlined SIR runtime per file.  A future optimization pass can
  trim this.
- Lossless round-trips.  Comments, whitespace, identifier capitalization
  and source-positioning past the function level are not preserved.
