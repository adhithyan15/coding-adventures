# Changelog

## [0.1.2] - 2026-07-21

### Fixed (upstream — `semantic-ir-to-javascript` 0.47.0, not this crate's own lowering)

`semantic-ir-to-javascript`'s SIR23 addendum item 1 (`Symbolic.evalTerm`
arithmetic/comparison/logic folding, that crate's own `CHANGELOG.md`
`[0.47.0]` entry) lands a real, scoped evaluator for the symbolic domain
— the same shared-crate fix `derive-to-semantic-ir`'s own `CHANGELOG.md`
`[0.1.2]` entry documents, generalized across every Stream B frontend
that uses `symbolic-vm::SymbolicBackend` unchanged (confirmed:
`reduce-runtime` is one of them). This crate's own `src/lower.rs` is
completely unchanged — every fix here is entirely upstream, confirmed by
`tests/test_lower.rs`'s ~59 pre-existing shape assertions still passing
unmodified — but 15 of `tests/oracle.rs`'s 38 `known_bug` cases now
genuinely agree end-to-end (run and confirmed, not guessed), so their
markers flip to `known_bug: None` here:

- `multiplication_binds_tighter_than_addition`, `parens_override_
  precedence`, `power_is_right_associative` — `Add`/`Mul`/`Pow` numeric
  folding.
- `unary_minus_binds_looser_than_power`, `negative_integer_literal` —
  `Neg`'s numeric fold, correcting the same mistaken "would still need
  display-convention work" assumption `derive-to-semantic-ir`'s own
  `[0.1.2]` entry describes (a folded `Neg` on a numeric literal is
  already a plain integer term, not a compound one).
- `exact_integer_division_folds_to_an_integer`, `inexact_division_folds_
  to_a_rational` — `Div`'s exact-integer/exact-rational folding.
- `additive_identity_simplifies_a_free_symbol` — the `x + 0 -> x`
  identity law.
