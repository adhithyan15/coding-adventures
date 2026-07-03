# semantic-ir-to-python

Third backend for the narrow-waist Semantic IR.  Lowers
[semantic-ir](../semantic-ir/) modules into **self-contained** Python 3
source code — the runtime helpers (`Symbol` / `Pair` / `Closure` classes
plus builtin implementations) are inlined into every produced `.py`
file, so the output runs on stock CPython 3.10+ with no `pip`
dependencies.

Implements [SIR14](../../../specs/SIR14-semantic-ir-to-python.md).

## Public API

```rust
use semantic_ir_to_python::{compile, PythonBackend};
use semantic_ir::Backend;

let artifact = compile(&sir_module)?;
// or:
let backend = PythonBackend::new();
let artifact = backend.compile(&sir_module)?;
```

## Block-as-expression strategy

Python distinguishes statements from expressions, but SIR `Block`s
appear in expression position.  The emitter renders non-trivial
blocks using Python 3.8+ **assignment expressions** (walrus
operator) tupled together with the result expression:

```python
((x := 1), (y := 2), _sir_plus(x, y))[-1]
```

This evaluates the bindings left-to-right and returns the tuple's
last element (the block's value).  Tested deterministically — no
codegen variance across runs.

## Default parameters

SIR default parameters are **call-time** and a default may reference an
**earlier param** (e.g. `def f(a, b = a + 1)`).  Python's *native* default
args are def-time and cannot reference other params (`def f(a, b=a)` raises
`NameError`), so native syntax is wrong for this model.  The emitter instead
uses a **sentinel + body-prologue** strategy:

- The runtime header defines `_SIR_MISSING = object()`.
- A defaulted param emits `name=_SIR_MISSING`, so callers may omit it.
- The function body opens with a resolve-prologue, in param order:

  ```python
  if name is _SIR_MISSING:
      name = <default expr>
  ```

  The default runs in the body, where earlier params are in scope — giving
  correct call-time, param-referencing semantics.  Calls emit only the
  arguments present (no padding); an omitted trailing default binds the
  sentinel, which the prologue resolves.  `IndirectCall`/closure defaults are
  deferred.

## Capability declaration

Accepts: `Closures`, `Pairs`, `Symbols`, `Strings`, `DynamicTyping`,
`OptionalTypeAnnotations`, `MutualRecursion`, `Globals`, `DefaultParams`
(and the SIR16/17 expression, mutation, loop, OOP and exception features).

Rejects: `TailCalls` (CPython has no TCO), `Intrinsics` (empty
whitelist).

## Related crates

- [`semantic-ir`](../semantic-ir/) — the IR
- [`twig-to-semantic-ir`](../twig-to-semantic-ir/) — first frontend
- Sister backends: [`semantic-ir-to-typescript`](../semantic-ir-to-typescript/),
  [`semantic-ir-to-rust`](../semantic-ir-to-rust/)
