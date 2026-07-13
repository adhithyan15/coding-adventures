# Changelog

## [0.1.0] - 2026-07-12

### Added

- Initial `apl-to-semantic-ir` frontend crate (MA-4f, per
  [HML01](../../../specs/HML01-math-to-semantic-ir.md) §2/§5) — the last
  remaining APL rollout item, and the first frontend to consume the SIR22
  addendum (`Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/
  `IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`, plus the `Max`/`Min`/`Eq`/
  `Ne`/`Lt`/`Le`/`Ge`/`Gt` `ElementwiseOpKind` variants) that `semantic-ir`
  shipped specifically for APL ahead of this crate landing.
- `compile`/`compile_source` lowering `coding-adventures-apl-parser`'s
  `GrammarASTNode` CST into a `semantic_ir::Module`.
- Supported: number literals (int/float, high-minus `¯` negative sign),
  stranded literals (`1 2 3` → a single rank-1 `ArrayLit`), variables,
  parenthesised grouping; assignment including right-associative chained
  assignment (`A←B←3`, unrolled into dependency-ordered statements); all 12
  scalar dyadic atoms (`+ - × ÷ ⌈ ⌊ = ≠ < ≤ ≥ >`), unconditionally lowered to
  `ElementwiseOp` (no scalar-vs-array disambiguation needed — a genuine
  simplification over `matlab-to-semantic-ir`, since none of APL's atoms
  have a non-elementwise reading); the 6 atoms with a monadic meaning (`+ -
  × ÷ ⌈ ⌊`), mapped onto `"neg"`/`"sign"`/`"recip"`/`"ceil"`/`"floor"`
  builtins (`+` is a pass-through no-op — no complex numbers in this cut);
  `⍴`/`⍳`/`,` (shape-reshape, index-generator-index-of, ravel-catenate),
  monadic and dyadic; `/` (reduce) and `\` (scan), monadic-only; `∘.` (outer
  product), dyadic-only; auto-print of a bare top-level value expression
  onto the shared `"print"` builtin (APL's own real language semantic per
  MA05 §4, unlike MATLAB's `;`-suppression convention).
- Explicit, disclosed rejections (each a clean `AplLowerError`, never
  silently mis-lowered): the 6 comparison atoms used monadically (no
  monadic meaning in APL); a reduce/scan-decorated function used dyadically
  (both are inherently monadic); an outer-product-decorated function used
  monadically (inherently dyadic); `⍴`/`⍳`/`,` decorated with an operator
  (not scalar dyadic functions). Constructs the grammar itself cannot
  produce (boxing, the rank conjunction, user-defined functions, control
  flow) need no explicit rejection code.
- 44 tests: 41 unit tests in `tests/test_lower.rs` covering every dyadic
  atom individually, every monadic atom (valid and rejected), reduce/scan
  (valid + rejected arity), outer product (valid + rejected arity),
  `⍴`/`⍳`/`,` (monadic and dyadic, plus operator-decoration rejection),
  stranded literals, high-minus literals, chained assignment,
  first-occurrence-vs-reassignment, parenthesised grouping, undefined-
  variable rejection, parse-error propagation, and a full multi-line
  program that validates cleanly via `semantic_ir::validate`; 3 tests in
  `tests/test_validator.rs` mirroring `matlab-to-semantic-ir`'s own
  capability-rejection pattern (`semantic-ir-to-javascript` correctly
  rejects any module using SIR22/SIR22-addendum nodes, which is nearly
  every APL program).
- No `tests/e2e_node.rs`: unlike MATLAB, APL has no literal-only escape
  hatch from the array domain (every dyadic scalar op is unconditionally an
  `ElementwiseOp`), so a real round-trip through a backend needs
  `sir-runtime-array`'s SIR22 codegen, which does not exist yet (tracked
  separately, HML01 §4).
- Marks MA-4f done in `MA05-apl-language.md` §6, with a design-notes
  writeup mirroring MA-4d/MA-4e's own style.
