# Changelog

## [0.1.0] - 2026-07-20

### Added

- Initial `maple-to-semantic-ir` frontend crate (HML01 Stream B), the
  **fifth and final** to target SIR23 (symbolic-expression/pattern
  domain), sibling to `wolfram-to-semantic-ir`, `macsyma-to-semantic-ir`,
  `derive-to-semantic-ir`, and `reduce-to-semantic-ir`: `compile`/
  `compile_source` lowering `coding-adventures-maple-parser`'s
  `GrammarASTNode` CST into a `semantic_ir::Module`. Closes Stream B's
  currently-tracked language list — every math-CAS language HML01 names
  today now has a shipped SIR23 frontend.
- Design: retargets `maple-runtime`'s own `lower_node` rule-name dispatch
  (which already lowers this exact CST to `symbolic_ir::IRNode`) onto
  `semantic_ir::Expr`'s SIR23 vocabulary (`SymSymbol`/`SymApply`)
  instead — much of the shape is a direct copy of
  `reduce-to-semantic-ir`'s own lowering (`maple-runtime`'s own module
  doc comment says so explicitly: both languages are "surface operators +
  `head(args)` calls" with no pattern/rewrite-rule vocabulary in this
  subset).
- **Scope boundary, disclosed from day one, verified empirically against
  `code/grammars/maple/maple.grammar` and `maple.tokens`** (not just
  trusted from `maple-runtime`'s own doc comment, per this repo's
  verify-before-implementing discipline): Maple's grammar has no
  pattern-matching or rewrite-rule syntax at all (no `_`/blank, no `->`/
  `:>` rule arrow — Maple DOES have an `ARROW` token, but it appears in
  exactly one production, `arrow_def`, MA09 §3's function-definition
  spelling, never as a pattern-rule arrow — no `/.`/`//.` replacement,
  and no `STRING` token at all) — this crate therefore only ever
  constructs `Expr::SymSymbol`/`Expr::SymApply` (plus reused
  `IntLit`/`FloatLit`), never `Expr::StrLit`,
  `SymPatternBlank`/`SymPatternNamed`/`SymRule`/`SymReplaceAll`, and
  never observes `Feature::PatternMatching`. This also means
  `measure_depth_iterative`/`drop_iterative` only need a match arm for
  `Expr::SymApply` — `If`/`Assign`/`Define`/`Set` are all `SymApply` with
  a different head symbol or bracket, not new `Expr` variants.
