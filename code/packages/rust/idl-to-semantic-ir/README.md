# idl-to-semantic-ir

IDL (Interactive Data Language) CST → narrow-waist Semantic IR (SIR22
array/matrix domain + KW1 keyword-argument vocabulary). **MA-12e**, the
final item in IDL's Wave-6 rollout per
[`MA12-idl-language.md`](../../../specs/MA12-idl-language.md) §6 — and
front3 of [`HML01-math-to-semantic-ir.md`](../../../specs/HML01-math-to-semantic-ir.md)'s
"every math language also builds a `<lang>-to-semantic-ir` frontend"
pipeline stage.

## Where this fits

```
idl-lexer  →  idl-parser  →  idl-runtime + idl-repl   (existing, unchanged)
                          →  idl-to-semantic-ir       (this crate, NEW)
```

Like every sibling `-to-semantic-ir` frontend in this repo
(`scilab-to-semantic-ir`, `q-to-semantic-ir`, `matlab-to-semantic-ir`, ...),
this crate is a wholly separate, ahead-of-time lowering pass over the same
`GrammarASTNode` CST `idl-runtime`'s tree-walking evaluator already walks —
it depends on `coding-adventures-idl-parser` only, **not**
`coding-adventures-idl-runtime`.

## Usage

```rust
use coding_adventures_idl_to_semantic_ir::compile_source;

let module = compile_source("x = 1 + 2\n", "demo").unwrap();
assert!(module.functions.iter().any(|f| f.name == "main"));
```

`compile(&GrammarASTNode, module_name)` is also exported for callers that
already have a parsed tree (e.g. from `coding_adventures_idl_parser::try_parse_idl`).

## The two flagged semantic decisions

This task specifically asked that the two subtle, review-caught-once
semantic details be verified directly against `idl-runtime`'s own
(already-fixed) source, not re-derived independently — both are below.

### `#` vs `##` (matrix product operand order)

Verified directly against `idl_runtime::eval::eval_multiplicative`'s
current behavior:

```text
A ## B   =>  matmul(A, B)   -- ordinary, direct order
A # B    =>  matmul(B, A)   -- SWAPPED
```

`##` lowers to `MatMul { lhs: A, rhs: B }`; `#` lowers to
`MatMul { lhs: B, rhs: A }` — operands swapped, mirroring `idl-runtime`'s
own fix exactly. See `src/lower.rs`'s module doc comment ("`#` vs `##`")
and `tests/test_lower.rs`'s `hash_is_matmul_with_operands_swapped` /
`hash_hash_is_ordinary_matmul_with_operands_in_source_order`.

### 2-D subscript column/row order

IDL's own source-level subscript order is `[column, row]` (`a[i, j]`: `i`
selects the column, `j` the row — MA12 §2 note 1). `idl-runtime`'s own
`resolve_subscripts` commits to a concrete mapping onto `array-runtime`'s
`(row, col)` addressing: `cols = subscript_positions(subs[0], ncols)`,
`rows = subscript_positions(subs[1], nrows)` — i.e. the *first*-written
subscript is the column, the *second*-written is the row.

`Expr::IndexGet`/`Stmt::IndexSet`'s own `indices: Vec<IndexArg>` is
row-major in *SIR's* convention (`indices[0]` = row, `indices[1]` = column
— the same convention `matlab-to-semantic-ir`/`scilab-to-semantic-ir` and
the JS backend's own `indexGet`/`indexSet` already use). So lowering
`a[i, j]` verbatim would silently swap rows and columns relative to
`idl-runtime`'s own ground truth — this frontend instead **reverses** the
two subscripts when building `indices`: `[lower(j), lower(i)]`, the
*second*-written IDL subscript first. See `src/lower.rs`'s module doc
comment's own dedicated section for the full derivation, and
`tests/test_lower.rs`'s `two_d_subscript_swaps_column_row_order_relative_to_source`.

