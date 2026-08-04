# matlab-to-semantic-ir

MATLAB CST → narrow-waist Semantic IR. The first frontend to target
[SIR22](../../../specs/SIR22-array-matrix-semantic-ir.md), the array/matrix
domain extension of the SIR10 narrow-waist IR (see
[HML01](../../../specs/HML01-math-to-semantic-ir.md)).

## Where this fits

```
MATLAB source
   │
   ▼  coding_adventures_matlab_parser::try_parse_matlab(src)
parser::grammar_parser::GrammarASTNode   (generic CST)
   │
   ▼  matlab_to_semantic_ir::compile
semantic_ir::Module                      (per SIR10 + SIR16 + SIR22)
```

The lowered `Module` can then be validated (`semantic_ir::validate`) and
handed to any SIR backend (e.g. `semantic-ir-to-javascript`) — "write the
MATLAB frontend once, target every SIR backend" is the whole point of this
narrow-waist design.

## Usage

```rust
use matlab_to_semantic_ir::compile_source;

let module = compile_source("x = 1 + 2;\n", "demo")?;
```

## Scope (v0.1.0)

MATLAB is a large language; this first cut covers a well-defined,
extensively tested subset and returns a clean `MatlabLowerError` for
anything outside it, rather than silently mis-lowering. See `src/lower.rs`'s
module doc comment for the exact supported-construct list and every
deliberate, disclosed scope limit (stepped/matrix-valued `for` loops,
`end`-relative indexing, matrix division, multi-output functions, nested
functions, `break`/`continue`/`return` — semantic-ir has no early-exit
control-flow node at all yet, a whole-IR gap rather than something specific
to this frontend — `switch`/`try`/`global`, cell arrays, and lambdas).

### Scalar/array disambiguation

MATLAB's `+ - * / \ ^` are polymorphic between scalar and matrix operands at
*runtime*; this frontend has no shape/type inference, so it uses a
conservative, purely syntactic heuristic on the lowered expression tree (see
`expr_is_known_scalar` in `src/lower.rs`): an operand counts as "known
scalar" only if it is built transitively from literal numbers. This means
ordinary *variable* arithmetic (a loop accumulator, a function parameter,
...) always takes the array-domain (`ElementwiseOp`/`MatMul`) path, even
when the value is a genuine runtime scalar — a real limitation, not a bug,
documented (and demonstrated) in `tests/test_validator.rs`.

### Testing

- `tests/test_lower.rs` — unit tests over every lowering rule, every
  documented scope limit's rejection, and a DoS-guard regression pair
  (`a_pathologically_long_flat_{additive,multiplicative}_chain_is_cleanly_rejected`)
  reproducing a native-stack-overflow bug security review caught and this
  crate fixed before its first push — see `CHANGELOG.md`'s "Fixed" entry.
- `tests/test_validator.rs` — every lowered module passes
  `semantic_ir::validate`; a module using SIR22 nodes is correctly
  *rejected* by `semantic-ir-to-javascript`'s capability check (that backend
  does not implement SIR22 codegen yet), mirroring the same
  capability-rejection verification pattern used to land SIR22/SIR23
  themselves.
- `tests/e2e_node.rs` — lowers MATLAB source (both purely-literal scalar
  arithmetic and real array/matrix programs — matrix multiplication,
  elementwise scalar broadcast, indexed assignment, range+transpose),
  emits JavaScript, and **actually executes it with `node`**, gated on
  `node` availability. This proves the compiled JS *runs without
  crashing* and prints the expected value — but does not check that value
  against an independent implementation of MATLAB itself; that is
  `tests/oracle.rs`'s job.
- `tests/oracle.rs` — the first **oracle/golden test** in the HML01 track
  (spec §7): for a small corpus of MATLAB programs, runs the *same*
  computation through both `matlab-runtime` (this frontend's own sibling
  tree-walking interpreter — ground truth) and this crate's
  `compile_source` → `semantic-ir-to-javascript` → `node`, and asserts the
  two agree. Gated on `node` availability like `e2e_node.rs`. Its module
  doc comment is required reading before adding a new corpus entry: it
  explains why "the same source" is a `setup`/`final_expr` pair rather
  than one literal string (MATLAB's `disp` is a confirmed no-op in
  `matlab-runtime`; `semantic_ir` has no representation of MATLAB's
  *implicit*-display convention at all), documents the normalization rules
  applied before comparing, and catalogs several confirmed bugs/gaps found
  while scoping the corpus (an integer-literal-division bug, a
  unary-minus-on-power bug, a missing `Feature::ShortCircuit` declaration,
  a severe `while`-loop-accumulator correctness bug with its own dedicated
  test, and two `matlab-runtime` gaps — no function-definition support,
  no indexed-assignment support — that make certain constructs impossible
  to oracle-test until that sibling crate grows the missing support). See
  `CHANGELOG.md` for the full write-up of each.