- `comparison_true`, `comparison_false`, `less_equal_boundary_is_true`,
  `not_equal_is_true` — comparison folding to the `True`/`False` symbol
  (`not_equal_is_true`, Reduce's own `neq` keyword, has no Derive
  equivalent, so this is one more flip than Derive's 14).
- `and_short_circuits_to_true`, `three_term_and_chain_folds_n_ary`,
  `not_negates_a_true_comparison` — `And`/`Or`/`Not` folding (including
  the n-ary chain fold).

One case's `known_bug` **reason string is corrected in place** (stays
`Some`, since it still disagrees, but for a narrower, now-accurate
reason): `list_of_expressions_evaluates_elementwise` (`{1+1, 2*3, 2^3}`)
now compiles to and evaluates as `List(2, 6, 8)` — `List` still has no
handler of its own, but `evalTerm`'s applicative-order argument
evaluation folds each element for free regardless — so the ORIGINAL
reason ("each element is itself an unfolded Add/Mul/Pow term... never
{2, 6, 8}") is no longer true; only Reduce's own `{...}` curly-brace
display convention is still missing (a separate, not-yet-scheduled
display-convention item — Reduce's own convention isn't part of this
rollout's item 4, which is Derive-only per the addendum's "Scope
boundary" section).

The remaining 22 `known_bug` cases are unaffected and still fail for
their own already-documented reasons: held-form execution (`Assign`/
`Define`/`If`, item 2 of the rollout — 8 cases), Reduce's own SIR23
display convention (not part of this rollout at all — 8 cases), and the
THIRD, Reduce-specific gap (`First`/`Append`/free-symbol `Cons`/
`CompoundExpression` having no `symbolic-vm` handler at all, a
native-runtime gap one layer further back than `semantic-ir-to-
javascript` — 5 cases, one overlapping the held-form gap). None of those
are in this upstream fix's scope, so none are force-flipped.

## [0.1.1] - 2026-07-21

### Added

- **`tests/oracle.rs` — HML01 §7 oracle/golden testing, cross-checking
  `reduce-runtime` (ground truth) against `reduce_to_semantic_ir::
  compile_source` → `semantic_ir::Module` → `semantic_ir_to_javascript::
  compile` → a real `node` process.** The direct Reduce sibling of
  `derive-to-semantic-ir/tests/oracle.rs`, completing HML01 §5's Stream B
  rollout note *for `reduce-to-semantic-ir` specifically* (that spec text
  is updated by this PR; `wolfram-to-semantic-ir`/`macsyma-to-semantic-ir`/
  `maple-to-semantic-ir` are unaffected and still have no oracle file of
  their own). 38-case corpus: bare integer/float/symbol atoms; ordinary
  operator precedence and right-associative `^`/`**`; unary minus binding
  looser than `^`; exact-integer vs. genuine-rational division; an
  additive-identity simplification; assignment (plain and list-valued)
  read back by a later statement; single- and multi-parameter procedure
  definition/call; `if`/`then`/`else` (both branches, plus the two-branch
  no-`else` form's "false → `False`" convention); every comparison/logic
  keyword (`= neq < <= > >= and or not`, including a 3-term `and` chain
  exercising the n-ary logical-chain fold); flat/singleton/elementwise-
  evaluated/empty list literals; list accessors (`first`/`append`); cons
  (`.`), both the literal-list-folding shape and the non-folding shape;
  and a group statement `<< ... >>` exercising in-order side effects.
  `DIF`/`INT`/trig calculus are deliberately absent — MA08 §3's own table
  and `reduce-runtime::lower`'s `standard_function` bridge table confirm
  Reduce's R-4 scope has no calculus/trig bridging at all (unlike Derive),
  so there is nothing in that area to test.
- Adds a dev-dependency on `coding-adventures-reduce-runtime` (this
  frontend's own sibling native-runtime crate) for `tests/oracle.rs`'s
  ground truth only — the non-dev `[dependencies]` section still does not
  depend on it; lowering itself only ever needs the parse-tree shape.

### Found, NOT fixed here — already-documented shared-crate gaps, cited not
### re-discovered

Building this corpus confirmed Reduce hits the **identical**
`semantic-ir-to-javascript` gaps `derive-to-semantic-ir`'s own oracle PR
(#8754) found and documented — see that crate's `CHANGELOG.md`'s `[0.1.1]`
entry and, now generalized across every Stream B frontend,
`SIR23-symbolic-pattern-semantic-ir.md`'s own "Addendum — SIR23 symbolic
evaluator + per-language display convention" section (which explicitly
confirms `derive-runtime`, `reduce-runtime`, and `maple-runtime` all
construct `SymbolicBackend::new()` completely unchanged, so all three hit
this exact gap for the exact same reason). This changelog entry cites that
finding, it does not re-derive it:

- **No SIR23 evaluation or simplification of any kind.** `Expr::SymApply`
  compiles unconditionally to `__Sir.Symbolic.apply(head, [args])` — a
  pure, inert term constructor. 24 of this crate's 38 new oracle cases hit
  this gap directly (every arithmetic/comparison/logic/assignment/
  procedure-call/`if` case beyond a bare literal/symbol atom), plus one
  more (`negation_of_a_free_symbol`) that needs no fold but still fails to
  print correctly, and one (`group_statement_evaluates_side_effects_in_
  order`) that combines this gap with the Reduce-specific gap below.
- **No per-source-language SIR23 display convention.** Even a term that
  WAS already fully reduced still prints wrong: `Symbolic.
  toDisplayString` renders every compound term generically as
  `head(args, ...)`, with no infix, no `{...}` curly-brace convention, no
  `and`/`or`/`not`/`neq` lowercase-keyword convention, and no
  case-bridging back to Reduce's own lowercase builtin spelling
  (`reduce-runtime::printer::print_reduce` reverses all of these). 5 of
  this crate's 38 new oracle cases (`equation_with_a_free_variable_stays_
  symbolic`, `flat_list_literal`, `singleton_list_literal`, `empty_list_
  literal`, `cons_onto_a_literal_list_folds_at_lowering_time`) hit ONLY
  this gap.

### Found, NOT fixed here — a THIRD, Reduce-specific gap, genuinely
### different from the two shared `semantic-ir-to-javascript` gaps above

Unlike Derive, three of this crate's oracle cases (`first_of_a_list_has_
no_shared_vm_handler`, `append_of_two_lists_has_no_shared_vm_handler`,
`cons_of_two_free_symbols_has_no_shared_vm_handler`) hit a gap **one layer
further back in the pipeline than `semantic-ir-to-javascript`**: MA08 §5
and `reduce-runtime`'s own module doc comment already disclose that
`symbolic_vm::handlers::build_handler_table` (the shared *native* Rust
evaluator, not the JS backend) has no handler at all for
`CompoundExpression`/`First`/`Second`/`Third`/`Rest`/`Part`/`Append`/
`Reverse`/a non-folding `Cons` — confirmed empirically by this crate's own
oracle corpus: `reduce-runtime::eval("first({1, 2, 3});\n")` itself
returns the unevaluated string `"first({1, 2, 3})"`, not a numeric or list
result. For these specific cases, the ground truth is ALREADY
unevaluated, so the only actual disagreement between it and the compiled
side is the display-convention gap above, not a missing evaluation the
compiled side alone is failing to perform. `group_statement_evaluates_
side_effects_in_order` combines this THIRD gap (the outer
`CompoundExpression` never collapses to its last statement's value on
EITHER side) with the ordinary SIR23 evaluation gap (the compiled side's
inner `Assign` never binds, unlike the ground truth's, where it genuinely
does).

### Known limitations

- **No local lowering bugs were found in this pass.** This crate's
  lowering was already independently verified against `reduce-runtime::
  lower`'s identical dispatch table (0.1.0 entry below), and `tests/
  test_lower.rs`'s 59 shape-assertion tests (unmodified by this PR, all
  still passing) already cover every grammar production directly — an
  oracle test that can only ever compare unevaluated term shapes (per the
  gaps above) surfaces no new class of bug beyond what those direct shape
  assertions already check.
- `tests/oracle.rs` performs the same test-local `wrap_top_level_in_print`
  transformation `derive-to-semantic-ir/tests/oracle.rs` needed (see that
  file's own module doc for the full rationale): `reduce_to_semantic_ir::
  compile_source` itself is unchanged and still emits no `print`/
  `console.log` of its own for any other caller — `tests/e2e_node.rs`'s
  own "no `disp`-equivalent stdout" design note is still accurate.
- Unlike Derive's own oracle file, `ground_truth` here needs **no**
  worksheet-index-prefix-stripping step: `reduce-runtime::eval`'s output
  has no numbered-input convention at all (confirmed directly against
  `reduce-runtime`'s own module doc and by this file's own probe run), so
  its raw per-line output is already directly comparable to the compiled
  side's `console.log` output.
- Given the two shared-crate gaps plus the one Reduce-specific gap above,
  only 4 of the 38 corpus cases are `known_bug: None` (bare integer/float/
  symbol literals) — the same "clean" fraction `derive-to-semantic-ir`'s
  own oracle PR found (4 of 38), confirming this is the actual, current
  state of the shared SIR23 JS backend, not a shortfall specific to either
  frontend's corpus design.

## [0.1.0] - 2026-07-19

### Added

- Initial `reduce-to-semantic-ir` frontend crate (HML01 Stream B), the
  **fourth** to target SIR23 (symbolic-expression/pattern domain), sibling
  to `wolfram-to-semantic-ir`, `macsyma-to-semantic-ir`, and
  `derive-to-semantic-ir`: `compile`/`compile_source` lowering
  `coding-adventures-reduce-parser`'s `GrammarASTNode` CST into a
  `semantic_ir::Module`.
- Design: retargets `reduce-runtime`'s own `lower_node` rule-name dispatch
  (which already lowers this exact CST to `symbolic_ir::IRNode`) onto
  `semantic_ir::Expr`'s SIR23 vocabulary (`SymSymbol`/`SymApply`) instead —
  much of the shape is a direct copy of `derive-to-semantic-ir`'s own
  lowering (`reduce-runtime`'s own module doc comment says so explicitly),
  since Reduce, like Derive, has no `f[x]`-universal-application syntax and
  no pattern/rewrite-rule vocabulary in this subset (MA08 §4 defers `let`
  rules).
- Covers all of `reduce-parser`'s currently-implemented grammar
  productions (`program`/`statement_line`/`if_expr`/`group_expr`/
  `assignment`/`logical_or`/`logical_and`/`logical_not`/`comparison`/
  `cons`/`additive`/`multiplicative`/`unary`/`power`/`postfix`/`atom`/
  `list_literal`/`group`/`arglist`).
- **Scope boundary, disclosed from day one, verified empirically against
  `code/grammars/reduce/reduce.grammar` and `reduce.tokens`** (not just
  trusted from `reduce-runtime`'s own doc comment, per this repo's
  verify-before-implementing discipline): Reduce's grammar has no
  pattern-matching or rewrite-rule syntax at all (no `_`/blank, no `x_`
  named pattern, no `->`/`:>` rule arrow, no `/.`/`//.` replacement, and no
  `STRING` token at all) — this crate therefore only ever constructs
  `Expr::SymSymbol`/`Expr::SymApply` (plus reused `IntLit`/`FloatLit`),
  never `Expr::StrLit`, `SymPatternBlank`/`SymPatternNamed`/`SymRule`/
  `SymReplaceAll`, and never observes `Feature::PatternMatching`. This also
  means `measure_depth_iterative`/`drop_iterative` only need a match arm
  for `Expr::SymApply` — `If`/`CompoundExpression`/`Cons` are all
  `SymApply` with a different head symbol, not new `Expr` variants.
- **Three genuinely new constructs beyond Derive's grammar**, each
  retargeting `reduce-runtime::lower`'s own identical logic:
  - An expression-shaped `if` (`if_expr = "if" expr "then" expr [ "else"
    expr ]`) — lowers to `If(cond, then[, else])`, usable anywhere an
    `expr` can appear (including as a `:=` right-hand side), unlike
    anything in Derive's grammar.
  - A group statement `<< s1; s2; ... >>` (`group_expr`) — lowers to
    `CompoundExpression(s1, s2, ...)`.
  - Cons (`a . b`, `cons = additive [ DOT cons ]`) — `fold_cons` folds a
    cons onto a structurally literal `List` RHS directly into one flat
    `List` (the one shape MA08 §3 documents); a non-list RHS lowers to a
    bare `Cons(a, b)` head, a disclosed, documented gap mirroring
    `reduce-runtime::lower::fold_cons` exactly.
  - Lists (`{a, b, c}`, curly braces per MA08 §1/§3) are always flat (no
    row/matrix shape — matrices are out of Reduce's scope, MA08 §4), so
    `lower_list_literal` reuses `lower_arglist` directly instead of
    `derive-to-semantic-ir::lower_vector`'s row-counting split.
- **Confirmed and reused a REAL divergence from MA08 §3's own prose**:
  the spec's table spells arithmetic's "Lowers to" column as `Plus`/
  `Subtract`/`Times`/`Power`, even describing `a/b` expanding to
  `Times[a, Power[b,-1]]` and `-a` to `Times[-1,a]` — **none of those
  spellings exist in `symbolic-ir`**, confirmed directly
  (`grep -n '"Plus"\|"Subtract"\|"Times"\|"Power"' symbolic-ir/src/lib.rs`
  returns nothing). The REAL heads — what `symbolic_vm::handlers::
  build_handler_table` actually wires, and what `reduce-runtime::lower`
  itself already uses — are `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`, the exact
  same heads `derive-to-semantic-ir`/`macsyma-to-semantic-ir` use. This
  crate reuses those same real heads (the identical `symbolic_ir`
  constants), NOT new `Plus`/`Subtract`/`Times`/`Power` string literals —
  a disclosed, deliberate divergence from the spec's literal prose
  (already corrected in MA08's own changelog-style note), not new-head
  invention.
- **Confirmed and reused a REAL gap**: `CompoundExpression`, `First`,
  `Second`, `Third`, `Rest`, `Part`, `Append`, `Reverse` (and a non-folding
  `Cons`) have no evaluation handler in the shared `symbolic_vm::handlers::
  build_handler_table` — `reduce-runtime` reuses the shared backend
  unchanged rather than building a bespoke one, per its own design
  mandate. Largely moot for this crate (it never evaluates anything, per
  the "everything is data" design every SIR23 frontend shares) — confirmed
  directly by reading `semantic-ir-to-javascript/src/emit.rs`'s SIR23
  codegen, which lowers `Expr::SymApply` to
  `__Sir.Symbolic.apply(head, [args...])` uniformly for ANY head spelling,
  with no per-head special-casing. This crate reuses the exact head
  spellings `reduce-runtime` uses for these, via its own locally-defined
  `pub const`s (`COMPOUND_EXPRESSION`, `CONS`, `FIRST`, `SECOND`, `THIRD`,
  `REST`, `PART`, `APPEND`, `REVERSE` — not exported by `symbolic-ir`, and
  this crate does not depend on `reduce-runtime` itself, mirroring the
  same "locally-defined pub const, spelled to match a sibling crate's
  constant" pattern `macsyma-to-semantic-ir` needed for its own
  `WHILE_HEAD`/`FOR_EACH_HEAD`/`BLOCK_HEAD`/`RETURN_HEAD` constants).
- `h(l, m) := body` (LHS lowers to `SymApply{head: SymSymbol(_), ..}`)
  lowers to `Define(h, List(l, m), body)`; a bare `x := body` lowers to
  `Assign(x, body)` — disambiguated purely by the lowered LHS's shape,
  since Reduce's grammar has exactly ONE assignment token (`:=`). Unlike
  Derive's self-referential `assignment = logical_or [ ASSIGN assignment
  ]`, Reduce's right-hand side is the WIDER `expr` production (a
  grammar-level divergence `reduce.grammar`'s own comment discloses, not
  an oversight this crate works around) — `x := if a>0 then 1 else -1`
  and `x := << a:=1; a+1 >>` both parse and lower directly through the
  same `if_expr`/`group_expr` dispatch, no special-casing needed.
- Reduce's `neq` (a `KEYWORD`-typed token, matched by literal value
  alongside the four symbolic comparison token *types*) lowers to
  `NotEqual` — a comparison Derive's grammar has no equivalent token for
  at all.
- Recursion-depth hardening applied from day one, proactively carried over
  from `wolfram-to-semantic-ir`'s, `macsyma-to-semantic-ir`'s, and
  `derive-to-semantic-ir`'s own security-review history, even though
  neither `reduce-parser` nor `reduce-runtime` (the retarget source)
  applies any of these guards themselves:
  - `MAX_EXPR_DEPTH` (256), the same value and reasoning as the sibling
    SIR23 frontends, kept for family-wide consistency even though
    `reduce-parser`'s own cap (128) is lower than `derive-parser`'s (200)
    — this constant bounds a different axis (this crate's own
    chain-folding budget) than the parser's CST-nesting cap.
  - `check_chain_length` caps every flat operator-chain fold (`additive`/
    `multiplicative`/`logical_or`/`logical_and`) before any tree is built
    — confirmed these ARE flat EBNF repetitions (not right-recursion)
    directly against `reduce-parser`'s own `MAX_RULE_DEPTH` doc comment,
    which measured an uncapped parser accepting one million repeated
    items with zero crashes for exactly this shape.
  - `check_postfix_chain_length` caps chained call application
    (`f(x)(y)(z)…`) — like Derive's postfix (and simpler than Wolfram's
    cumulative-budget variant), Reduce's `postfix` has only one suffix
    shape, so a plain per-chain group count is already an exact bound.
  - `check_apply_arg_count` caps `arglist`/`list_literal` element counts
    **and** `group_expr`'s flat `{ (SEMI|DOLLAR) expr }` statement-
    sequence length — a defense-in-depth allocation-size backstop, mirrors
    `derive-to-semantic-ir`'s identical reuse of this one guard across
    `arglist` and vector-row counts.
  - `measure_depth_iterative`/`drop_iterative` — the authoritative,
    construction-composition-independent iterative depth check and
    iterative teardown, run once per top-level statement.
  - **No additional lowering-side guard is needed** for Reduce's five
    genuinely self-referential (right-recursive) productions —
    parenthesised nesting, the `:=` chain, the `if`/`else` chain, the cons
    (`.`) chain, and the power (`^`) chain — since `reduce-parser`'s own
    `MAX_RULE_DEPTH` (128; binding constraint measured at a 179-rule-frame
    cons-chain floor, ~28.5% margin) already bounds how deep any of these
    can nest in the CST this crate ever receives. Verified with dedicated
    regression tests: a 5,000-level deep cons chain and a 5,000-level deep
    `if`/`else` chain are both cleanly rejected (by the parser, surfaced
    as a clean `Err` through `compile_source`), never crash.
- `Feature::Floats` regression avoided proactively: `number_literal_expr`
  is an instance method (not a free function) specifically so every
  `FloatLit`-constructing branch can call `self.observed.add(Feature::
  Floats)` immediately — this is a confirmed, previously-shipped bug in
  both `matlab-to-semantic-ir` and `wolfram-to-semantic-ir`.
- `compile_source` needs no worker-thread stack enlargement, unlike
  `wolfram-to-semantic-ir::compile_source`: `reduce-parser`'s own
  `MAX_RULE_DEPTH` (128) is already documented safe on a bare default
  (~2 MiB) stack with comfortable margin (28.5% below its own measured
  crash floor) — mirrors `macsyma-to-semantic-ir`'s/`derive-to-semantic-
  ir`'s simpler `compile_source` shape.
- `tests/e2e_node.rs` — written directly against the *current* SIR23 JS
  backend state (real `__Sir.Symbolic.*` codegen, confirmed by reading
  `derive-to-semantic-ir`'s current test bodies, not module doc comments)
  — compiles and runs representative Reduce programs (arithmetic, a
  procedure definition+call, assignment, list accessor calls, lists/cons,
  `if` expressions, a group statement, a multi-statement program) through
  `node`, including constructs with no shared-VM evaluation handler
  (`CompoundExpression`, list accessors, a non-folding `Cons`) — proving
  the SIR23 codegen path accepts and executes them as pure data
  construction regardless of runtime evaluability.
- 78 tests: 59 unit tests over exact `Expr` shapes for every grammar
  production plus DoS-guard regressions (flat-chain, chained call
  application, a wide list literal, a wide group statement, a deeply
  parenthesised expression, a deep cons chain, a deep `if`/`else` chain,
  and exact-boundary cases at 256/257) and the `Feature::Floats`
  regression, 10 validator/capability-acceptance tests (including the two
  no-shared-VM-handler constructs), 8 e2e `node`-execution tests (skip
  cleanly if `node` is unavailable), 1 doctest.
- Adds `reduce-to-semantic-ir` to `code/packages/rust/Cargo.toml`'s
  workspace `members` and marks it done in
  `HML01-math-to-semantic-ir.md`'s language list and Stream B rollout
  note, closing the `reduce-to-semantic-ir` gap that note previously
  called out as an open follow-on item (only `maple-to-semantic-ir`
  remains open now).