(This mapping cannot be exercised end-to-end with a genuine 2-D array value
in this cut's own test suite — see "Known gaps" below, "No way to construct
a genuine 2-D array.")

## Other IDL-specific lowering decisions

- **Subscript indexing base — no shift at all.** IDL is 0-based already
  (MA12 §2, confirmed directly against `idl_runtime::eval::resolve_index`,
  which has no `- 1` anywhere) — unlike MATLAB/Scilab's 1-based surface,
  this frontend applies **zero** index-base translation.
- **Case folding — yes, uppercase at lowering time.** `idl-runtime` folds
  every identifier (variable, `PRO`/`FUNCTION`, parameter, keyword name) to
  uppercase at bind/lookup time, citing real IDL's own documented
  case-insensitivity. This frontend makes the identical decision at
  lowering time (`fold_case`, `str::to_uppercase`), so a SIR module
  downstream sees the same canonical spelling `idl-runtime` would compute
  for the identical program. This is the first `-to-semantic-ir` frontend
  in this repo to fold case at all — neither `scilab-to-semantic-ir` nor
  `q-to-semantic-ir` do (Scilab/Q are both case-sensitive; confirmed by
  grepping both crates for any case-folding logic, finding none), so there
  was no frontend precedent to extend here beyond `idl-runtime`'s own call.
- **Two namespaces (`PRO`/`FUNCTION` may share a name).** `semantic_ir::Module::functions`
  is one flat namespace, so every procedure's own SIR function name is
  mangled with a `$PROC` suffix (`GREET` → `GREET$PROC`) — a real IDL
  `NAME` token can never spell `$` (`idl.tokens`' own `NAME` pattern has no
  `$` in its character class), so this can never collide with a
  user-declared name. `FUNCTION` definitions keep their name unmangled. A
  `procedure_call_stmt` looks its target up in the `$PROC`-mangled set; a
  function call (expression position) looks its target up unmangled.
- **Keyword arguments — SIR's existing KW1 vocabulary, not positional
  desugaring.** `HML01` §3's own MA-12e sketch anticipated desugaring
  keyword arguments to positional bindings, leaving "whether a first-class
  keyword-argument SIR node is warranted" open. Checked directly against
  `semantic-ir`'s *current* state: `Expr::KeywordArg`/`ParamKind::Keyword`
  (KW1) already exist, and — contrary to `semantic_ir::manifest::Feature`'s
  own doc comment (which still says `KeywordParams` "is NOT yet accepted by
  any backend," now stale) — `semantic-ir-to-javascript`'s own
  `ACCEPTED_FEATURES` list *does* include `Feature::KeywordParams` today
  (KW4 shipped). So this frontend uses the existing, already-accepted KW1
  vocabulary directly. One wrinkle: IDL's `KW=kw` header binds a *call-site*
  keyword name to a possibly-*differently-spelled* body-local variable;
  `semantic_ir::Param` has only one name field, so this frontend declares
  the `Param` under the call-site name and, when the local spelling
  genuinely differs, prepends one ordinary `LetStarBinding` aliasing the
  local name to the parameter — zero new vocabulary, just an explicit
  rename statement.
- **`RETURN` only in tail position.** `semantic-ir` has no early-exit
  control-flow node (the same whole-IR gap documented for `break`/
  `continue` below). This frontend supports exactly the shape that needs no
  early-exit node: a `RETURN`/`RETURN, expr` as the literal last statement
  of a routine's own top-level body, lowered directly into the `Block`'s
  trailing value. Any other placement (nested in a branch/loop, or
  non-trailing) is a clean, disclosed error.
- **`FOR` loop counter is scope-rewound after the loop.** Mirrors
  `scilab-to-semantic-ir`'s own hard-won fix: the shared JS backend's
  `ForRange` codegen JS-block-scopes the loop variable, so reusing an
  existing name as the counter (or reading the counter's value after the
  loop) is rejected outright rather than silently reading a stale value.

## Scope (v0.1.0)

**Supported**: literals (int/float by lexeme, strings), array literals
(always rank-1, MA12 §2), variables/assignment (including subscripted
assignment), arithmetic `+ - * /` (always elementwise — IDL's bare `*` is
*never* a matmul disambiguation the way MATLAB's/Scilab's is; matrix
product is exclusively `#`/`##`), `^` (power, **left-associative** —
verified against `idl-parser`'s own README, the opposite of Scilab's/
MATLAB's right-associative `^`), word comparisons `EQ NE LT LE GT GE`,
control flow (`IF...THEN...ELSE`, `FOR...DO`, `WHILE...DO`,
`REPEAT...UNTIL`, a bare `BEGIN...END` block flattened inline — IDL has no
block-level scoping), `PRO`/`FUNCTION` definitions and calls with keyword
arguments and the `/KEYWORD` boolean shorthand, subscripting (plain, 2-D,
ranged, strided, wildcard `*`), `PRINT` (one argument), `TRANSPOSE`,
`INDGEN`/`FINDGEN`/`DINDGEN`/`LINDGEN`.

**Deliberately out of scope, each rejected with a clean, explicit
`IdlLowerError`** (never silently mis-lowered — see `src/lower.rs`'s module
doc comment for the full account of each):

- `AND`/`OR`/`XOR`/`NOT` (bitwise) — no existing SIR/backend primitive
  reproduces IDL's genuine **64-bit** bitwise semantics (verified against
  `idl_runtime::builtins`'s own `(x as i64) & (y as i64)`-shaped
  implementation). `Expr::LogicalAnd`/`LogicalOr` are short-circuit
  *boolean* nodes (a real semantic mismatch for non-`{0,1}` operands or
  side-effecting operands); JavaScript's own native bitwise operators
  truncate to 32 bits, not 64. A correct fix needs a genuinely new,
  `BigInt`-based backend runtime helper — `semantic-ir-to-javascript`
  follow-up work, out of this frontend-only task's scope (mirroring how
  J's `tally`/`replicate`/`exp` and Q's five `q_*` primitives were each
  *introduced* by one frontend PR but wired into the shared JS backend by a
  later, separate one).
- `SIN COS TAN SQRT ABS EXP ALOG ALOG10` — no elementwise math-function
  `BuiltinCall` name is registered in the JS backend today (confirmed by
  grepping every sibling array-family frontend; none registers one either).
- `TOTAL`/`MIN`/`MAX` — `idl-runtime` computes these via a **whole-array**
  reduction (`array_runtime::ops::{sum,max,min}`) regardless of rank; the
  nearest existing SIR node, `Expr::Reduce` (the APL addendum), instead
  folds *per row* for a rank-2 target — exact only when the target is
  provably rank ≤ 1, which this frontend (no type inference) cannot verify
  for a general argument. Reusing it anyway would trade a clean rejection
  for a construct that is *sometimes* silently wrong — this repo's
  discipline treats that as strictly worse.
- `N_ELEMENTS`/`SIZE` — no existing "total element count / dimension
  vector, any rank" primitive (the nearest candidate, `BuiltinCall("tally", ...)`,
  is exact only for rank-1 targets, for the identical reason `TOTAL`/`MIN`/
  `MAX` are excluded).
