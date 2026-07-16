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
    op:   ElementwiseOpKind,   -- Add | Sub | Mul | Div | Pow (original cut)
                                  | Max | Min | Eq | Ne | Lt | Le | Ge | Gt
                                  (APL addendum — see "New Expr variants
                                  (APL primitives)" below)
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

## New `Expr` variants (APL primitives)

This spec's own Motivation section always claimed to be the substrate for
"every future array-family frontend (APL, J, K/Q, Scilab, IDL)," but the
original cut above only populated the `Expr` variants MATLAB's own frontend
needed. MATLAB has no first-class *operator* syntax for reduce, scan,
outer product, reshape, iota, ravel, or catenate — it would call
`sum(x)`/`reshape(x, ...)`/etc. as ordinary function calls, which this
repo's MATLAB frontend subset doesn't yet support at all (only `disp` is a
recognized builtin call today). APL (`apl-runtime`/`apl-parser`, already
shipped) exposes all seven of these as first-class bare glyphs, so the
original cut left a genuine gap between what this spec always claimed to
cover and what it actually modeled. This addendum closes that gap.

Every variant below carries `span` and is `Pure`, same as every other
SIR22 node, and is mapped 1:1 onto an existing op shape — either
`array_runtime::ops::{reduce,scan,outer}` (for the three that take an
`ElementwiseOpKind`) or `apl-runtime::builtins`'s own bespoke,
non-`BinOp`-shaped logic (for the rest) — the same "map 1:1 onto an
existing op shape" discipline this spec's own Motivation states, extended
to a second source op-shape since APL's `⍴`/`⍳`/`,` don't fit the `BinOp`
shape any more than they did for `apl-runtime` itself.

```text
Reduce { op: ElementwiseOpKind, target: Box<Expr>, span }
    -- `+/A` (APL reduce): folds `target` with `op` along its one axis.
       Maps 1:1 onto `array_runtime::ops::reduce(op, a)`.

Scan { op: ElementwiseOpKind, target: Box<Expr>, span }
    -- `+\A` (APL scan): a running fold of `target` with `op`, emitting
       one result per prefix. Maps 1:1 onto `array_runtime::ops::scan(op, a)`.

OuterProduct { op: ElementwiseOpKind, lhs: Box<Expr>, rhs: Box<Expr>, span }
    -- `A∘.×B` (APL outer product): every pairwise `op` application
       between `lhs`'s and `rhs`'s elements. Maps 1:1 onto
       `array_runtime::ops::outer(op, a, b)`.

Shape { target: Box<Expr>, span }
    -- monadic `⍴A` (APL shape): the dimensions of `target` as a vector.
       Mirrors `apl-runtime::builtins::shape`.

Reshape { shape: Box<Expr>, target: Box<Expr>, span }
    -- dyadic `A⍴B` (APL reshape): reinterpret `target`'s data under the
       new dimensions given by `shape`. Mirrors
       `apl-runtime::builtins::reshape(a, b)`, where `a` is the shape
       vector and `b` is the data — field names spell out that role
       instead of reusing `lhs`/`rhs`, since (unlike `MatMul`/
       `ElementwiseOp`) the two operands are not interchangeable in kind.

IndexGenerator { count: Box<Expr>, span }
    -- monadic `⍳N` (APL iota / index generator): the vector
       `0, 1, ..., N-1` (0-based at the `Array` level, even though APL's
       own surface syntax is 1-indexed). Mirrors
       `apl-runtime::builtins::index_generator`.

IndexOf { haystack: Box<Expr>, needle: Box<Expr>, span }
    -- dyadic `A⍳B` (APL index-of / search): for each element of
       `needle`, its position in `haystack` (or `haystack`'s length if
       not found). Mirrors `apl-runtime::builtins::index_of(a, b)`, where
       `a` is the haystack and `b` is the needle.

Ravel { target: Box<Expr>, span }
    -- monadic `,A` (APL ravel): flatten `target` to a rank-1 vector.
       Mirrors `apl-runtime::builtins::ravel`.

Catenate { lhs: Box<Expr>, rhs: Box<Expr>, span }
    -- dyadic `A,B` (APL catenate): concatenate `lhs` and `rhs` along
       their ravel order. Mirrors `apl-runtime::builtins::catenate`.
```

`ElementwiseOpKind` (used by `ElementwiseOp` above, and by `Reduce`/`Scan`/
`OuterProduct` here) grew from five variants to thirteen in this addendum:
`Add | Sub | Mul | Div | Pow | Max | Min | Eq | Ne | Lt | Le | Ge | Gt`. The
eight new variants (`Max`/`Min`/the six comparisons) mirror
`array_runtime::BinOp`'s own identical extension, added alongside
`apl-runtime` for APL's `⌈`/`⌊` max/min glyphs and its six comparison
glyphs `= ≠ < ≤ ≥ >` (PR #8072). `Reduce`/`Scan`/`OuterProduct` reuse the
whole `ElementwiseOpKind` enum rather than a narrower subset — the IR does
not restrict which `op` a frontend may pick for these adverbs (e.g. `Pow`
has no APL reduce-adverb precedent, but nothing stops a frontend from
constructing `Reduce { op: Pow, .. }`), mirroring how `ElementwiseOp`
itself places no restriction on which arithmetic op appears there.

No new `Feature` flag was added: `Reduce`/`Scan`/`OuterProduct`/`Shape`/
`Reshape`/`Ravel`/`Catenate` observe both `Feature::MatrixOps` and
`Feature::ArrayColumnMajor` — the same class of genuine array/matrix
operation as `MatMul`/`Transpose` above. `IndexGenerator`/`IndexOf` observe
only `Feature::NDArrays` — they construct/query arrays without inherently
being a "matrix op" any more than `Range`/`IndexGet` are.

No frontend crate consumed any of these nine variants at the time this
addendum was written — it was IR-substrate-only, mirroring how
`array-runtime`'s AR-2 task preceded `apl-runtime` as its own separate
step before that runtime crate could be built. **This is no longer
true**: `apl-to-semantic-ir` (a follow-up task after this addendum, as
anticipated) does emit `Reduce`/`Scan`/`OuterProduct` from APL's
`+/`/`+\`/`∘.×` operators. No backend implements codegen for any of the
nine yet, though — `sir-runtime-array` (the JS/TS runtime package)
deliberately scoped them out (see that package's own `src/index.ts` doc
comment) since no frontend needed them when it shipped, and
`semantic-ir-to-javascript`'s later SIR22 base-cut codegen PR found that,
because these nine share `NDArrays`/`MatrixOps`/`ArrayColumnMajor` with
the base cut, a plain feature-flag capability check can no longer tell
"safe" modules from "still unimplemented" ones — see that crate's
`find_unimplemented_sir22_addendum_node` for the dedicated tree-walk this
now requires.

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

- **JS** (`semantic-ir-to-javascript`) — **done**: new `match` arms emit calls
  into an *inlined* `__Sir.Array.*` sub-runtime (a plain-JS port of
  `sir-runtime-array`, not an `import`/`require` — this backend always
  inlines its runtime helpers, unlike the TS backend's imported-package
  model) for the base cut (`ArrayLit`/`Range`/`MatMul`/`ElementwiseOp`/
  `Transpose`/`IndexGet`/`IndexSet`). `NDArrays`/`MatrixOps`/
  `ArrayColumnMajor` are in `ACCEPTED_FEATURES`. The SIR22 "APL addendum"
  nodes below share these same three features but remain deferred — this
  backend adds a dedicated tree-walk check inside `compile()` (beyond the
  ordinary feature-flag capability check) so a module using one of the
  nine still fails cleanly rather than reaching an emit-time panic; see
  that crate's `find_unimplemented_sir22_addendum_node`.
- **TS** (`semantic-ir-to-typescript`) — not yet done: still rejects
  `NDArrays`/`MatrixOps`/`ArrayColumnMajor` via the plain capability-
  rejection path (a real `import { ... } from
  "@coding-adventures/sir-runtime-array"` codegen PR, mirroring the JS
  backend's call shapes but against the published npm package instead of
  an inlined port, is a follow-up).
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

This spec has since received one additive addendum — the "New `Expr`
variants (APL primitives)" section above — which bumped
`metadata::CURRENT_SIR_VERSION` again (`"3"` → `"4"`, following SIR23's
`"2"` → `"3"` bump). Both the original MATLAB-oriented cut and the APL
addendum share this same versioning discipline: every SIR text token
addition is a version bump, regardless of whether it lands in the spec's
original PR or a later addendum to it.

## References

Internal: [`HML01`](HML01-math-to-semantic-ir.md),
[`SIR10`](SIR10-narrow-waist-semantic-ir.md),
[`SIR16`](SIR16-ir-extensions-for-python-and-javascript.md) (extension
precedent), [`MA00`](MA00-array-runtime.md) (`array-runtime`'s `Array` value
model and `execute()` op shapes this spec mirrors 1:1),
[`MA01`](MA01-matlab-language.md) (the frontend that will emit these nodes).
