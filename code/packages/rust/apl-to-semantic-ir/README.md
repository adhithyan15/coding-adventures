# apl-to-semantic-ir

APL CST → narrow-waist Semantic IR. Task **MA-4f** — the last remaining
rollout item for APL (see
[MA05](../../../specs/MA05-apl-language.md) §5/§6) — and the first frontend
to actually *consume* the SIR22 addendum
(`Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/`IndexOf`/
`Ravel`/`Catenate`) that [semantic-ir](../semantic-ir) shipped specifically
for APL ahead of this crate landing.

## Where this fits

```
APL source
   │
   ▼  coding_adventures_apl_parser::try_parse_apl(src)
parser::grammar_parser::GrammarASTNode   (generic CST)
   │
   ▼  apl_to_semantic_ir::compile
semantic_ir::Module                      (per SIR10 + SIR22 + SIR22 addendum)
```

The lowered `Module` can then be validated (`semantic_ir::validate`) and
handed to any SIR backend — "write the APL frontend once, target every SIR
backend" is the whole point of this narrow-waist design. This crate depends
only on `coding-adventures-apl-parser` (the CST), **not**
`coding-adventures-apl-runtime` (the tree-walking evaluator) — lowering only
needs the parse-tree shape.

## Usage

```rust
use apl_to_semantic_ir::compile_source;

let module = compile_source("A←3+4\n", "demo")?;
```

## Scope (v0.1.0)

APL's grammar (per MA05 §4, as already implemented by `apl-parser`/
`apl-runtime`) has **no control flow and no user-defined functions** in this
cut — just straight-line assignment and value expressions. That makes this
frontend simpler than `matlab-to-semantic-ir` in two concrete ways:

1. **No scalar/array disambiguation.** Every one of APL's 12 scalar dyadic
   atoms (`+ - × ÷ ⌈ ⌊ = ≠ < ≤ ≥ >`) lowers *unconditionally* to
   `Expr::ElementwiseOp` — none of them has a non-elementwise reading the way
   MATLAB's `*` does, so there is no `expr_is_known_scalar`-style heuristic
   to write at all. `3+4` and `A+B` produce the exact same node shape.
2. **A single `main` function.** There are no separate named functions to
   collect in a first pass; the whole program lowers into one `main`.

See `src/lower.rs`'s module doc comment for the exact per-construct lowering
table, the chained-assignment (`A←B←3`) unrolling design, the auto-print
convention (APL's bare `value_expr` auto-prints — a real language semantic
per MA05 §4, not a REPL nicety), and the handful of deliberately-rejected
constructs:

- The 6 comparison atoms (`= ≠ < ≤ ≥ >`) used monadically — no monadic
  meaning in APL.
- A reduce/scan-decorated `function_expr` used dyadically (`3+/4`) — both
  are inherently monadic.
- An outer-product-decorated `function_expr` used monadically (`∘.×1`) —
  outer product is inherently dyadic.
- `⍴`/`⍳`/`,` decorated with `/`/`\`/`∘.` — these three primitives are not
  "scalar dyadic functions" the way the other 12 atoms are.

Each of the above is syntactically constructible by `apl-parser`'s grammar
but semantically invalid, and gets a clean `AplLowerError` rather than a
silent misinterpretation or a panic — mirroring
`apl-runtime::eval`'s identical set of runtime rejections, just caught at
lowering time instead of evaluation time.

Boxing/nested arrays, the rank conjunction, user-defined functions, and
control flow are **not** rejection-tested here because `apl-parser`'s
grammar cannot produce them at all — there is no CST shape for this lowerer
to ever reach.

## Well-known `BuiltinCall` names introduced here

`+` (conjugate) is the one monadic case that needs no wrapper at all — it is
a genuine no-op since this cut has no complex numbers, so the operand passes
through unchanged. The other five monadic scalar atoms map onto
`Expr::BuiltinCall`:

| Atom | Meaning     | Builtin name |
|------|-------------|--------------|
| `-`  | negate      | `"neg"` (reused from `matlab-to-semantic-ir`) |
| `×`  | sign        | `"sign"` (new) |
| `÷`  | reciprocal  | `"recip"` (new) |
| `⌈`  | ceiling     | `"ceil"` (new) |
| `⌊`  | floor       | `"floor"` (new) |

A bare top-level value expression is wrapped in `BuiltinCall("print", ...)`,
reusing the exact name `matlab-to-semantic-ir` maps its own `disp(x)` call
onto.

### Testing

- `tests/test_lower.rs` — unit tests over every dyadic atom (all 12), every
  monadic atom (6 valid + 6 rejected), reduce/scan (valid monadic use +
  rejected dyadic use), outer product (valid dyadic use + rejected monadic
  use), `⍴`/`⍳`/`,` (both monadic and dyadic forms, plus rejection when
  decorated with an operator), stranded literals, high-minus negative
  literals, chained assignment, first-occurrence-vs-reassignment,
  parenthesised grouping, undefined-variable rejection, parse-error
  propagation, and a full multi-line program that validates cleanly via
  `semantic_ir::validate`.
- `tests/test_validator.rs` — mirrors `matlab-to-semantic-ir`'s own
  capability-rejection pattern: every lowered module validates, and
  `semantic-ir-to-javascript` (which does not implement SIR22/SIR22-addendum
  codegen yet) correctly *rejects* any module using those nodes — which, for
  APL, is nearly every program (see the "No scalar/array disambiguation"
  point above: even `3+4` is an `ElementwiseOp`).
- **No `tests/e2e_node.rs`.** Unlike MATLAB (whose purely-literal arithmetic
  subset avoids the array domain entirely), APL's *every* dyadic scalar
  operation is unconditionally `Expr::ElementwiseOp` — there is no
  literal-only escape hatch. A genuine round-trip through a real backend
  therefore needs `sir-runtime-array`'s SIR22 codegen, which does not exist
  yet (tracked separately per HML01 §4) — an e2e-through-`node` test isn't
  possible for this crate today.
