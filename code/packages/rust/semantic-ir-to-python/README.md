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

## Type-directed operator selection (SIR21 T3c-3)

`+ - * < > <= >= == !=` are `BuiltinCall`s that, by default, lower to a
runtime-dispatch helper (`_sir_plus(x, y)` above) because SIR carries no
static type for either operand in the fully-dynamic pipeline every
frontend produces today. Before reaching that fallback, the emitter now
builds a `TypeEnv` per function and consults `semantic-ir`'s
`op_select::resolve_binary` with each operand's statically-known type
(from `TypeEnv::expr_type`, a direct `VarRef` to a declared `Param`/
`Capture`/`LetBinding` only — no inference). When both operands agree on
a concrete int/float/comparable type, the emitter specialises to native
Python infix (`(x + y)`) instead of the helper call; string concatenation
and any `Dynamic`/mismatched pair keep routing through the helper,
unchanged. No shipped frontend populates operand types in a way this
backend can reach yet (see `code/specs/SIR21-type-system-and-integer-semantics.md`),
so this is presently inert for every real program — the wiring exists so
a future typed frontend/backend slice has one correct place to plug in.

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

## SIR22 array/matrix domain

`ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/`Transpose`/`IndexGet` (+
`Stmt::IndexSet`) — the SIR22 base cut — and the nine-node "APL addendum"
(`Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/
`IndexOf`/`Ravel`/`Catenate`, which shares these same three features with
no flag of its own) all lower to calls into `_sir_array_*`, imported from
the published `coding-adventures-sir-runtime-array` pip package — **this
backend follows the TypeScript backend's imported-package model here**,
not its own usual inlined-runtime convention (contrast the OOP/
exceptions/pairs/regex/shell/range imports above, and C/Go/Rust/Ruby's
identical-slice PRs, which all inline a ported sub-runtime instead).
The import is gated by `uses_array` (any of `Feature::NDArrays`/
`MatrixOps`/`ArrayColumnMajor`), so a pure arithmetic module never gains
the dependency. `coding_adventures_sir_runtime_array`'s own `NDArray`
preserves native Python `int`/`float` propagation rather than forcing
every element to a double (`Add`/`Sub`/`Mul`/`Pow` stay `int` when both
operands are; `Div` always true-divides), matching MATLAB's `./`
semantics without a spurious `.0` on an all-integer computation — see
that package's own README/CHANGELOG for the full design and security
notes (NaN-safe AND-form bounds checks, DoS-safe shape validation before
allocation, including for every addendum function that computes an
output/allocation size from two independently-controlled operands —
`outer`, `index_of`, `catenate`, `reshape`, `index_generator`).

`Reduce`/`Scan`/`OuterProduct` carry an `ElementwiseOpKind` and reuse
`elementwise_op_py_name` exactly like `ElementwiseOp` does; the remaining
six addendum nodes have no `op` field ("bespoke, not `BinOp`-shaped") and
just recurse into their operand(s). Unlike `Stmt::IndexSet` above (which
needs separate handling in both statement position and the walrus-tuple
expression-position path, since `Stmt` has no single shared emit
function), all nine addendum nodes are `Expr`s, so one `emit_expr` match
arm per node is enough — every position already routes through that one
function.

## Capability declaration

Accepts: `Closures`, `Pairs`, `Symbols`, `Strings`, `DynamicTyping`,
`OptionalTypeAnnotations`, `MutualRecursion`, `Globals`, `DefaultParams`,
`NDArrays`, `MatrixOps`, `ArrayColumnMajor` (and the SIR16/17 expression,
mutation, loop, OOP and exception features).

**`ConsoleIO`** (SIR28): `__sys_write__("stdout"|"stderr",
"none"|"per_value"|"once", unpack_arrays, ...values)` → `_sir_write(...)`
— a plain pass-through (no compile-time literal extraction, unlike the
C/Go/Rust backends): Python branches on the `stream`/`terminator` strings
at runtime. `_sir_write` lives in `coding-adventures-sir-runtime-core` and
is now the ONLY console-output primitive the backend emits — bare
`"print"`/`"puts"` `BuiltinCall`s and the `sir_print`/`sir_puts` functions
that used to implement them were removed once every frontend finished
migrating to `__sys_write__` (SIR28 §7) — see
[SIR28](../../../specs/SIR28-syscall-primitives.md).

Rejects: `TailCalls` (CPython has no TCO), `Intrinsics` (empty
whitelist).

## Related crates

- [`semantic-ir`](../semantic-ir/) — the IR
- [`twig-to-semantic-ir`](../twig-to-semantic-ir/) — first frontend
- Sister backends: [`semantic-ir-to-typescript`](../semantic-ir-to-typescript/),
  [`semantic-ir-to-rust`](../semantic-ir-to-rust/)
