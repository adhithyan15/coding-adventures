# Changelog

## [0.1.2] - 2026-07-23

### Added

- **`tests/oracle.rs`**: one new corpus case,
  `whitespace_sensitive_strand_vs_subtraction_no_space_at_all` (`2-1` →
  `1`) — the third spelling of MA11 §3 bullet 2's negative-literal-vs-
  subtraction disambiguation, alongside the pre-existing `2 -1` (strand)
  and `2 - 1` (spaced subtraction) cases. `q-lexer`'s own
  `no_space_at_all_is_also_subtraction`/`no_space_at_all_stays_subtraction`
  tests and `q-runtime`'s own `scalar("2-1\n") == 1.0` assertion already
  confirmed the *lexer*/*runtime* resolve this correctly in isolation, but
  neither exercised the resolved CST shape through this crate's own
  lowering → `semantic-ir-to-javascript` → real `node` — this case closes
  that gap, confirmed to pass end-to-end (`known_bug: None`).

### Fixed (documentation only — no code/behavior change)

- `README.md`'s own "Testing" section had described `tests/oracle.rs` as a
  "33-case" corpus (and cited "5 of the 33 cases" as a known-bug gap) since
  the very first commit (`0.1.0`, #8826) — but the corpus was already
  50 cases at that point (confirmed via `git show` against that commit),
  and the 5-case display gap it described was already fixed in `0.1.1`
  (every entry has run `known_bug: None` since then). Both counts were
  stale from day one, not a regression introduced later. Also corrected
  `tests/test_lower.rs`'s documented count (56 → the actual 57). Past
  `CHANGELOG.md` entries below are left as originally written (an
  append-only historical record of what was believed true at the time,
  not a live spec) — this entry is the correction, not a rewrite of
  `[0.1.0]`/`[0.1.1]`. `README.md` now states the accurate, current counts
  (51-case oracle corpus after the addition above, 57 `test_lower.rs`
  cases) and drops the stale known-bug framing, since task #109 already
  closed that gap.

### Testing

- `cargo test -p q-to-semantic-ir`: all 81 tests pass (12 `e2e_node`, 1
  `oracle` covering the 51-case `CORPUS`, 57 `test_lower`, 10
  `test_validator`, 1 doctest).

## [0.1.1] - 2026-07-23

### Fixed (in the shared `semantic-ir-to-javascript` crate — task #109)

`tests/oracle.rs`'s own module doc comment ("A pre-existing shared-crate
display gap") disclosed 5 `known_bug` cases whose only remaining
disagreement was `semantic-ir-to-javascript` having no Q-specific display
convention: a genuine SIR22 `NDArray` result's negative values rendered
with APL's high-minus glyph `¯` instead of Q's own plain ASCII `-`,
regardless of source language, whenever `SIR_DISPLAY_J_UNDERSCORE` was
unset. This was the exact same shared-crate gap `j-to-semantic-ir/tests/
oracle.rs`'s own "Bug A" already found and left unfixed, per this repo's
"found, NOT fixed here" discipline for a bug in a crate consumed by many
frontends — not specific to this crate's own lowering, and not something
this crate could route around on its own.

Fixed directly in `semantic-ir-to-javascript` 0.51.4: a fifth,
mutually-exclusive `SIR_DISPLAY_Q_ASCII_MINUS` display flag, following the
exact pattern `SIR_DISPLAY_J_UNDERSCORE` established for J (see that
crate's own `CHANGELOG.md` for the full writeup). No change was needed in
this crate's own `src/lower.rs` — the gap was purely a shared-runtime
display-convention omission, not a lowering bug.

### Changed

- **`tests/oracle.rs`**: flips all 5 `known_bug: Some(DISPLAY_GAP)` cases
  to `known_bug: None`, confirmed by actually running the oracle test (not
  just inferred from source reading) — every one now agrees end-to-end
  with `q-runtime`: `whitespace_sensitive_strand_vs_subtraction_strand`
  (`2 -1`), `dyadic_sub_negative_result` (`3-4` → `-1`),
  `monadic_minus_negates_a_vector` (`-(1 2 3)` → `-1 -2 -3`),
  `each_on_an_elementwise_primitive_matches_direct_application` (`-'1 2 3`
  → `-1 -2 -3`), and `scan_then_reduce_negative` (`-/+\1 2 3` → `-8`). The
  now-unused `DISPLAY_GAP` constant is removed and the module doc comment
  updated to record the fix; the `known_bug` field itself stays on `Case`
  (unused by any entry today) for a future genuine bug to reuse.
- Verified via a genuine revert-and-confirm: temporarily forcing
  `semantic-ir-to-javascript`'s new `SIR_DISPLAY_Q_ASCII_MINUS` flag back
  to `false` reproduced the exact pre-fix failure (`¯1` instead of `-1`,
  etc.) for all 5 cases; restoring the flag made them pass again.

## [0.1.0] - 2026-07-22

### Added

- Initial `q-to-semantic-ir` frontend crate (MA-11e, per
  [MA11](../../../specs/MA11-q-language.md) §5/§6 and
  [HML01](../../../specs/HML01-math-to-semantic-ir.md) §2's amended
  per-language pattern: built alongside the runtime in this same wave, not
  as a later retrofit) — `compile`/`compile_source` lowering
  `coding-adventures-q-parser`'s `GrammarASTNode` CST into a
  `semantic_ir::Module`, targeting SIR22 (the array/matrix domain) plus the
  "APL/J addendum" (`Reduce`/`Scan`/`Ravel`/`Catenate`/`IndexGenerator`)
  those two crates already established.
- Built directly on `j-to-semantic-ir`'s design — the most structurally
  similar prior frontend (same `noun_expr`/`term`/`verb_expr` two-nonterminal
  shape, same primitive-verb dispatch pattern, MA11 §3's own "reused
  UNCHANGED" framing). Reuses that crate's exact conventions: `Ravel`-wrapped
  stranded literals (the identical column-major-storage fix), a
  `zero_base_index` helper for `!`'s 0-based convention (the exact same
  `-1` correction J's own `i.` needs, since the shared JS runtime's
  `IndexGenerator` codegen hardcodes APL's 1-based convention), and the same
  `MAX_EXPR_DEPTH` (256) defense-in-depth recursion guard.
- **All 17 of Q's primitive verb glyphs** (MA11 §4's full table): the 12
  scalar-dyadic ones (`+ - * % & | = <> < <= >= >`) lower unconditionally to
  `Expr::ElementwiseOp`; the 6 with a monadic meaning map onto a mix of
  reused (`neg`/`recip`/`floor`, already implemented for APL) and 5
  genuinely new `BuiltinCall` names (`q_first`, `q_where`, `q_reverse`,
  `q_not`) with no APL/J precedent at all — Q's own monadic/dyadic pairing
  table is genuinely different from both (e.g. `*` is "first/multiply" here,
  not J's "sign/multiply"). Monadic `+` (flip) reduces to plain identity in
  this cut, since no primitive can ever construct a rank-2 value at all (no
  reshape/matrix-literal exists in scope) — "transpose if rank 2, else
  identity" collapses to unconditional identity for every value this
  frontend can ever produce.
- **Two deliberate node-reuse decisions, saving two would-be-new builtins**:
  monadic `,` (enlist) reuses `Expr::Ravel` (provably identical to ravel on
  every reachable rank-0/rank-1 value in this cut — see `src/lower.rs`'s
  module doc comment's "Enlist reuses Ravel" section, and
  `q_runtime::builtins::enlist`'s own doc comment, which discloses the
  identical simplification runtime-side); dyadic `,` (join) reuses
  `Expr::Catenate` (`q_runtime::builtins::join`'s case-by-case shape is
  IDENTICAL to `apl_runtime::builtins::catenate`'s, confirmed by direct
  side-by-side comparison).
- Monadic `#` (tally) reuses J's existing `"tally"` builtin **as-is** — its
  rank0→1/rank1→n/rank2→r convention already matches
  `q_runtime::builtins::tally` exactly, no new code needed at all.
- Adverbs `'`/`/`/`\` (each/reduce/scan): same restriction as APL/J (reduce/
  scan only apply to the 12 scalar-dyadic primitives); `each` degenerates to
  direct application for the 4 primitives whose monadic meaning is already
  elementwise (`- % _ ~`) and for all 12 dyadically (mirrors
  `q_runtime::builtins::Prim::each_monadic_supported`/
  `each_dyadic_supported` exactly), a clean lowering error otherwise.
- **Dual list-literal syntax** (MA11 §3 bullet 3): `1 2 3` stranding and
  `(a;b;c)` explicit both lower to the identical `Ravel(ArrayLit(..))`
  shape. A list-literal element that is *syntactically, provably*
  non-scalar (a nested stranded/list literal) or function-valued is a clean
  lowering error (mirrors `q_runtime::eval::Interpreter::eval_list_literal`'s
  identical rejection) — an element that could merely *turn out* to be
  non-scalar/function-valued at runtime (an ordinary variable reference) is
  not statically decidable here any more than it is in `q-runtime` itself,
  so it is embedded as-is.
- **Function literals (`{[x;y] ...}`, and the bracket-omitted implicit
  `x`/`y`/`z` form) — the one lowering surface with no APL/J precedent at
  all** (MA11 §2/§3 bullet 1). Every function literal (whether directly
  assigned to a top-level name, or an inline literal applied the moment
  it's written) becomes its own genuine `semantic_ir::Function` with
  `captures: vec![]` (Q's `QFn::Lambda` captures nothing at all — MA11 §2/
  `q_runtime::eval::Lambda`'s own doc comment — so this crate needs none of
  Python's/Ruby's own free-variable-analysis/lambda-lifting machinery).
  Three call-site shapes, one shared dispatch decision
  (`Lowerer::expr_to_fnkind`): a NAME resolving to a directly-assigned
  top-level function, or an inline immediately-applied literal, both lower
  to `Expr::DirectCall` (statically known callee); anything else (a
  parameter, an unresolved global, a parenthesised/numeric/list-literal
  term) lowers to `Expr::IndirectCall` through whatever `Expr` it evaluated
  to — this is what makes the genuinely dynamic, higher-order case work
  with **no special-casing at all**: a function value passed as an argument
  and called through a parameter inside the callee's own body (mirrors
  `q_runtime`'s own
  `passing_a_function_value_as_an_argument_to_another_function` test
  exactly) just falls out of the same dispatch rule. See `src/lower.rs`'s
  module doc comment's "Function literals" section for the full design.
- **Top-level scope is `Scope::Global`, not `Scope::Local`** — a real,
  disclosed divergence from `j-to-semantic-ir`'s/`apl-to-semantic-ir`'s own
  convention, forced by a genuine Q semantic those two languages never
  needed to represent: a function literal's body can read a plain array
  variable assigned at the top level, from inside a **separate**,
  independently-compiled `Function` (MA11 §2/`q_runtime::eval::Lambda`'s own
  doc comment: "resolves any non-parameter name against the *global* frame
  at call time"). A `main`-local JS `let` is invisible to a sibling JS
  function, so every top-level Q variable becomes a genuine
  `semantic_ir::Global` instead (`init_function: "main"`), matching
  `semantic-ir-to-javascript::emit_globals`'s own "globals are module-level
  `let`s" convention. This also simplifies top-level assignment relative to
  J/APL: since `emit_globals` pre-declares every global before any function
  runs, there is no `LetStarBinding`-vs-`Assign` first-occurrence
  distinction needed at the top level at all — every top-level write is
  simply `Stmt::Assign { scope: Global, .. }`. A function-literal body's own
  *internal* assignment is still an ordinary `Scope::Local`/`Scope::Param`
  binding, scoped to that one function alone, with the usual
  first-occurrence `LetStarBinding`-vs-`Assign` convention.
- **Disclosed simplification: declared arity vs. call-site arity.**
  `q_runtime::eval::Interpreter::call_lambda` binds arguments positionally
  and does not require every declared parameter to receive one — a function
  declared with more parameters than a call site supplies (most commonly the
  implicit `x`/`y`/`z` form called monadically) simply leaves the extras
  unbound, erroring only if referenced (confirmed directly against
  `q-parser`'s own doc-comment example). `semantic_ir::Function`'s
  default-parameter model has no "declared but left unbound" concept, so
  every parameter after the first gets a disclosed sentinel default
  (`IntLit(0)`), accepting the common, well-formed case (fewer arguments
  than declared, body never reads the missing ones) at the cost of silently
  defaulting to `0` rather than truly erroring in the narrow case a program
  actually reads an unsupplied trailing parameter. The OTHER direction — too
  MANY arguments for the callee's own declared parameter count — is still a
  hard, disclosed lowering error, mirroring `call_lambda`'s own "function
  takes at most N parameter(s)" rejection exactly.
- Rejects (cleanly, at lowering time): the 6 comparison primitives used
  monadically; a reduce/scan-decorated primitive used dyadically; `! , # _
  ~` decorated with an adverb (none is a scalar dyadic verb); dyadic `!`
  (dict creation, explicitly deferred, MA11 §4); a **nested** function
  literal (one `{...}` appearing anywhere inside another's own body —
  mirrors `q_runtime::eval::Interpreter::build_lambda`'s identical
  `inside_a_call` rejection).

### Changed (shared `semantic-ir-to-javascript` crate — additive only)

Five of Q's primitives have no APL/J precedent and no existing `BuiltinCall`
runtime support (`q_first`, `q_where`, `q_reverse`, `q_not`, plus dyadic
`q_take`/`q_drop`/`q_match` — 7 new dispatch-table names in total). Ported
1:1 from `q_runtime::builtins::{first, where_indices, reverse, not_, take,
drop_, match_}` directly into `semantic-ir-to-javascript`'s `ArrayRt` IIFE
and its `builtins` dispatch table (mirroring exactly how J's own `tally`/
`replicate`/`monadicExp` were added when that crate needed them) — every
change is a **new**, additive function/dispatch-table entry; no existing
line was modified. Confirmed no regression: the shared crate's own full test
suite (257 tests across `src/lib.rs`, `emit.rs`'s embedded tests,
`tests/sir22_array.rs`, `tests/sir23_eval_depth_guard.rs`,
`tests/sir23_symbolic.rs`) and every one of its other downstream consumers'
test suites (`apl-to-semantic-ir`, `derive-to-semantic-ir`,
`j-to-semantic-ir`, `javascript-to-semantic-ir`, `macsyma-to-semantic-ir`,
`maple-to-semantic-ir`, `matlab-to-semantic-ir`, `octave-to-semantic-ir`,
`reduce-to-semantic-ir`, `scilab-to-semantic-ir`, `sir-conformance`,
`wolfram-to-semantic-ir`) still pass unchanged.

### Found, NOT fixed here (shared `semantic-ir-to-javascript` crate — pre-existing, follow-up task)

`tests/oracle.rs` confirms (directly running generated JavaScript through
`node`) the exact same shared-crate display gap `j-to-semantic-ir/tests/
oracle.rs`'s own "Bug A" already found for J, now confirmed for Q too:
`semantic-ir-to-javascript` has exactly two per-source-language negative-
number display flags (`SIR_DISPLAY_APL_HIGH_MINUS`, `SIR_DISPLAY_J_UNDERSCORE`)
and no third for Q. A bare/boxed scalar negative result renders via plain
`String(v)` when neither flag is set — which happens to already BE Q's own
ASCII-minus convention, so **that path has no bug for Q at all** (unlike
J, which needs a leading underscore neither flag produces) — but a genuine
NDArray result (any `ElementwiseOp`/`Reduce`/`Scan`/`Ravel`/`Catenate`, or
`neg`'s own rank ≥ 1 branch) reaches `ArrayRt.fmtNum`, which renders APL's
high-minus `¯` for any negative value whenever `SIR_DISPLAY_J_UNDERSCORE`
is unset — unconditionally, with nothing gating it for Q either. Confirmed
directly: `3-4` prints `¯1`, not `-1`; `-/1 2 10` prints `¯11`, not `-11`.
5 of this crate's 33 oracle-corpus cases hit this and are marked
`known_bug` accordingly (only `ground_truth` is asserted for those; the
`compiled`-side assertion is skipped, matching this repo's established
"found, NOT fixed here" discipline for a bug in a crate consumed by many
other frontends). Not fixed in this PR — needs a third
`SIR_DISPLAY_Q_ASCII`-shaped flag (or simply extending the "no flag set"
default to `ArrayRt.fmtNum` too) in `semantic-ir-to-javascript` itself.

### Testing

- `tests/test_lower.rs` — 56 unit tests asserting exact `Expr`/`Function`
  shapes for every grammar production: all 17 primitives (monadic and
  dyadic), each/reduce/scan, dual list-literal syntax and its two rejection
  cases, chained assignment and `Scope::Global` top-level scoping, and the
  full function-literal surface (named + inline definitions, implicit
  `x`/`y`/`z` param defaults, explicit param lists, multi-statement bodies,
  `DirectCall`/`MakeClosure`/`IndirectCall` dispatch including the
  higher-order case, a top-level global read from inside a function body,
  the nested-function-literal rejection, and the too-many-/too-few-argument
  arity checks).
- `tests/test_validator.rs` — 10 tests confirming every lowered module
  passes `semantic_ir::validate` and is accepted by
  `semantic-ir-to-javascript`'s `Backend::check_module`/`compile()`,
  including the function-literal machinery and a new-`BuiltinCall`-name
  module (the SIR validator has no builtin-name whitelist, so this also
  confirms validation is orthogonal to runtime-dispatch-table completeness).
- `tests/e2e_node.rs` — 12 tests actually running compiled Q programs
  through `node`, weighted toward the function-literal machinery (a named
  function, an inline immediately-applied literal, a function calling
  another already-defined function, and the genuinely higher-order
  `MakeClosure`+`IndirectCall` case) plus all 7 new `q_*` builtins and the
  reused `tally`/`Catenate`, every expected value chosen to be positive so
  none are affected by the display gap above.
- `tests/oracle.rs` (HML01 §7) — oracle/golden testing: the same Q source
  run through **two** independent implementations (`q-runtime`, the native
  ground truth, vs. this crate → `semantic-ir-to-javascript` → real `node`)
  and diffed, 33-case corpus. 28 cases agree end-to-end (fully verified,
  both ground truth and compiled output checked against the expected
  value); 5 are `known_bug` (ground truth still checked, compiled-side
  skipped) per the shared-crate display gap above.
- 1 doctest (`src/lib.rs`'s own `compile_source` example).
- Adds `q-to-semantic-ir` to `code/packages/rust/Cargo.toml`'s workspace
  `members`.
