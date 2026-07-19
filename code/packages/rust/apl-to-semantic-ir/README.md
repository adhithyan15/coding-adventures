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
  capability-acceptance pattern: every lowered module validates, and
  `semantic-ir-to-javascript` (which now implements real codegen for the
  full SIR22 domain, including the "addendum" this crate is the first
  consumer of) correctly *accepts* every module this frontend produces —
  which, for APL, is nearly every program (see the "No scalar/array
  disambiguation" point above: even `3+4` is an `ElementwiseOp`). This
  superseded an earlier version of this test asserting *rejection*, back
  when the backend had no SIR22-addendum codegen yet — see
  `CHANGELOG.md`'s 0.1.2 entry.
- `tests/e2e_node.rs` — real end-to-end, `node`-executed proof: 9 tests,
  each compiling real APL source (one per SIR22-addendum primitive —
  reduce, a non-`Add` reduce, scan composed with reduce, two outer-product
  shapes, shape-of-reshape, reshape with a bare stranded-literal shape,
  index generator, ravel of a reshaped matrix) through `compile_source` →
  `semantic_ir::validate` → `semantic_ir_to_javascript::compile` → a temp
  `.js` file → `node`, asserting the printed stdout.
- `tests/oracle.rs` — HML01 §7 oracle/golden testing: the SAME APL source
  cross-checked against `apl-runtime`'s independent tree-walking evaluator
  (ground truth) as well as the compiled-through-`node` path, closing the
  "APL/J's own oracle tests remain open follow-on items" note in
  [HML01](../../../specs/HML01-math-to-semantic-ir.md) §5. 17-case corpus:
  the 9 programs `tests/e2e_node.rs` already proves run correctly (now also
  checked against `apl-runtime`), 2 more completing oracle coverage of all
  9 SIR22-addendum node kinds, and 6 base-cut cases. Unlike the sibling
  MATLAB/Octave oracle files, this one needs no `setup`/`final_expr` split
  and no `normalize()` — see that file's own module doc for why, confirmed
  empirically rather than assumed. Building it also surfaced 3 genuine,
  previously-undiscovered bugs in monadic `- × ÷ ⌈ ⌊` (a wrong display
  glyph, a wrong value on array operands, and a hard crash on all 4 of
  `× ÷ ⌈ ⌊`) — documented in that file's module doc and `CHANGELOG.md`'s
  0.1.4 entry, excluded from the corpus, and reported as follow-up items
  rather than fixed here (fixing them needs a change to
  `semantic-ir-to-javascript`, a separate crate).
