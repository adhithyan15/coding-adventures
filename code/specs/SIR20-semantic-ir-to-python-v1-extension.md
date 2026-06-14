# SIR20 — `semantic-ir-to-python` extension for SIR v1

## Status

Extension of [SIR14](SIR14-semantic-ir-to-python.md).  Updates the
`semantic-ir-to-python` crate to handle the new node kinds introduced
in [SIR16](SIR16-ir-extensions-for-python-and-javascript.md) (loops,
sequences, maps, mutation, floats, short-circuit) so JavaScript →
SIR → Python runs end-to-end.

## Pipeline

```text
semantic_ir::Module  (SIR v1, with new node kinds)
   │
   ▼  semantic_ir_to_python::compile
Artifact { filename, source: <Python 3 source>, metadata }
```

The crate's *public API* doesn't change — `compile(&module)` still
returns an `Artifact` with a self-contained `.py` file.  Only the
internal `emit_expr` / `emit_stmt` get new arms.

## Capability declaration

`accepts_features()` is extended to include the full SIR-v1 set:

```rust
const ACCEPTED_FEATURES: &[Feature] = &[
    // SIR-v0 (unchanged)
    Feature::Closures,
    Feature::Pairs,
    Feature::Symbols,
    Feature::Strings,
    Feature::DynamicTyping,
    Feature::OptionalTypeAnnotations,
    Feature::MutualRecursion,
    Feature::Globals,
    // SIR-v1 (new)
    Feature::Floats,
    Feature::MutableBindings,
    Feature::Loops,
    Feature::Sequences,
    Feature::Maps,
    Feature::ShortCircuit,
];
```

Rejects: `TailCalls`, `Intrinsics` (unchanged).

## Per-node lowering (new arms)

| SIR node                            | Emitted Python                                                |
|-------------------------------------|---------------------------------------------------------------|
| `Expr::FloatLit { value }`          | `<value>` with explicit decimal (e.g. `3.14`)                  |
| `Stmt::Assign { name, value, .. }`  | `<name> = <value>`                                            |
| `Stmt::While { cond, body }`        | `while _sir_truthy(<cond>): <body>`                           |
| `Stmt::ForRange { var, start, stop, step, body }` | `for <var> in range(<start>, <stop>, <step>): <body>` |
| `Stmt::ForEach { var, iter, body }` | `for <var> in <iter>: <body>`                                 |
| `Stmt::SeqSet { seq, index, value }`| `<seq>[<index>] = <value>`                                    |
| `Stmt::MapSet { map, key, value }`  | `<map>[<key>] = <value>`                                      |
| `Expr::SeqLit { items }`            | `[<item1>, <item2>, ...]`                                     |
| `Expr::SeqIndex { seq, index }`     | `<seq>[<index>]`                                              |
| `Expr::SeqLen { seq }`              | `len(<seq>)`                                                  |
| `Expr::MapLit { entries }`          | `{<k1>: <v1>, <k2>: <v2>, ...}`                               |
| `Expr::MapGet { map, key }`         | `<map>[<key>]`                                                |
| `Expr::LogicalAnd { lhs, rhs }`     | `(<lhs> and <rhs>)`                                           |
| `Expr::LogicalOr { lhs, rhs }`      | `(<lhs> or <rhs>)`                                            |

### Builtin specialisations (new)

| Builtin              | Specialised Python                              |
|----------------------|------------------------------------------------|
| `!=`                 | `(a != b)`                                     |
| `<=`                 | `(a <= b)`                                     |
| `>=`                 | `(a >= b)`                                     |
| `%`                  | `(a % b)`                                       |
| `not`                | `(not a)`                                       |
| `neg`                | `(-a)`                                         |
| `range` (1/2/3 args) | `range(...)`                                    |
| `len`                | `len(...)`                                      |
| `str` / `int` / `float` / `bool` | `str(...)` / `int(...)` / `float(...)` / `bool(...)` |

## Block-as-expression strategy (revisited)

SIR14 used the walrus-tuple strategy for non-trivial blocks:
`((x := 1), (y := x + 2), final)[-1]`.

This still works for blocks containing only `LetBinding` / `LetStarBinding`
and `ExprStmt`.  But new statement kinds (`Assign`, `While`, `ForRange`,
`ForEach`, `SeqSet`, `MapSet`) **cannot** be expressed as
assignment-expressions in Python — they are statements only.

Strategy update: when a block contains any of the new statement kinds,
the emitter lifts the block to a **nested function** (declared at the
nearest enclosing function-statement boundary) and calls it inline:

```python
def __block_42():
    x = 1
    while x < 10:
        x = x + 1
    return x
__block_42()
```

The lifted def is hoisted to the start of the enclosing user function
(or, for top-level blocks, to module scope before the call site).

Trivial blocks (no statements) continue to render inline.  Walrus-only
blocks (LetBindings / LetStarBindings only) continue to use the
walrus-tuple strategy.

This keeps generated Python idiomatic where possible and falls back
to a clear def-and-call pattern when the statement set requires it.

## Runtime updates

The inlined Python runtime (per SIR14) gets a few additions:

```python
def _sir_truthy(v):
    return v is not False and v is not None and v != 0 and v != "" and v != [] and v != {}
```

Wait — Python's native truthiness already matches what we want
(0/""/[]/{}/None/False are all falsy).  So `_sir_truthy(v)` is just
`bool(v)`.  We keep `_sir_truthy` as a thin alias for consistency with
other backends.

No other runtime changes needed — Python's native `for`/`while`/list/dict
operations handle the new IR shapes directly.

## Tests

Existing SIR14 tests continue to pass (no regressions).  New tests:

- Each new node-kind lowering rule has a positive unit test.
- End-to-end golden + execution tests for JS → SIR → Python:
  - factorial via mutation + while
  - list-sum via for-each
  - dict access via MapGet
  - closure adder (already covered by SIR14)
- Block-lifting strategy: at least one test verifying that a block
  containing `While` lowers to a nested-def call rather than walrus.

When `python3` is on PATH, end-to-end execution + stdout comparison.

## Out of scope

- Type hints on parameters / return types (Python has them; SIR has
  the carrier but the v0 backend ignores them).
- async / await
- Comprehensions in emitted code (always emit explicit loops).
- Pretty-printing optimisation passes (e.g. collapsing trivial blocks).