- `INTARR FLTARR DBLARR LONARR` — no existing SIR primitive materializes
  "N zero-filled elements" at a *dynamic* (non-literal) runtime size.
- **Negative-from-end subscripts** (`a[-1]`) and **wildcard-range-end
  subscripts** (`a[s0:*]`) — both need the *runtime length* of the indexed
  axis, the identical class of problem MATLAB's `end`-relative indexing is.
  The syntactically obvious negative-literal case (`a[-1]`) is specifically
  detected and rejected with a clear message; a general expression that
  merely *might* evaluate negative at runtime is not specially rejected —
  it fails loud (a JS "out of bounds" exception), not silently, if it ever
  actually happens.
- 3-D-or-higher subscripting — `idl-runtime`'s own `resolve_subscripts` and
  the JS backend's own `indexGet`/`indexSet` both cap at rank 2.
- `break`/`continue` — `semantic-ir` has no early-exit control-flow node at
  all (confirmed: no `Break`/`Continue` variant anywhere in
  `semantic-ir/src/nodes.rs`), the identical whole-IR gap
  `scilab-to-semantic-ir` documents for the same reason.
- Structures, pointers, objects, `LIST`/`HASH`, `COMMON` blocks,
  `CASE`/`SWITCH`/`FOREACH`, `_EXTRA`/`_REF_EXTRA` — all deferred whole by
  MA12 §4 itself, unchanged here.

### No way to construct a genuine 2-D array value in this cut

Since `INTARR`/`FLTARR`/`DBLARR`/`LONARR` are out of scope and IDL's own
array-literal grammar has **no** 2-D row-separator syntax at all (unlike
Scilab/MATLAB's `[1 2; 3 4]` — `array_literal = LBRACKET [array_elements]
RBRACKET` is always flat), this frontend has no in-scope way to build a
genuine rank-2 array through supported constructs. 2-D subscripting
(`a[i, j]`) is lowered *correctly* (with the column/row swap documented
above), but this crate's own test suite cannot exercise it end-to-end with
real 2-D data — only via a direct structural assertion on the emitted
`IndexArg` order (`tests/test_lower.rs`). Likewise, `#`/`##`'s oracle-tested
end-to-end round trip (`tests/oracle.rs`) necessarily uses single-element
(rank-1) operands, for which the two operator orders are numerically
indistinguishable (scalar multiplication commutes) — the structural test
(`tests/test_lower.rs::hash_is_matmul_with_operands_swapped`) is the
load-bearing proof for that decision, not the oracle file.

## Testing

- `tests/test_lower.rs` — structural unit tests asserting the actual
  lowered `Expr`/`Stmt`/`Module` shapes (69 tests).
- `tests/test_validator.rs` — every module this frontend produces passes
  `semantic_ir::validate` and is structurally accepted by
  `semantic-ir-to-javascript`'s `Backend::check_module` (15 tests).
- `tests/e2e_node.rs` — compiles to JavaScript and actually runs it through
  `node`, asserting printed output (13 tests, gated on `node` availability).
- `tests/oracle.rs` (HML01 §7) — the same IDL program run through **two**
  independent implementations (`idl-runtime`'s own tree-walking evaluator,
  and this frontend → `semantic-ir-to-javascript` → `node`), diffed; a
  29-case corpus, gated on `node` availability.

```sh
cargo test -p coding-adventures-idl-to-semantic-ir -- --nocapture
cargo clippy -p coding-adventures-idl-to-semantic-ir --all-targets
```
