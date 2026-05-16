# SIR14 — Semantic IR → Python (Rust backend)

## Status

Third backend for the narrow-waist Semantic IR
([SIR10](SIR10-narrow-waist-semantic-ir.md)).  Joins SIR12 (TypeScript)
and SIR13 (Rust).  Implemented as the Rust crate
[`semantic-ir-to-python`](../packages/rust/semantic-ir-to-python/).

The crate consumes a [`semantic_ir::Module`] and emits a **self-
contained** Python 3 source file — no external `pip` packages
required; all runtime helpers live in module-local definitions.

## Overview

```text
semantic_ir::Module
   │
   ▼  semantic_ir_to_python::compile
Artifact { filename, source, metadata }
        │
        ▼ source is a single .py file
        │
        ▼ runs on stock CPython 3.10+
        ▼ no external dependencies
```

## Public API

```rust
use semantic_ir::{Backend, Module};

pub struct PythonBackend;
impl PythonBackend { pub fn new() -> Self; }
impl Backend for PythonBackend {
    fn target_tag(&self) -> &'static str { "python" }
    ...
}

pub fn compile(module: &Module) -> Result<Artifact, BackendError>;
```

## Capability declaration (v0)

Accepts: `Closures`, `Pairs`, `Symbols`, `Strings`, `DynamicTyping`,
`OptionalTypeAnnotations`, `MutualRecursion`, `Globals`.

Rejects: `TailCalls` (CPython has no TCO), `Intrinsics` (empty
whitelist).

## Value model

Python is dynamically typed, so SIR values map naturally:

| SIR type      | Python representation              |
|---------------|------------------------------------|
| `Int`         | `int`                              |
| `Bool`        | `bool`                             |
| `Nil`         | `None`                             |
| `Symbol`      | a `Symbol` class (interned)        |
| `Str`         | `str`                              |
| `Pair`        | a `Pair` class with `car` / `cdr`  |
| `Closure`     | a `Closure` class wrapping a callable |

Globals live in a module-level `_globals: dict[str, Any]` populated
by the synthesised `_init()` function.

## Per-node lowering rules

| SIR node                    | Emitted Python                                   |
|-----------------------------|--------------------------------------------------|
| `IntLit { value }`          | `<value>`                                        |
| `BoolLit { value }`         | `True` / `False`                                 |
| `NilLit`                    | `None`                                           |
| `SymLit { name }`           | `_sir_intern("<name>")`                          |
| `StrLit { value }`          | `"<escaped>"`                                    |
| `VarRef { name, Local }`    | `<name>`                                         |
| `VarRef { name, Param }`    | `<name>`                                         |
| `VarRef { name, Capture }`  | `<name>` (Python closures capture by lexical ref)|
| `VarRef { name, Global }`   | `_globals["<name>"]`                             |
| `VarRef { name, Builtin }`  | `_sir_builtins["<name>"]`                        |
| `If { cond, then, else }`   | `(<then> if _sir_truthy(<cond>) else <else>)`    |
| `Block { stmts, value }`    | Lifted into a nested `def` if stmts non-empty; else direct expr |
| `LetBinding`                | `<name> = <value>` (inside the block-def)         |
| `LetStarBinding`            | `<name> = <value>`                               |
| `DirectCall { fn, args }`   | `<fn>(<args>)`                                   |
| `IndirectCall { tgt, args}` | `_sir_apply(<tgt>, [<args>])`                    |
| `BuiltinCall { name, args}` | `_sir_<helper>(<args>)`                          |
| `MakeClosure { fn, caps }`  | `_sir_make_closure(<fn>, [<cap_values>])`        |

### Block-as-expression strategy

Python distinguishes expressions and statements; `LetBinding`s are
statements, but SIR `Block`s appear in expression positions.  The
emitter lifts each non-trivial Block into a fresh nested `def`:

```python
def __block_42():
    x = 1
    y = 2
    return x + y
__block_42()
```

For trivial blocks (no statements), the value expression renders
inline.

## Closure ABI

Synthesised SIR functions take captures-first then params.  In
Python:

```python
def __lambda_0(captured_n, x):
    return _sir_plus(x, captured_n)
```

The `MakeClosure` lowering produces a Python lambda that prepends
captures to the runtime args:

```python
_sir_make_closure(__lambda_0, [captured_n_value])
```

`_sir_make_closure` is a runtime helper that returns a `Closure`
wrapping `lambda *args: fn(*captures, *args)`.

## Output formatting

- Indentation: 4 spaces (PEP 8).
- Line endings: `\n`.
- Generated file ends with a trailing newline.
- Function-span comments emitted before each `def`.
- `sanitize_comment` strips line terminators per the SIR12 / SIR13
  defence.

## Tests

`cargo test -p semantic-ir-to-python` covers per-node lowering,
identifier sanitisation, deterministic output, and end-to-end
Twig→SIR→Python pipelines.  Where `python3` is available on the
CI machine, the emitted source is also executed and stdout
compared against expected.

## Out of scope (deferred)

- Type-hint enrichment (`def foo(x: int) -> int`)
- Source maps
- Raw-Python intrinsic injection
- `async def` / `await`
