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

## Capability declaration

Accepts: `Closures`, `Pairs`, `Symbols`, `Strings`, `DynamicTyping`,
`OptionalTypeAnnotations`, `MutualRecursion`, `Globals`.

Rejects: `TailCalls` (CPython has no TCO), `Intrinsics` (empty
whitelist).

## Related crates

- [`semantic-ir`](../semantic-ir/) — the IR
- [`twig-to-semantic-ir`](../twig-to-semantic-ir/) — first frontend
- Sister backends: [`semantic-ir-to-typescript`](../semantic-ir-to-typescript/),
  [`semantic-ir-to-rust`](../semantic-ir-to-rust/)
