# Changelog

## [0.1.0] - 2026-07-24

### Added

- Initial release — **MA-12e** of the IDL frontend (spec
  [`MA12`](../../../specs/MA12-idl-language.md) §6), built alongside
  `idl-runtime`/`idl-repl` per that spec's own rollout ordering and
  [`HML01`](../../../specs/HML01-math-to-semantic-ir.md) §2's "built
  alongside the runtime" per-language pattern — front3 of the HML01 track.
- `compile(&GrammarASTNode, module_name)` / `compile_source(source, module_name)`,
  mirroring every sibling `-to-semantic-ir` frontend's public API shape
  (`scilab-to-semantic-ir`, `q-to-semantic-ir`, ...).
- Lowers IDL's imperative surface (MA12 §4's in-scope subset) onto the
  existing SIR10/SIR16/SIR22/KW1 vocabulary: literals, array literals
  (always rank-1), variables/assignment (plain and subscripted), arithmetic
  (`+ - * /`, always elementwise — IDL's bare `*` is never a matmul
  disambiguation, unlike MATLAB's/Scilab's own `*`), `^` (power,
  **left-associative**, the opposite of Scilab's/MATLAB's right-associative
  `^` — verified against `idl-parser`'s own README), word comparisons
  (`EQ NE LT LE GT GE`), control flow (`IF...THEN...ELSE`, `FOR...DO`,
  `WHILE...DO`, `REPEAT...UNTIL`, a bare `BEGIN...END` block flattened
  inline since IDL has no block-level scoping), `PRO`/`FUNCTION`
  definitions and calls with keyword arguments and the `/KEYWORD` boolean
  shorthand, subscripting (plain, 2-D, ranged, strided, wildcard `*`),
  `PRINT` (one argument), `TRANSPOSE`, and the
  `INDGEN`/`FINDGEN`/`DINDGEN`/`LINDGEN` family (all four map onto
  `Expr::Range`).
- **Two flagged, directly-verified semantic decisions** (this task's own
  brief specifically asked that these be checked against `idl-runtime`'s
  own already-fixed source, not re-derived independently — see
  `src/lower.rs`'s module doc comment and this crate's `README.md` for the
  full account):
  - **`#` vs `##` operand order**: `A ## B` lowers to `MatMul { lhs: A, rhs: B }`
    (ordinary order); `A # B` lowers to `MatMul { lhs: B, rhs: A }`
    (**operands swapped**) — verified directly against
    `idl_runtime::eval::eval_multiplicative`'s current behavior
    (`execute(Kernel::MatMul, &acc, &rhs)` for `##`,
    `execute(Kernel::MatMul, &rhs, &acc)` for `#`).
  - **2-D subscript column/row swap**: IDL's own source order is
    `[column, row]` (`a[i, j]`: `i` is the column, `j` is the row — MA12 §2
    note 1), but SIR's `IndexGet`/`IndexSet` expects `[row, column]`
    (`indices[0]` = row, `indices[1]` = column, matching every existing
    array-family frontend and the JS backend's own `indexGet`/`indexSet`).
    Verified directly against `idl_runtime::eval::resolve_subscripts`'s own
    `cols = subs[0]`/`rows = subs[1]` mapping — this frontend therefore
    **reverses** the two subscripts (`indices: [lower(j), lower(i)]`) when
    lowering a 2-D `index_suffix`, rather than lowering them in written
    order (which would silently swap rows and columns relative to
    `idl-runtime`'s own ground truth).
- **Case folding**: every identifier (variable, `PRO`/`FUNCTION`, parameter,
  keyword name) is uppercase-folded at lowering time
  (`fold_case`/`str::to_uppercase`), mirroring `idl_runtime::eval::fold_case`'s
  own decision exactly — the first `-to-semantic-ir` frontend in this repo
  to fold case at all (Scilab/Q are both case-sensitive; confirmed no
  sibling crate has any case-folding logic).
- **Two namespaces**: a `PRO`'s own SIR function name is mangled with a
  `$PROC` suffix (a real IDL `NAME` token can never spell `$`), keeping it
  distinct from a same-named `FUNCTION` in `semantic_ir::Module`'s single
  flat function namespace — a `procedure_call_stmt` resolves against the
  mangled name, an expression-position function call against the plain one.
- **Keyword arguments via SIR's existing KW1 vocabulary**
  (`Expr::KeywordArg`/`ParamKind::Keyword`), not positional desugaring —
  checked directly against `semantic-ir`'s current state rather than
  assumed from `HML01` §3's own older sketch: KW1 already exists in the
  shared core, and `semantic-ir-to-javascript`'s `ACCEPTED_FEATURES` list
  already includes `Feature::KeywordParams` (contrary to
  `semantic_ir::manifest::Feature`'s own stale doc comment, which still
  says it "is NOT yet accepted by any backend" — corrected finding, not
  acted on outside this crate). A `KW=kw` header whose call-site keyword
  name genuinely differs from its body-local spelling gets one prepended
  alias `LetStarBinding`, not a new `Param` field.
- **`RETURN` supported only in tail position** (the literal last statement
  of a routine's own top-level body) — `semantic-ir` has no early-exit
  control-flow node, the same whole-IR gap documented for `break`/
  `continue` (confirmed: no `Break`/`Continue`/early-`Return` variant
  anywhere in `semantic-ir/src/nodes.rs`).
- **`REPEAT...UNTIL` desugars via a hoisted flag, not by lowering the loop
  body twice** — a security regression caught and fixed before landing
  (never shipped in a release): an earlier revision of `lower_repeat`
  lowered the loop body once inline (the guaranteed first run) and once
  more as a `WHILE`'s own body (the remainder), an exponential-duplication
  DoS shape identical to the one `scilab-to-semantic-ir::lower_select`'s
  own doc comment describes rejecting for `select`/`case` — K textually
  nested `REPEAT...UNTIL` statements produced a lowered `Module` of size
  O(2^K), regardless of whether the duplicate copy came from re-lowering
  the AST or `.clone()`-ing the already-built `Block`. Fixed by lowering
  the body exactly once, folding the "run at least once" requirement into
  the `WHILE` loop's own condition via a hoisted `$repeat_N` boolean flag
  (`$` can never appear in a real IDL `NAME` token, so this can never
  collide with a user-declared identifier — the same collision-freedom
  argument this crate's own `$PROC` namespace-mangling suffix and
  `scilab-to-semantic-ir`'s own `$select_N` hoisted temporary both already
  rely on). See `tests/test_lower.rs`'s
  `deeply_nested_repeat_until_does_not_blow_up_lowered_module_size`
  regression test.
- Documented, disclosed gaps (each rejected with a clean `IdlLowerError`,
  never silently mis-lowered — see `README.md` for the full per-construct
  rationale): `AND`/`OR`/`XOR`/`NOT` (bitwise — no existing SIR/backend
  primitive reproduces IDL's genuine 64-bit truncated bitwise semantics);
  `SIN COS TAN SQRT ABS EXP ALOG ALOG10`; `TOTAL`/`MIN`/`MAX` (whole-array
  reduction — the nearest existing node, `Expr::Reduce`, folds per-row for
  rank-2, exact only for rank ≤ 1, which this frontend cannot verify
  without type inference); `N_ELEMENTS`/`SIZE`; `INTARR FLTARR DBLARR
  LONARR`; negative-from-end subscripts (`a[-1]`) and wildcard-range-end
  subscripts (`a[s0:*]`) (both need the indexed axis's runtime length, the
  same unresolved class of problem MATLAB's `end`-relative indexing is);
  3-D-or-higher subscripting.
- `FOR`-loop counter is scope-rewound after the loop (a program reading the
  counter's value after the loop fails to *lower*, cleanly) and reuse of an
  already-known variable as the counter is rejected outright — both mirror
  `scilab-to-semantic-ir`'s own hard-won `ForRange`/JS-block-scoping fix.
- `MAX_EXPR_DEPTH`/`MAX_BLOCK_DEPTH` (200) recursion-depth guards, plus a
  `check_chain_length` DoS guard on every flat operator-chain tier
  (`logical`/`comparison`/`additive`/`multiplicative`/`power`) mirroring
  `scilab-to-semantic-ir::check_chain_length`'s own rationale: a flat CST
  repetition costs the *parser* no native stack, but this lowering pass's
  own fold still builds a genuinely N-deep nested `Expr` tree, and
  *dropping* that tree later can overflow the native stack for a
  pathologically long chain.
- Test suite: `tests/test_lower.rs` (69 structural unit tests asserting
  actual lowered `Expr`/`Stmt`/`Module` shapes), `tests/test_validator.rs`
  (15 tests confirming `semantic_ir::validate` + JS-backend
  `check_module` acceptance), `tests/e2e_node.rs` (13 tests, actually
  compiling to JS and running through `node`), `tests/oracle.rs` (HML01
  §7 — a 29-case corpus cross-checking `idl-runtime`'s own tree-walking
  evaluator against the compiled-JS-via-`node` path; all 29 agree with no
  `known_bug` marker needed).

### Known gaps

- No in-scope way to construct a genuine rank-2 array value (no
  `INTARR`/`FLTARR`/`DBLARR`/`LONARR`, and IDL's own array-literal grammar
  has no 2-D row-separator syntax) — so the 2-D-subscript column/row swap
  and the `#`/`##` operand-order fix are each verified structurally
  (`tests/test_lower.rs`) rather than via a genuinely non-commutative,
  real-2-D end-to-end oracle case.