- **A REAL structural difference from Reduce: the dispatch is genuinely
  SPLIT, not blindly copied.** `reduce.grammar`'s `expr = if_expr |
  group_expr | assignment` sits at the top of Reduce's expression
  grammar, so `if`/`:=` are reachable from *every* `expr` position.
  `maple.grammar` draws a hard line Reduce's own grammar does not:
  `statement = if_expr | assignment` sits in its OWN nonterminal, never
  reachable from `expr` at all. This crate's `lower_node` is still one
  shared dispatch table (mirroring `maple-runtime::lower::lower_node`'s
  own single `match`, not two separate Rust functions), but the
  grammar's own reachability graph is what enforces the real divide:
  `lower_if`/`lower_assignment` are only ever reached from the top-level
  statement loop, never nested inside an arithmetic/comparison/logical
  operand. Regression tests confirm `x := if a then 1 end if;` and `a :=
  b := 5;` both fail to compile (syntax errors), matching
  `maple-parser`'s own identically-named regression tests.
- **Assignment is deliberately NARROWER than Reduce's/Derive's own
  call-shaped-LHS disambiguation**: `assignment = NAME ASSIGN ( arrow_def
  | expr ) | expr` — the LHS is a bare `NAME` token, full stop. Maple's
  `f(x) := expr` spelling means something narrower in real Maple (a
  remember-table patch onto an existing procedure, MA09 §1/§4) than
  Reduce's/Derive's general-definition idiom of the identical shape, so
  `maple.grammar` makes it fail to *parse* at all — confirmed by a
  regression test (`remember_table_spelling_is_rejected`). This crate
  never needs "is the LHS call-shaped" logic. Instead, a SEPARATE
  `arrow_def`/`arrow_params` production (`f := (x, y) -> x + y` / `f :=
  x -> x^2`) is the general-purpose function-definition spelling —
  `lower_arrow_def`/`lower_arrow_params` lower it to `Define[f,
  List[params...], body]`, the same `Define` shape Derive's/Reduce's own
  (differently-spelled) definitions already use. Zero/one/multiple
  parameter arrow-definitions all covered by dedicated tests.
- **`if`/`elif`/`else` right-folds, mirroring Macsyma's elif chain, NOT
  Reduce's simpler 2-or-3-child `if`**: `if_expr = "if" expr "then"
  statement { "elif" expr "then" statement } [ "else" statement ] ( "end"
  "if" | "fi" )` — `lower_if` retargets `maple-runtime::lower_if`'s exact
  collect-pairs-then-fold-right-to-left logic. Because Maple requires an
  explicit close for every `if_expr`, there is no dangling-else ambiguity
  the way Reduce's `if`/`else` chain has to resolve by convention. A new
  `check_elif_chain_length` guard (mirroring `check_chain_length`'s
  reasoning) bounds `elif`-arm count before the fold runs, since
  `maple-parser`'s own doc comment confirms the `{ "elif" ... }`
  repetition is a flat EBNF shape (zero parser-stack cost regardless of
  width) — the fold itself is what builds an N-deep tree.
- **`Set` — a canonical head genuinely new to this repo (MA09 §5)**:
  Maple is the first language here with two distinct bracketed aggregate
  literals — `[a, b, c]` (ordered → `List`, a shared existing head) and
  `{a, b, c}` (unordered → `Set`). `symbolic-vm`'s shared handler table
  has no handler for `Set` (confirmed by `maple-runtime`'s own doc
  comment) — reused the identical local-`pub-const` pattern
  `reduce-to-semantic-ir`/`reduce-runtime` established for their own new
  `COMPOUND_EXPRESSION`/`CONS`/… constants, spelled to match
  `maple-runtime`'s own `SET` constant.
- **`diff`/`int` bridge to `D`/`Integrate`** — a plain two-entry surface-
  name bridge table, mirroring `maple-runtime::surface_head_to_ir`'s
  identical table exactly (no calculus reimplementation).
- **Booleans — the first literal `true`/`false` TOKENS in this CAS
  family**: neither Derive's nor Reduce's grammar has a dedicated boolean
  literal token. `maple.grammar`'s `atom` rule is the first to include
  `"true"`/`"false"` as their own alternatives — bridged to the shared
  backend's pre-bound `True`/`False` symbols, the same bridge
  `macsyma-compiler::lower_token` already uses. **Verified directly**
  that `symbolic-ir` exports no `TRUE`/`FALSE` constants (`grep -n
  '"True"\|"False"\|pub const TRUE\|pub const FALSE'
  symbolic-ir/src/lib.rs` finds nothing but a stray test literal) — uses
  bare string literals, matching `maple-runtime`'s own bridge.
- **`postfix` is NOT chainable — verified, no guard invented for a
  non-existent risk**: `maple.grammar`'s `postfix = atom [ LPAREN
  [arglist] RPAREN ] ;` has a single OPTIONAL call suffix (confirmed
  directly against the grammar file), unlike Reduce's/Derive's REPEATED
  `{ LPAREN [arglist] RPAREN }` chain — `f(x)(y)` is not valid Maple
  syntax in this subset at all. A regression test
  (`postfix_call_is_not_chainable`) confirms `compile_source` rejects it
  as a parse error. No `check_postfix_chain_length`-equivalent guard
  exists anywhere in this crate — the axis it would guard is
  structurally impossible here, not merely bounded by a cap.
- The `;`-vs-`:` statement terminator is deliberately NOT tracked — MA09
  §3's own statement-separator row calls it "a display flag on the
  surrounding session, not an IR node." Unlike `maple-runtime`'s own
  `LoweredStatement`/`Display` REPL-rendering types, this frontend just
  emits every statement as a plain `Stmt::ExprStmt`, mirroring how
  neither `derive-to-semantic-ir` nor `reduce-to-semantic-ir` replicate
  their own native runtimes' prompt/display machinery either.
- Recursion-depth hardening applied from day one, proactively carried
  over from `wolfram-to-semantic-ir`'s, `macsyma-to-semantic-ir`'s,
  `derive-to-semantic-ir`'s, and `reduce-to-semantic-ir`'s own
  security-review history, even though neither `maple-parser` nor
  `maple-runtime` (the retarget source) applies any of these guards
  themselves:
  - `MAX_EXPR_DEPTH` (256), the same value and reasoning as the sibling
    SIR23 frontends, kept for family-wide consistency even though
    `maple-parser`'s own cap (150) is lower than `derive-parser`'s (200)
    — this constant bounds a different axis (this crate's own
    chain-folding budget) than the parser's CST-nesting cap.
  - `check_chain_length` caps every flat operator-chain fold
    (`additive`/`multiplicative`/`logical_or`/`logical_and`) before any
    tree is built — confirmed these ARE flat EBNF repetitions (not
    right-recursion) directly against `maple-parser`'s own
    `MAX_RULE_DEPTH` doc comment.
  - `check_elif_chain_length` — new to this crate (see above).
  - `check_apply_arg_count` caps `arglist` element counts (shared by
    call arguments, list literals, and set literals) **and**
    `arrow_params`'s flat parameter-name count — a defense-in-depth
    allocation-size backstop, mirrors `reduce-to-semantic-ir`'s
    identical reuse of this one guard across multiple flat-`Vec`
    productions.
  - `measure_depth_iterative`/`drop_iterative` — the authoritative,
    construction-composition-independent iterative depth check and
    iterative teardown, run once per top-level statement.
  - **No additional lowering-side guard is needed** for Maple's six
    genuinely self-referential (right-recursive or prefix-recursive)
    productions — parenthesised `group` nesting, list-/set-literal
    nesting, a `not`-prefix chain, a unary-minus-prefix chain, the power
    (`^`) chain, and nested `if`/`end if` (or `fi`) — since
    `maple-parser`'s own `MAX_RULE_DEPTH` (150; binding constraint
    measured at a 218-rule-frame `not`-chain floor, ~31.2% margin)
    already bounds how deep any of these can nest in the CST this crate
    ever receives. Verified with dedicated regression tests: a
    5,000-level deep parenthesised expression, a 5,000-level deep `not`
    chain, and a 5,000-level deep nested-`if`/`end if` chain are all
    cleanly rejected (by the parser, surfaced as a clean `Err` through
    `compile_source`), never crash.
- `Feature::Floats` regression avoided proactively: `number_literal_expr`
  is an instance method (not a free function) specifically so every
  `FloatLit`-constructing branch can call `self.observed.add(Feature::
  Floats)` immediately — this is a confirmed, previously-shipped bug in
  both `matlab-to-semantic-ir` and `wolfram-to-semantic-ir`.
- `compile_source` needs no worker-thread stack enlargement, unlike
  `wolfram-to-semantic-ir::compile_source`: `maple-parser`'s own
  `MAX_RULE_DEPTH` (150) is already documented safe on a bare default
  (~2 MiB) stack with comfortable margin (~31.2% below its own measured
  crash floor) — mirrors `macsyma-to-semantic-ir`'s/`derive-to-semantic-
  ir`'s/`reduce-to-semantic-ir`'s simpler `compile_source` shape.
- `tests/e2e_node.rs` — written directly against the *current* SIR23 JS
  backend state (real `__Sir.Symbolic.*` codegen, confirmed by reading
  `reduce-to-semantic-ir`'s current test bodies, not module doc
  comments) — compiles and runs representative Maple programs
  (arithmetic, an arrow-definition+call, assignment, lists/sets, `if`/
  `elif`/`else`, boolean literals, `diff`/`int` calls, a multi-statement
  program) through `node`, including the `Set` construct with no
  shared-VM evaluation handler — proving the SIR23 codegen path accepts
  and executes it as pure data construction regardless of runtime
  evaluability.
- 86 tests: 66 unit tests over exact `Expr` shapes for every grammar
  production plus statement/expression-split syntax-error regressions
  (chained assignment, the remember-table spelling, `if` as an
  assignment RHS, chained postfix application) and DoS-guard regressions
  (flat-chain, a wide list/set literal, a wide `arrow_params` list, a
  wide `elif` chain, a deeply parenthesised expression, a deep `not`
  chain, a deep nested-`if` chain, and exact-boundary cases) and the
  `Feature::Floats` regression, 11 validator/capability-acceptance
  tests (including the no-shared-VM-handler `Set` construct), 8 e2e
  `node`-execution tests (skip cleanly if `node` is unavailable), 1
  doctest.
- Adds `maple-to-semantic-ir` to `code/packages/rust/Cargo.toml`'s
  workspace `members` and marks it done in
  `HML01-math-to-semantic-ir.md`'s language list and Stream B rollout
  note — this is the LAST entry in Stream B's previously-tracked
  language list, so the rollout note's framing is updated accordingly.
