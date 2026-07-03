# SIR22 — Array/matrix IR extension (numeric-array math languages)

## Motivation

[`SIR10`](SIR10-narrow-waist-semantic-ir.md) has no numeric-array vocabulary:
no N-D array type, no matrix multiply, no ranges, no MATLAB-style indexing.
This spec adds that vocabulary, additively — exactly the discipline
[`SIR16`](SIR16-ir-extensions-for-python-and-javascript.md) used to add loops
and sequences for Python/JS without touching Twig. Every SIR10/SIR16 module
remains valid; this spec only adds new `Expr` variants, `SirType` variants,
and `Feature` flags.

This is Stream A of [`HML01`](HML01-math-to-semantic-ir.md) — the substrate
for `matlab-to-semantic-ir` and `octave-to-semantic-ir` today, and for every
future array-family frontend (APL, J, K/Q, Scilab, IDL) per
[`HML00`](HML00-historical-math-languages-roadmap.md).

Every new node kind is deliberately mapped **1:1** onto an existing
`array_runtime::execute()` op shape (see `array-runtime`'s public API), so
that lowering a MATLAB parse tree into these nodes is mechanical — the
frontend's job is picking the right node, not inventing new semantics.

## Scope

**In scope:**

- Dense N-D numeric arrays (the MATLAB/Octave "everything is a matrix" model)
- Matrix literals (`[1 2; 3 4]`), ranges (`1:5`, `0:2:10`)
- Matrix ops: matmul, elementwise arithmetic, transpose (plain + conjugate)
- 1-based-at-the-frontend / 0-based-in-the-IR indexing (get and set),
  including whole-row/column (`:`) and `end`-relative forms — all resolved to
  concrete `IndexGet`/`IndexSet` nodes by the frontend
- Exact rational and complex scalars (`SirType::Rational`, `SirType::Complex`)
  — shared with [`SIR23`](SIR23-symbolic-pattern-semantic-ir.md), landed once

**Explicitly out of scope (deferred):**

- Sparse matrices, N-D arrays beyond what `array-runtime` already models
- GPU-specific SIR nodes — GPU dispatch stays entirely inside
  `matrix-runtime`'s cost-based planner on the Rust side of each frontend's
  own evaluation; the *SIR* only ever sees "do a matmul," never "on which
  device." A JS backend runs everything on `sir-runtime-array`'s CPU-only
  `Float64Array` path; that is a backend limitation, not an IR concern.
- Cell arrays, structs, classdef — same things MATLAB itself defers today
  (see [`MA01`](MA01-matlab-language.md) §2)

## New `SirType` variants

```text
SirType::NDArray { elem: Box<SirType>, rank: Option<usize> }
    -- rank: None means "unknown/dynamic rank" (a frontend that can't prove
       rank statically leaves it absent; backends must handle the absent
       case, they never infer it)
SirType::Rational   -- exact numerator/denominator pair, arbitrary precision
                        left to the backend's runtime (shared with SIR23)
SirType::Complex     -- { re: f64, im: f64 } pair (shared with SIR23)
```

## New `Expr` variants

All new variants carry `span` like every existing node.

```text
ArrayLit {
    rows: Vec<Vec<Expr>>,      -- row-major in the literal syntax; the
                                  frontend's job to reconcile with the
                                  storage-convention attribute below
    span,
}

Range {
    start: Box<Expr>,
    step:  Option<Box<Expr>>,  -- None means step = 1
    stop:  Box<Expr>,
    span,
}

MatMul { lhs: Box<Expr>, rhs: Box<Expr>, span }

ElementwiseOp {
    op:   ElementwiseOpKind,   -- Add | Sub | Mul | Div | Pow
    lhs:  Box<Expr>,
    rhs:  Box<Expr>,
    span,
}

Transpose { target: Box<Expr>, conjugate: bool, span }

IndexGet {
    target:  Box<Expr>,
    indices: Vec<IndexArg>,
    span,
}

IndexSet {
    target:  Box<Expr>,
    indices: Vec<IndexArg>,
    value:   Box<Expr>,
    span,
}

IndexArg =
    | Scalar(Box<Expr>)   -- a[3] — already 0-based, frontend translated
    | Whole               -- a[:, k] — the ":" meaning "every element on
                             this axis"
    | Range(Box<Expr>)    -- a[1:5] — a Range expr reused as an index arg
```

`Transpose { conjugate: true }` is MATLAB `'`; `conjugate: false` is `.'`.
Both map onto `array_runtime::execute(Transpose, …)`; the frontend decides
which at lowering time (per the `'` transpose-vs-string lexer decision
already made in [`MA01`](MA01-matlab-language.md) §3 — that decision is
orthogonal to and unaffected by this spec).

`end`-relative indices (MATLAB `A(end)`, `A(end-1)`) are **not** a separate
IR concept — the frontend resolves `end` to a concrete expression (a call to
a `size`/`length`-equivalent builtin minus an offset) before emitting
`IndexArg::Scalar`. The IR never sees `end`; per SIR10 discipline,
disambiguation is the frontend's job.

## Storage convention

`array_runtime::Array` is column-major internally (Fortran/MATLAB order),
but that convention is currently *implicit* — baked into the Rust struct's
memory layout, invisible to any consumer that isn't `array-runtime` itself.
A JS backend owns its own representation and needs the convention stated
explicitly, so it becomes a manifest-level fact:

```text
Feature::ArrayColumnMajor
```

present in a module's manifest whenever any `ArrayLit`/matrix op appears.
`sir-runtime-array` (the JS runtime, see §"Backend" in
[`HML01`](HML01-math-to-semantic-ir.md) §4) stores its `Float64Array` buffer
column-major to match, so index arithmetic in the emitted JS is a direct
translation of the same formula `array-runtime` already uses
(`(r, c) → c * nrows + r`) rather than a silent transpose at the boundary.

## New `Feature` flags

```text
Feature::NDArrays
Feature::MatrixOps
Feature::Rationals
Feature::Complex
Feature::ArrayColumnMajor
```

A backend that doesn't declare `NDArrays`/`MatrixOps` in
`accepts_features()` cleanly rejects any module using these nodes (SIR10's
existing capability-check mechanism — no new mechanism needed).

## Effects

All new `Expr` variants are `Pure` — array construction, indexing, and
arithmetic have no observable side effects distinct from the value they
compute. `IndexSet` is the one exception with a mutation-shaped effect; it
is lowered as a `Stmt`-position operation (like `Assign`), not a value-producing
`Expr`, consistent with how SIR16's `Assign` is a `Stmt`.

## Backend impact

- **JS/TS** (`semantic-ir-to-javascript`/`-to-typescript`): new `match` arms
  emit calls into `sir-runtime-array` (`__SirArray.matmul(...)`,
  `__SirArray.elementwise(...)`, `__SirArray.range(...)`,
  `__SirArray.indexGet(...)`/`indexSet(...)`), imported only when
  `Feature::MatrixOps`/`NDArrays` is in the manifest — same gating pattern as
  the existing OOP/exception runtime imports.
- **Rust/Go/Python backends**: not required to support this in the first
  wave; they reject modules declaring `NDArrays`/`MatrixOps` per the existing
  capability-rejection path. No code changes required to these backends for
  this spec to land safely.

## Versioning

This is an additive extension within the SIR line (same discipline as
SIR16/SIR18/KW1 before it) — no existing module or backend match arm needs
to change; backends simply gain new arms or explicitly decline the new
features. Modules using SIR22 nodes bump `metadata.sir_version` to record
that fact for validators.

## References

Internal: [`HML01`](HML01-math-to-semantic-ir.md),
[`SIR10`](SIR10-narrow-waist-semantic-ir.md),
[`SIR16`](SIR16-ir-extensions-for-python-and-javascript.md) (extension
precedent), [`MA00`](MA00-array-runtime.md) (`array-runtime`'s `Array` value
model and `execute()` op shapes this spec mirrors 1:1),
[`MA01`](MA01-matlab-language.md) (the frontend that will emit these nodes).
