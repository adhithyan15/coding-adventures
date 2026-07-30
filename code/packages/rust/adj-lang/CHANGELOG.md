# Changelog

## [0.64.0] — 2026-07-30

### Added — RS-3b: `statemachine` grammar + AST + adapter + lowering (ADJ-STATEMACHINE §2, §5)

The native `statemachine` construct now parses, adapts, and lowers its STRUCTURE — the
control-flow sibling of `table`/`formulabook` for long-horizon procedural reasoning
(ADJ-STATEMACHINE RS-3b). **No driver / no execution yet** — that is RS-3c.

- **Grammar** (already regenerated): `statemachine_decl` with `initial`, `state`/`transition`,
  `exit when … yield …`, and `budget N steps`; guards are `( apply | IDENT ) [ relop expr ]`
  and actions `assert term` (the RS-3b minimal subset). Keywords are IDENT-matched literals —
  `.tokens` untouched.
- **AST**: `Statement::StateMachine { name, uses, initial, states, exits, budget, annotations }`
  plus `StateDef` / `TransitionDef` / `ExitDef` / `SmGuard` / `SmAction`. A guard's subject is
  carried as an `ast::Term` (atom for a bare IDENT, compound for an `apply`).
- **Adapter**: `adapt_statemachine` (+ `adapt_sm_state` / `_transition` / `_exit` / `_guard` /
  `_action`), modelled on `adapt_table`, reusing `first_name_not` / `first_named_child` /
  `collect_annotations` / `adapt_use` / `adapt_term` / `adapt_expr` and the same relop mapping
  as `adapt_predicate`.
- **Lowering**: each machine lowers to a validated, provenance-stamped `LoweredStateMachine`
  (`LoweredState` / `LoweredTransition` / `LoweredExit` / `LoweredGuard` / `LoweredAction`),
  exposed on `LoweredProgram::state_machines`. Guards/actions lower through the SAME
  term/compute forms the rest of the language uses (`lower_term` / `lower_expr` /
  `lower_cmp_op`) — no parallel evaluator. Five typed well-formedness errors: `SmMissingInitial`
  (defensive — the grammar already requires `initial`, so an omitted clause is a parse error),
  `SmMissingExit`, `SmBudgetNotPositive`, `SmUnknownState`, `SmMissingProvenance` (the shared
  write gate — a shipped machine must be sourced).

Purely additive; no existing behaviour changes. Covered end-to-end by
`adj-lang-cli/tests/rs3b_statemachine_lower_e2e.rs` (a well-formed machine compiles clean; each
malformed one yields its specific diagnostic).

## [0.63.0] — 2026-07-24

### Added — RS-5f: `mode nearest` table lookup (ADJ-TABLES §3.4)

The lowering mode dispatch now accepts `nearest` alongside `range` and `interpolated`
(all three are built tactics); an unrecognized mode is still `LowerError::LookupUnknownMode`.
`nearest` snaps the query key to the closest tabulated key (nearest-neighbour). Like `range`
(and unlike `interpolated`), it returns the value cell verbatim, so it requires only a numeric
**key** column — the existing shared `LookupNonNumericKeyColumn` check — and does **not** impose
the `interpolated`-only numeric-value-column requirement. Doc comments on `LookupUnknownMode`
updated to list all three modes. Purely additive; no existing behaviour changes.

## [0.62.0] — 2026-07-23

### Added — RS-5d: `mode interpolated` table lookup (ADJ-TABLES §3.3)

The `? lookup <table> <key> = <n> mode interpolated give <val>` tactic is now built — the
piecewise-linear sibling of `mode range`. Lowering changes:

- The mode dispatch accepts `interpolated` alongside `range` (both are built tactics); an
  unrecognized mode is still `LowerError::LookupUnknownMode`. The reserved-mode error
  `LookupModeUnsupported` is **retired** (it existed only to reject `interpolated`).
- `interpolated` additionally validates that the **value** column is numeric in every row —
  the new `LowerError::LookupNonNumericValueColumn` — because it computes on the value cells
  (`v0 + (v1−v0)·(q−k0)/(k1−k0)`); you cannot linearly blend a category label. (`range` returns
  the value cell verbatim and imposes no such check.)

The lowered form is unchanged (`LoweredRangeLookup` carries the mode through); the interpolation
arithmetic itself lives in the CLI's exact-rational tactic (see `adj-lang-cli` 0.23.0).

## [0.61.0] — 2026-07-23

### Added — NUM-6c: the `to_currency(x, code [, places])` money rendering (ADJ-NUMERIC-SUBSTRATE §4.1, §4.3)

```
let due  = to_currency(subtotal + tax, usd)      % → "USD 42.50"
let yen  = to_currency(price, jpy, 0)            % → "JPY 1980"
```

- `to_currency` joins `round_to`/`round_sig`/`to_scientific`/`to_percent` as a built-in
  recognised during `Apply` lowering (same native comma-list surface, no new grammar). It lowers
  to the new `logic_engine::ComputeExpr::ToCurrency` node via a new `ExprAst::ToCurrency(expr,
  code, places)` AST node, under the default half-even mode.
- The **currency code** is the second argument, written as a **bare identifier** (`usd`) — it
  parses as an `ExprAst::Ref` and is read directly at the intercept, never resolved as a slot or
  expanded. Identifiers lex lowercase, so the code is normalized to the canonical uppercase
  ISO-4217 form for the rendered string and the audit record (`usd` → `USD 42.50`). A
  missing/non-identifier code (e.g. a number in the code slot) is a clean compile error.
- The `places` third argument is **optional**: `to_currency(x, code)` uses the documented default
  (`DEFAULT_CURRENCY_PLACES = 2`, the common minor-unit precision). A stated count must be a
  non-negative integer literal (`≥ 0` — `to_currency(x, jpy, 0)`) within the cap
  (`MAX_ROUND_PLACES = 100`); anything else, or the wrong argument count, is a clean compile error.
- All four expression walkers carry the new node (cloning the code string), so it composes inside
  formula bodies, `let`s, and predicate application positions exactly like the other precision ops.

## [0.60.0] — 2026-07-23

### Added — NUM-6c: the `to_percent(x [, places])` percentage rendering (ADJ-NUMERIC-SUBSTRATE §4.1, §4.3)

```
let share = to_percent(votes / total, 1)   % → "42.7%"
let round = to_percent(part / whole)        % default 2 decimal places
```

- `to_percent` joins `round_to`/`round_sig`/`to_scientific` as a built-in recognised
  during `Apply` lowering (same native comma-list surface, no new grammar). It lowers to
  the new `logic_engine::ComputeExpr::ToPercent` node via a new `ExprAst::ToPercent(expr,
  places)` AST node, under the default half-even mode.
- The `places` argument is **optional**: `to_percent(x)` uses the documented default
  (`DEFAULT_PERCENT_PLACES = 2`), resolved at lowering so the engine node always carries a
  concrete count and the audit records what was used. A stated `to_percent(x, n)` requires
  `n` a **non-negative** integer literal (`≥ 0` — zero places is meaningful, `"50%"`)
  within the precision cap (`MAX_ROUND_PLACES = 100`). A non-integer, negative, oversized,
  or non-literal `n`, or the wrong argument count, is a clean compile error.
- All four expression walkers carry the new node, so it composes inside formula bodies,
  `let`s, and predicate application positions exactly like the other precision ops.

## [0.59.0] — 2026-07-22

### Added — NUM-6c: the `to_scientific(x [, figures])` scientific-notation rendering (ADJ-NUMERIC-SUBSTRATE §4.1, §4.3)

```
let reported = to_scientific(avogadro, 4)   % → "6.022e23"
let quick    = to_scientific(measured)      % default 6 significant figures
```

- `to_scientific` joins `round_to`/`round_sig` as a built-in recognised during
  `Apply` lowering (same native comma-list application surface, no new grammar). It
  lowers to the new `logic_engine::ComputeExpr::ToScientific` node via a new
  `ExprAst::ToScientific(expr, figures)` AST node, under the default half-even mode.
- The `figures` argument is **optional**: `to_scientific(x)` uses the documented
  default mantissa precision (`DEFAULT_SCI_FIGURES = 6`), resolved here at lowering so
  the engine node always carries a concrete count and the audit records what was used.
  A stated `to_scientific(x, n)` requires `n` a positive integer literal (`≥ 1`, since
  a scientific mantissa has at least one significant figure) within the precision cap
  (`MAX_ROUND_PLACES = 100`). A non-integer, zero, negative, oversized, or non-literal
  `n`, or the wrong argument count, is a clean compile error — never a silent format.
- All the expression walkers (`collect_refs`, `charged_clone`, `expand_rec`,
  `substitute_expr`) carry the new node, so it composes inside formula bodies, `let`s,
  and predicate application positions exactly like `round_to`/`round_sig`.

## [0.58.0] — 2026-07-22

### Added — NUM-6b: the `round_sig(x, n)` significant-figures narrowing (ADJ-NUMERIC-SUBSTRATE §4.1–§4.4)

```
let reported = round_sig(measured / trials, 3)   % 3 significant figures
```

- `round_sig` joins `round_to` as a built-in recognised during `Apply` lowering
  (same native comma-list application surface, no new grammar). Both now build a
  `logic_engine::RoundSpec` — `Places` for `round_to`, `SigFigures` for `round_sig`
  — carried by the shared `ExprAst::RoundTo` node and lowered to the engine's exact
  `Round` eval.
- `n` must be an integer literal within the DoS cap; **≥ 1** for `round_sig` (zero
  significant figures is meaningless), ≥ 0 for `round_to`. Anything else is a clean
  `FormulaBadArgument` compile error.

## [0.57.0] — 2026-07-22

### Added — NUM-6a: the `round_to(x, n)` precision narrowing (ADJ-NUMERIC-SUBSTRATE §4.1–§4.4)

A formula body (or a `let`) can now round to a stated precision:

```
let dose_rounded = round_to(total / doses, 2)   % 10/3 → 3.33, exactly 333/100
```

- New `ExprAst::RoundTo(x, n)`, lowering to the engine's exact-path
  `ComputeExpr::Round` (default half-even mode). Dimension-preserving.
- Surface is the **native application** grammar — `round_to` is recognised as a
  built-in during `Apply` lowering, *before* the user-formula lookup, so it reuses
  the same comma-list call grammar as `quotient(a, b)` with **no new grammar and no
  LaTeX change**. (The kickoff spec's LaTeX `\operatorname{round}(x, n)` form does
  not parse a comma-separated argument list — the frontend splits on the top-level
  comma — so §4.1 was updated to the native surface; see the spec's surface note.)
- The precision `n` must be a **non-negative integer literal** ≤ 100 (a DoS cap);
  a non-integer, negative, oversized, or non-literal `n` is a clean compile error
  (`FormulaBadArgument`), never a silent mis-rounding.

## [0.56.0] — 2026-07-21

### Added — RS-4 PR-D4a: the `quote`/`at`/`snapshot` surface binding (ADJ-REASON-MATH §E.3.1)

A grounded clause can now write a **pinned verbatim span** — the bytes that make `adj-verify`
report `fully_verified`:

```
relate inhibits(aspirin, cyclooxygenase)
    quote "Aspirin inhibits cyclooxygenase" at 0 snapshot "<64-hex sha256>"
    source "Pharmacology reference"
    trust authoritative
```

- **Grammar**: new `quote_annotation = "quote" STRING "at" NUMBER "snapshot" STRING`, added to the
  `annotation` alternation (grammars regenerated).
- **AST**: `Annotation::Quote { text, byte_offset, snapshot_hex }`.
- **Lower**: the pin populates `Provenance::quote` (a `VerbatimSpan`) + `Provenance::snapshot`
  (a `ContentHash`) via `with_quote`. It is **fail-closed** on well-formedness — a snapshot that is
  not a 64-char SHA-256 hex, or a quote whose text has no visible content, is a compile error
  (`LowerError::MalformedQuotePin`), never a half-built `Verbatim` span the verifier would reject.
  A `quote` on a table row pins that row's own span. Duplicate `quote` is a `DuplicateAnnotation`.

The `byte_offset` is emitted by the grounding spider at ingest, never hand- or model-authored
(`feedback_no_byte_arithmetic_for_llm`). Reviewers read the `quote` text and `source` label; the
machine owns the arithmetic.

**Scoped as D4a** (per feedback_smaller_prs): this is the surface + fail-closed *well-formedness*
lowering. The *anchored* compile-time check (does the text really sit at `byte_offset` in the named
snapshot?) and the `adj-verify --snapshots` end-to-end path are D4b — the latter also fixes a latent
cli-builder flag-parse hang discovered while wiring this up. Until D4b, `adj-verify` still enforces
the anchored check at verify time, so a bad pin is caught; D4b moves that gate earlier, to compile.

## [0.55.0] — 2026-07-18

### Added — RS-5e: per-row provenance on a `table` (ADJ-TABLES)

**A table row can now carry the span that defends *it*.** Until now a `table` had ONE
`source`/`locator`/`trust` envelope, so every answer — in every band, from every row — quoted the
same sentence. That is an accounting error, not a cosmetic one: the audit trail asserted a fact and
cited a span that did not support it. The RS-5c range lookup made it glaring (the selected row is
explicit in the audit), but exact lookup had always been mis-cited the same way.

- **Grammar**: `table_row` gains an optional `[ LBRACE { annotation } RBRACE ]` block. The braces
  are deliberate — a table's envelope is written *after* its rows, so a bare trailing annotation
  would be ambiguous (last row's, or the table's?). `LBRACE` disambiguates. Regenerated
  `_parser_grammar.rs`.
- **AST**: `TableRow` gains `annotations: Vec<Annotation>`.
- **Lower**: new `row_provenance()` folds a row's block **over** the envelope *field by field*, so a
  row supplies only what differs — usually just its own `source` span — and inherits the shared
  `locator`/`trust`. Corroborating `cites` are appended. Duplicate keys inside one row block stay a
  clean `DuplicateAnnotation`.
- **No renderer change was needed**, which is the elegant part: each row already lowered to its
  **own** `Fact`, and every citation path (exact recall, range lookup, the proof DAG's `via_facts`)
  already cites *the fact that produced the answer*. Giving that fact the row's provenance was the
  entire fix.
- **Backward compatible**: a row with no block inherits the whole envelope, exactly as before, so
  every table authored pre-RS-5e is unchanged.

## [0.54.0] — 2026-07-18

### Added — RS-5c: range / bracket lookup over a `table` read as a step function (ADJ-TABLES)

A `table` can now be queried as a **step function**, not only by exact key. The new surface
`? lookup <table> <key_col> = <n> mode range give <value_col>` selects the breakpoint row whose
key column is the greatest key `<= n` and returns its value column — the tactic for tax brackets,
dose bands, and reference-range classification. This is the follow-up to RS-5b's exact lookup;
range/interpolated were spec'd there and deferred, and this lands the range half.

- **Grammar**: `query_decl` now folds a `lookup_expr` alternative
  (`QUESTION ( lookup_expr | term )`), so the range form coexists with the exact binding query.
  `lookup`/`mode`/`give` are IDENT-matched literals — **no new lexer tokens**. Regenerated
  `_parser_grammar.rs` / `_lexer_grammar.rs`.
- **AST**: new `Statement::RangeLookup { table, key_col, key_value, mode, value_col }`.
- **Adapter**: `adapt_lookup` (positional Name-token binding, robust to a column literally named
  `mode`/`give`) + `adapt_signed_number` (optional leading `MINUS`, folded exactly into the literal).
- **Lower**: validates against the table registry and resolves the key/value columns to positional
  indices — new `LookupUnknownTable` / `LookupUnknownColumn` / `LookupNonNumericKeyColumn`
  (the key column must be numeric) / `LookupModeUnsupported` (`interpolated` is reserved for RS-5d)
  / `LookupUnknownMode`. Emits the validated `LoweredRangeLookup` (now on `LoweredProgram`).
- The `table` declaration is **unchanged** — a range table is an ordinary table read differently.

## [0.53.0] — 2026-07-14

### Added — NX-2: parse numeric literals to exact values (no silent f64 truncation)

A written decimal literal is now stored **exactly as written** instead of being narrowed to `f64`
at parse time (spec: `code/specs/ADJ-EXACT-NUMBERS.md`). Before this change a table cell or valued
fact like π to 39 places bound at only ~16 significant digits; now every digit survives parse →
store → query.

- New AST node **`NumLit { Int(i64), Exact(BigDecimal) }`** with `to_f64_lossy()`. `Term::Num` and
  `TableCell::Number` now carry a `NumLit` instead of an `f64`.
- The adapter's two **ground-term** sites (a table cell and a valued-fact argument) parse the
  NUMBER token via new `parse_numlit`: a whole number that fits `i64` becomes `Int` (keeping the
  engine's small-integer fast paths), everything else is parsed with `BigDecimal::from_str` into
  `Exact`. The two **compute-leaf** sites (`ExprAst::Lit`, inherently `f64`) keep their
  `parse_finite` → `f64` path — the labeled-lossy boundary.
- `lower_*` emit `Number::Int` / `Number::Exact` directly, never through `f64`.
- Behavior note: a magnitude that overflows `f64` (`1e400`) is no longer rejected at parse — it is
  a valid exact decimal now stored with full precision. A scale-amplification payload
  (`1e-2000000000`) is still rejected by `BigDecimal`'s `MAX_SCALE` budget.
- **DoS guard.** Because `.adj` source is untrusted and `BigDecimal` base-10 conversion is
  `O(digits²)`, `parse_numlit` caps an exact literal's **byte length** (`MAX_NUMBER_TOKEN_LEN =
  4096`, checked before the quadratic parse) and its **scale magnitude** (`MAX_NUMBER_TOKEN_SCALE =
  4096`, so a tiny token like `1e-1000000` cannot force a ~1 MB render/`to_f64` string). Both sit
  ~100× above any legitimate constant (π to 39 places is 41 bytes), so only hostile payloads are
  rejected — restoring the implicit bound the old `f64` parse provided. Adversarial tests cover
  both shapes.
- Depends on `bignum-core` ≥ 0.5.0 and `logic-core` ≥ 0.2.1.

## [0.52.0] — 2026-07-13

### Fixed — recursion-depth guard against native stack overflow (DoS)

`parse` built its `GrammarParser` with no recursion-depth cap, even though
`adj-lang` is reachable via `adj-lang-cli` on arbitrary `.adj` files — a
real, not theoretical, attack surface. Deeply-nested input, in any of this
grammar's three *independent* recursive shapes (parenthesised arithmetic
nesting via `factor → expr → term_expr → factor`, direct call nesting via
`term`'s own self-recursion, or `rulebook { rulebook { … } }` nesting via
`statement`'s own alternation including `rulebook_decl`), would recurse
until it overflowed the native thread stack — an uncatchable process abort
— before this crate's own `Result`-returning entry points ever got a
chance to report anything. (The rulebook shape was missed in the first
pass of this fix and caught by `/security-review` before merge.)

All three shapes were independently measured (binary search, uncapped
parser, the true default per-test-thread stack — no `RUST_MIN_STACK`
override, no explicit `Builder::stack_size`, matching what `cargo test`
and a production caller both actually get — debug build, adversarial
5000-level input): paren nesting safe through 260 rule-frames, crashes at
262; rulebook nesting safe through 245, crashes at 250; call nesting (the
*binding*, lower floor) safe through 124, crashes at 126. Added a bespoke
`MAX_RULE_DEPTH = 90` — about 27% below the binding 124-rule-frame floor —
and wired it into `parse` via `.with_max_depth(...)`.

- Added `MAX_RULE_DEPTH: usize = 90` and wired it into `parse`.
- 9 new regression tests (3 per independent recursive shape): deep
  adversarial input on an enlarged-stack thread returns a clean `Err`,
  input at the measured real-nesting boundary (28 levels for paren
  nesting, 44 for rulebook nesting, 86 for call nesting) still parses
  while one level past it doesn't, and the cap trips before the native
  stack would overflow even on a default-stack thread.

No change to behaviour for any input that nests below the cap.

## [0.51.0] — native tabular data: the `table` construct (ADJ-TABLES RS-5)

### Added

- **`table` construct** — a first-class, importable, provenanced tabular relation, a sibling of
  `dictionary`/`rulebook`/`formulabook`. Surface:
  `table <name> { [use <dict>…] columns c1,… row (v1,…) … source "…" locator "…" trust <tier> }`.
  `table`/`columns`/`row` are IDENT-matched literals (no new lexer keywords). Grammar: new
  `table_decl`/`columns_decl`/`table_row`/`row_item` rules; AST: `Statement::Table` +
  `TableRow`/`TableCell`; adapter: `adapt_table`/`adapt_row_item`.
- **Rows lower to relations** — each `row (v1,…,vn)` lowers to a ground `logic_engine::Fact`
  `name(v1,…,vn)` carrying the table's provenance, byte-identical to how a `relate` edge lowers. So
  **exact lookup is the existing SLD binding query** (`? name(key, $V)`) with zero new engine code,
  the answer names the table's citation as its proof, a missing key abstains, and a looked-up number
  feeds a `let`/`formula` through the existing slot/`Ref` path. Cells map 1:1 onto the engine's three
  ground term kinds (`Num`/`Atom`/`Str`).
- **Guards** — `LowerError::TableArity` (a row whose cell count ≠ the declared `columns`),
  `TableMissingProvenance` (a shipped table must be `source`d — the write gate shared with
  `formula`/`relate`), and `TableNoColumns` (defensive).
- Motivation and design in `code/specs/ADJ-TABLES.md`; the first shipped table,
  `reference/length-conversions.adj` (NIST exact length→metre factors), demonstrates ingesting a
  published table verbatim and citing it once. Range/bracket and interpolated lookup are staged
  follow-ups (RS-5c/RS-5d).

## [0.50.0] — multi-step formula bodies (ADJ-RULE-SUBSTRATE RS-2)

### Added

- **Multi-step `formula` bodies** — a formula may now be written as a block of named `let`-steps
  followed by a final expression: `formula f(p…) { let s1 = e1  let s2 = e2  <body> }`. Each step
  names an intermediate value; a later step and the final body may reference the parameters plus any
  **earlier** step. Grammar: new `formula_body` (either the existing `= <expr>` sugar or the block form)
  and `formula_step` rules; AST: `FormulaStep` + `FormulaDef.steps`. The lowerer folds the steps into a
  single **effective body** by in-order substitution, so the RS-1 param-substitution and
  formula-calls-formula expansion consume it unchanged (a multi-step body is surface sugar for the
  equivalent nested single expression). Scope is validated strictly left-to-right — an undeclared or
  forward step reference is a clean `LowerError::FormulaFreeVariable`. The size budget bounds an
  adversarial step chain to `FormulaExpansionTooLarge`. The single-expression form is unchanged (purely
  additive). Worked example: the shipped `clinical/cockcroft_gault.adj` (four named steps composing
  `difference`/`product`/`quotient`). *(Per-step `DerivedRef` trace nodes — each step as its own audit
  entry — are deferred to the RS-4 execution-trace renderer; RS-2 delivers the expressible, correct,
  composing construct.)*

## [Unreleased] — `formulabook` / `formula` — importable, provenanced, parameterized formulas (ADJ-FORMULA-LIBRARIES rung-0)

### Added

- **`formulabook <name> { use <dict>… formula… }`** and **`formula <name>(<params>) = <expr>`** — the
  rung-0 substrate of the *compute* standard library (a sibling of `rulebook`). A `formula` is a named,
  importable, reusable `let`: `<expr>` reuses the existing `let` expression grammar verbatim, and a leaf
  naming a declared `<param>` is a **formal parameter**, bound at apply time. Each formula carries the
  same `source "…" locator "…" trust <tier>` provenance envelope every grounded clause carries.
- **Formula application** — a consumer's `? name(args)` whose functor names a registered formula (with
  matching arity) is APPLIED, not treated as a hypothesis query: each parameter binds to its argument
  (a like-named `observe`d slot or a number literal), the body is substituted, and the result is
  evaluated through the **existing** `logic_engine::compute` (`ComputeExpr`) path and bound as a derived
  value named after the formula — carrying the formula's cited provenance for the audit trail.
- **Two validations**: parameter-scoping (`LowerError::FormulaFreeVariable` — a body identifier that is
  not a declared parameter) and a provenance-required lint (`LowerError::FormulaMissingProvenance` — a
  shipped formula must carry a non-empty `source`). Vocabulary enforcement now accepts a formula-application
  query so the closed-vocabulary gate does not reject the new construct.
- AST: `Statement::Formulabook { name, uses, formulas }` + `FormulaDef { name, params, body, annotations }`.

## [0.49.0] - 2026-07-03 — `constrain asciimath "…"` — a second frontend reaches the constraint surface

### Added

- **`constrain asciimath "<relation>"`** is now accepted anywhere `constrain latex "…"` is. Until now
  only the LaTeX surface could state a *constraint* (`constrain latex "x^2 = 4"`); a model that writes
  its equations in AsciiMath had to fall back to the bare `constrain <expr> <relop> <expr>` form. Now
  the AsciiMath relation surface reaches the constraint sublanguage too: `constrain asciimath "x^2 = 4"`
  and inequalities like `constrain asciimath "a <= b"` lower to a `Statement::Constrain` and feed the
  solver (`solve for { … }` / `check`) exactly as the LaTeX form does.
- **One code path, one frontend swapped.** The new `adapt_constrain_asciimath` is a verbatim mirror of
  `adapt_constrain_latex` — it parses the string with the SAME AsciiMath `MathFrontend` already used for
  `asciimath "…"` expression factors (`parse_asciimath_math`), yielding the neutral
  `MathExpr::Rel(op, lhs, rhs)`, then lowers the operator through the SAME `lower_latex_relop` and both
  sides through the SAME `latex_math_to_expr_ast`. So `x^2` becomes the identical single
  `ComputeOp::Pow` node the LaTeX surface produces — no new relation semantics, **no new tree-walker**,
  and therefore no new stack-overflow/DoS surface (the AsciiMath crate owns its own `MAX_DEPTH`
  discipline and `#![forbid(unsafe_code)]`).
- New grammar productions `constrain_asciimath_decl = "constrain" asciimath_relation ;` and
  `asciimath_relation = "asciimath" STRING ;` (regenerated `_parser_grammar.rs`; no new tokens, so
  `_lexer_grammar.rs` is unchanged). Unsupported/non-relation AsciiMath still surfaces via the shared
  `AdapterError::UnsupportedLatexMath`.
- Two new e2e tests: `native_asciimath_relation_lowers_to_constraint` (equality → `ComputeOp::Pow`
  quadratic) and `native_asciimath_inequality_relation_lowers` (`a <= b` → `RelOp::Le`).

## [0.48.0] - 2026-07-03 — a FOURTH math frontend: `unicodemath "…"` — PFE01 quartet complete

### Added

- **`unicodemath "<math>"`** is now accepted anywhere an ADJ arithmetic expression is (as an `expr`
  factor), alongside `latex "…"`, `asciimath "…"`, and `mathml "…"`. Unicode plain-math is what
  people and models actually *type* with real glyphs — e.g. `(3+4) ÷ 2`, `3 × 4`, `x² + y²`, `√x`,
  `π·α` — and the repo already ships a `unicode-math` `MathFrontend` that parses it to the **same
  neutral `MathExpr`** the other three frontends produce.
- This **completes the pluggable-frontends quartet (PFE01)**: four genuinely different notations (a
  macro language, a terse ASCII syntax, an XML tree, and raw Unicode) all compute through the
  **identical, unchanged** `latex_math_to_expr_ast` lowering. The adapter's only frontend-specific
  step is the parse call (`parse_unicodemath_math`). So the whole arithmetic + named-function surface
  is available to Unicode-math **for free** — no new lowering, and **no new engine op**. (`÷` means
  the same as LaTeX `\div` / AsciiMath+MathML division.)
- New grammar production `unicodemath_expr = "unicodemath" STRING ;` (regenerated `_lexer_grammar.rs`
  / `_parser_grammar.rs`); new `AdapterError::UnicodeMathParse` for parse failures (unsupported
  *nodes* still surface via the shared `UnsupportedLatexMath`).
- Three end-to-end tests through `compile_and_decide`: `unicodemath "(3+4) ÷ 2"` = 3.5,
  `unicodemath "3 × 4"` = 12, and an observed slot binding inside Unicode-math
  (`observe x(2)` + `unicodemath "x × x"` = 4).
- **Security/DoS:** the adapter adds **no new recursive tree-walker** — it reuses the existing
  `latex_math_to_expr_ast` lowering. The Unicode-math parser owns its own recursion guard
  (`MAX_DEPTH = 64`) and `#![forbid(unsafe_code)]` in its crate, so no new stack-overflow surface is
  introduced here (no new deep-input regression test is warranted on the adapter side).

## [0.47.0] - 2026-07-03 — a THIRD math frontend: `mathml "…"` (pluggable frontends, PFE01)

### Added

- **`mathml "<math>"`** is now accepted anywhere an ADJ arithmetic expression is (as an `expr`
  factor), alongside `latex "…"` and `asciimath "…"`. Presentation MathML is what many rendering
  pipelines and some models emit — e.g. `<mfrac><mn>1</mn><mn>2</mn></mfrac>`,
  `<mn>3</mn><mo>+</mo><mn>4</mn>`, `<mi>x</mi><mo>*</mo><mi>x</mi>` — and the repo already ships a
  `mathml` `MathFrontend` that parses it to the **same neutral `MathExpr`** the LaTeX and AsciiMath
  frontends produce.
- Third demonstration of the **pluggable-frontends thesis (PFE01)**: the adapter's only
  frontend-specific step is the parse call (`parse_mathml_math`). The resulting `MathExpr` flows
  through the **identical, unchanged** `latex_math_to_expr_ast` lowering, so the whole arithmetic +
  named-function surface is available to MathML **for free** — no new lowering, and **no new engine
  op**. (`<mfrac>` means the same as LaTeX `\frac{1}{2}` and AsciiMath `1/2`: all `MathExpr::Frac`.)
- New grammar production `mathml_expr = "mathml" STRING ;` (regenerated `_parser_grammar.rs`); new
  `AdapterError::MathMlParse` for parse failures (unsupported *nodes* still surface via the shared
  `UnsupportedLatexMath`, since the neutral-tree lowering names its errors after the first frontend).
- Three end-to-end tests through `compile_and_decide`: `mathml "<mfrac><mn>7</mn><mn>2</mn></mfrac>"`
  = 3.5 (reusing the very `MathExpr::Frac` lowering LaTeX `\frac` uses),
  `mathml "<mn>3</mn><mo>+</mo><mn>4</mn>"` = 7, and an observed slot binding inside MathML
  (`observe x(2)` + `mathml "<mi>x</mi><mo>*</mo><mi>x</mi>"` = 4).
- **Security/DoS:** the adapter adds **no new recursive tree-walker** — it reuses the existing
  `latex_math_to_expr_ast` lowering. The MathML parser owns its own recursion guard (`MAX_DEPTH = 64`,
  iterative teardown) and `#![forbid(unsafe_code)]` in its crate, so no new stack-overflow surface is
  introduced here (no new deep-input regression test is warranted on the adapter side).

## [0.46.0] - 2026-07-03 — a SECOND math frontend: `asciimath "…"` (pluggable frontends, PFE01)

### Added

- **`asciimath "<math>"`** is now accepted anywhere an ADJ arithmetic expression is (as an `expr`
  factor), alongside the existing `latex "…"`. AsciiMath is a math dialect many math-tuned models
  emit — e.g. `(a+b)/c`, `1/2`, `x*x` — and the repo already ships an `asciimath` `MathFrontend`
  that parses it to the **same neutral `MathExpr`** tree the LaTeX frontend produces.
- This is the **pluggable-frontends thesis (PFE01)** made concrete: the adapter's only
  frontend-specific step is the parse call (`parse_asciimath_math`, the counterpart to
  `parse_latex_math`). The resulting `MathExpr` flows through the **identical**
  `latex_math_to_expr_ast` lowering, so the entire arithmetic + named-function surface
  (fractions, products, sums, powers, the trig/hyperbolic families, …) is available to the
  AsciiMath surface **for free** — no new lowering, and **no new engine op**.
- New grammar production `asciimath_expr = "asciimath" STRING ;` (regenerated `_lexer_grammar.rs`
  / `_parser_grammar.rs`); new `AdapterError::AsciiMathParse` for parse failures (unsupported
  *nodes* still surface via the shared `UnsupportedLatexMath`, since the neutral-tree lowering
  names its errors after the first frontend).
- Three end-to-end tests through `compile_and_decide` prove it computes: `asciimath "(3+4)*2"` = 14,
  `asciimath "(3+4)/2"` = 3.5 (reusing the very `MathExpr::Frac` lowering LaTeX `\frac` uses), and an
  observed slot binding inside AsciiMath (`observe x(2)` + `asciimath "x*x"` = 4).
- **Security/DoS:** the adapter adds **no new recursive tree-walker** — it reuses the existing
  `latex_math_to_expr_ast` lowering. The AsciiMath parser owns its own recursion/DoS discipline in
  its crate, so no new stack-overflow surface is introduced here (no new deep-input regression test
  is warranted on the adapter side).

## [0.45.0] - 2026-07-03 — inverse hyperbolic functions (`arsinh`/`arcosh`/`artanh`) in `latex "…"`

### Added

- `latex "\operatorname{arsinh}(x)"`, `\operatorname{arcosh}(x)`, `\operatorname{artanh}(x)` — the
  inverse (area) hyperbolic functions — now lower. The mirror of the reciprocal-hyperbolic arm: none
  is a frontend `Func`, so each arrives as the operator-name juxtaposition
  `Bin(Mul, Text("arsinh"), (x))` (or `Bin(Mul, Symbol("arsinh"), (x))` for the bare `\arsinh` macro).
  Each is composed from its closed-form logarithm identity using only primitives the engine already
  has — the natural log (`NamedFn::Ln`) and the power op (`ArithOp::Pow`, for both squaring `^2` and
  the square root `^0.5`) — so **no engine op is added**:
  `arsinh(x) = ln(x + (x²+1)^0.5)`, `arcosh(x) = ln(x + (x²−1)^0.5)`,
  `artanh(x) = 0.5·ln((1+x)/(1−x))`. Results are the standard real branch (and NaN outside each
  function's real domain, matching the underlying `ln`/root). This finishes the hyperbolic family:
  direct (`sinh`/`cosh`/`tanh`), reciprocal (`coth`/`sech`/`csch`), and now inverse
  (`arsinh`/`arcosh`/`artanh`) all lower.
- Common surface spellings are all accepted: the area form (`arsinh`), the inverse-notation form
  (`arcsinh`), and the terse form (`asinh`) — likewise `arcosh`/`arccosh`/`acosh` and
  `artanh`/`arctanh`/`atanh` — across both the bare-macro and `\operatorname{…}`/`\mathrm{…}` spellings.
- Adapter-only, no engine/AST change. The argument recurses through the SAME
  `latex_math_to_expr_ast` the `\sin`/`\coth` arms use (no new tree-walk, no new DoS surface); the
  identity clones the already-lowered, already-bounded argument where it is named more than once.

## [0.44.0] - 2026-07-02 — reciprocal hyperbolic functions (`\coth`/`\sech`/`\csch`) in `latex "…"`

### Added

- `latex "\coth(x)"`, `\sech(x)`, `\csch(x)` — the reciprocal hyperbolic functions — now lower.
  The frontend has `Func` variants for `\sinh`/`\cosh`/`\tanh` but not their reciprocals, so `\coth`
  is an unknown control sequence that arrives as the operator-name juxtaposition
  `Bin(Mul, Symbol("coth"), (x))` (a bare `Symbol` named `coth` can only come from that macro —
  plain `coth` in math mode is the product `c·o·t·h`). Each reciprocal is composed exactly from the
  hyperbolic `NamedFn` it inverts — coth = 1/tanh, sech = 1/cosh, csch = 1/sinh — so no engine op is
  added. This closes the trig/hyperbolic symmetry: the circular reciprocals `cot`/`sec`/`csc`
  already lowered; their hyperbolic twins now do too.
- Both spellings are recognised: the bare macro (`\coth(x)`) and the operator-name form
  (`\operatorname{coth}(x)` / `\mathrm{coth}(x)`, which the frontend renders as `Text("coth")`).
- Adapter-only, no engine/AST change. The argument recurses through the SAME
  `latex_math_to_expr_ast` the `\sin`/`\exp` arms use (no new tree-walk, no new DoS surface); the
  left-factor match is a shallow `Symbol`/`Text` name check.

## [0.43.0] - 2026-07-02 — binomial coefficients (`\binom{n}{k}`) in `latex "…"`

### Added

- `latex "\binom{n}{k}"` (and the `\dbinom` / `\tbinom` display/text-style spellings, which the
  frontend normalises to the same `MathExpr::Binom`) now lowers to the concrete value of the
  binomial coefficient "n choose k" — `C(n, k) = n! / (k!·(n−k)!)`. This is DISTINCT from
  `\frac{n}{k}`: it is the combinatorial COUNT, not the ratio `n/k`. Example: `\binom{5}{2}` = 10,
  `\binom{9}{7}` = `\binom{9}{2}` = 36, `\binom{4}{0}` = 1, and it composes with surrounding
  arithmetic (`\binom{4}{2} + \binom{3}{1}` = 9).
- Only the decidable case is evaluated — both arguments CONCRETE non-negative integers with
  `k ≤ n ≤ 1000`. The value is computed with the **multiplicative product formula**
  `∏_{i=1}^{min(k,n−k)} (n−k+i)/i`, which is exact (integer-valued at every step) and needs at most
  `n/2` multiply/divide operations. A symbolic argument (`\binom{n}{k}` with variables), a negative
  argument, `k > n`, an `n` beyond the cap, or a coefficient too large to represent exactly as an
  f64 integer (e.g. `\binom{60}{30}`) is an explicit `UnsupportedLatexMath` — never a guess, an
  approximation, or a silently-rounded literal.
- Adapter-only, no engine/AST change. Both arguments are read with the existing NON-recursive
  `number_as_i64`, so a pathological argument like `\binom{aaaa…}{2}` (a deep `Bin(Mul)`
  juxtaposition spine in the `n` slot) is rejected on the outermost node WITHOUT walking the spine
  — no new unbounded tree-walk, no stack-overflow DoS surface (regression-tested with a
  20,000-letter braced argument).

## [0.42.0] - 2026-07-02 — symbolic / computed power exponents (`x^y`, `x^{a+b}`) in `latex "…"`

### Changed

- The `latex "x^n"` power exponent may now be **symbolic or computed**, not just a non-negative integer
  literal: `x^y` (with `y` observed) computes `x` raised to `y`, and `x^{a+b}` raises to a computed
  exponent. Both the base and the exponent lower as general expressions to a single native
  `ComputeOp::Pow`; the engine evaluates the exponent at run time and enforces its own rules — the
  exponent must be dimensionless and finite, and a non-integer exponent on a dimensioned base is
  rejected (no fractional dimension) — so a symbolic exponent computes for the dimensionless case and
  is cleanly rejected otherwise. Numeric exponents (`x^{2}`, `x^{10}`) are unchanged. (Root DEGREES
  `\sqrt[n]{x}` still require a concrete positive integer, since a symbolic degree cannot form the
  reciprocal `1/n`.) Removed the now-unused `latex_power_exponent` literal-only validator.

## [0.41.0] - 2026-07-02 — finite `\sum` / `\prod` unroll in `latex "…"`

### Added

- `latex "\sum_{i=1}^{3} …"` and `\prod_{k=1}^{4} …` with **concrete finite integer bounds** now compute
  by **unrolling**: for each `k` in `lo..=hi` the loop variable is substituted into the body and the
  terms are folded with `+` (sum) or `·` (product). `\sum_{i=1}^{3} i` = 1+2+3 = 6; `\prod_{k=1}^{4} k`
  = 24. Composes with subscripts — `\sum_{i=1}^{3} x_i` expands to `x_1 + x_2 + x_3`, each `x_k` binding
  to its own `observe`. A **symbolic** bound (`\sum_{i=1}^{n}`), an **integral** (`\int`), an inverted
  range, or a range beyond the 256-term unroll cap is an explicit `UnsupportedLatexMath` — never
  approximated. The index-substitution walker is **depth-budgeted** (rejects rather than recursing
  without limit), so a deeply-nested braced body cannot overflow the stack. Pure adapter recognition —
  no engine, AST, or lowering change.

## [0.40.0] - 2026-07-02 — over/under annotations (`\overset`, `\underset`, `\overbrace`, `\underbrace`) compute transparently in `latex "…"`

### Added

- `latex "\overset{note}{base}"`, `\underset{note}{base}`, `\overbrace{base}`, `\underbrace{base}` and
  any other over/under annotation now **compute, lowering transparently to the `base`** and dropping
  the annotation mark. Like an accent, an over/under mark is a notational decoration, not an
  operation: `\overbrace{a + b}^{\text{sum}}` labels the sum in prose but computes `a + b`. Previously
  these were an `UnsupportedLatexMath` error. (`\overline{x}`/`\underline{x}` already computed — they
  parse as `Accent`, handled by the accent-transparent arm.) Pure adapter recognition — no engine,
  AST, or lowering change; the arm recurses into the single `base` sub-node exactly like the accent
  arm, so no new deep-walk vector. Note: `\sum`/`\prod`/`\int` (`MathExpr::BigOp`) remain unsupported —
  a finite summation/product needs index-binding and bounded unrolling, a larger feature deferred to a
  dedicated slice.

## [0.39.0] - 2026-07-02 — subscripted variables (`x_i`, `x_1`, `V_{max}`) bind as distinct names in `latex "…"`

### Added

- `latex "x_i"`, `x_1`, `V_{max}` and any other **subscripted variable** now lower to a single flat
  identifier `base_sub` (`x_i` → `x_i`, `x_1` → `x_1`, `V_{max}` → `V_max`) that binds to a matching
  `observe`. A subscript does not compute — `x_1` and `x_2` are two DISTINCT observed quantities, and
  `V_{max}` / `C_{peak}` are named readings, not `V` times something — so mangling the subscript into a
  name (rather than treating `x_i` as `x·i`) is what a model means. A single-letter/number subscript is
  a `Symbol`/`Number`; a BRACED multi-letter subscript `{max}` arrives from the frontend as a
  juxtaposition chain of single-letter `Symbol`s, which the new `subscript_ident_part` helper flattens
  back into the word. An arithmetic subscript (`x_{i+1}`) the helper cannot name is an explicit
  `UnsupportedLatexMath` — never a silent mis-binding. (The mangled name must be a legal `observe`
  identifier: the ADJ surface lexer requires a lowercase start, so a `V_{max}` reading binds when
  declared `observe v_max(…)`; the adapter mangling itself is case-preserving.) Pure adapter
  recognition — no engine, AST, or lowering change.

## [0.38.0] - 2026-07-02 — accent-wrapped operands (`\hat{x}`, `\bar{x}`, …) compute transparently in `latex "…"`

### Added

- `latex "\hat{x}"`, `\bar{x}`, `\vec{x}`, `\tilde{x}` and any other **accent** over an operand now
  compute, lowering **transparently to the accented operand**. In arithmetic an accent is a
  notational decoration, not an operation: a model that writes a statistics formula like
  `\hat{p}(1 - \hat{p})` (estimated-variance numerator) or `\bar{x} - \bar{y}` (difference of means)
  means the accented symbol to carry its operand's value — the hat/bar just marks it as an
  estimate/mean in prose. The adapter recognises `MathExpr::Accent { body, .. }` and lowers to
  `latex_math_to_expr_ast(body, …)`, so `\hat{a}(b - \hat{a})` computes as `a·(b − a)` with dimension
  and value flowing through the decoration unchanged (previously an `UnsupportedLatexMath` error).
  This is the first **non-function** LaTeX construct consumed after the named-operator surface
  saturated. Pure **adapter** recognition: no engine, AST, or lowering change. +1 e2e unit test
  (`\hat{a}(b − \hat{a})` = 21; `\bar{x} + \bar{y}` = 10).

## [0.37.0] - 2026-07-02 — `\operatorname{sin/cos/…}` trig-family word spellings in `latex "…"`

### Added

- `latex "\operatorname{sin}(x)"` and the whole **trigonometric family** — direct
  (`sin`/`cos`/`tan`/`cot`/`sec`/`csc`), inverse (`asin`/`acos`/`atan` and their `arc…`
  aliases), and hyperbolic (`sinh`/`cosh`/`tanh`) — now compute, reaching the SAME native
  `ExprAst::Call(NamedFn::…)` as their backslash-macro spellings (`\sin(x)`, `\arctan(x)`). A
  model that writes the operator *name* instead of the macro lands on the identical node.
  `\operatorname{…}` is a TEXT command, so — exactly like `\operatorname{exp}`/`floor`/`sgn` —
  these parse as the operator-name juxtaposition `Bin(Mul, Text("sin"), (x))` rather than a
  `Call`; one consolidated adapter arm recognises the family via the new `operator_name_trig_fn`
  helper (which maps the trimmed text name — accepting `arcsin`≡`asin` etc. — to its `NamedFn`)
  and lowers to `ExprAst::Call` (transcendental, Scalar→Scalar). Pure **adapter** recognition:
  no engine, AST, or lowering change. +1 e2e unit test (cos(0)=1; sinh(0)=0; arctan(0)=0 via
  the alias path).

## [0.36.0] - 2026-07-02 — `\operatorname{abs/exp/log/ln}` word spellings in `latex "…"`

### Added

- `latex "\operatorname{abs}(x)"`, `\operatorname{exp}(x)`, `\operatorname{log}(x)`, and
  `\operatorname{ln}(x)` now compute the **single-argument unary functions**, reaching the SAME
  native nodes as their existing spellings: `abs`→`ExprAst::Abs` (the `|x|` absolute value,
  dimension-preserving), and `exp`/`log`/`ln`→`ExprAst::Call(NamedFn::Exp/Log/Ln)` (the
  transcendentals already lowered from the `\exp`/`\log`/`\ln` macro `Call`s). A model that writes
  the operator *name* instead of the bracket/macro lands on the identical node. `\operatorname{…}`
  is a TEXT command, so — exactly like `\operatorname{floor}`/`sgn`/`trunc` — these parse as the
  operator-name juxtaposition `Bin(Mul, Text("exp"), (x))` rather than a `Call` or a `Fenced`; the
  adapter matches via `operator_name_is(lhs, …)` and lowers to the existing node. Pure **adapter**
  recognition: no engine, AST, or lowering change. +1 e2e unit test (abs(a−b)=7; exp(ln(x))
  round-trip; base-10 log(1000)=3).

## [0.35.0] - 2026-07-02 — `\operatorname{min/max/gcd/lcm}` word spellings in `latex "…"`

### Added

- `latex "\operatorname{min}(a, b)"`, `\operatorname{max}(…)`, `\operatorname{gcd}(…)`, and
  `\operatorname{lcm}(…)` now compute the **variadic binary functions**, reaching the SAME
  native ops (`ComputeOp::Min2`/`Max2`/`Gcd`/`Lcm`) as their already-supported function-call
  spellings (`\min(a, b)`, `\gcd(a, b, c)`): a model that writes the operator *name* instead
  of the backslash macro lands on the identical node. `\operatorname{…}` is a TEXT command,
  so — unlike `\min(…)`, which parses as a `Call` — `\operatorname{gcd}(a, b)` parses as the
  operator-name juxtaposition `Bin(Mul, Text("gcd"), (a, b))` (the same shape the adapter
  already recognises for `\operatorname{floor}`/`sgn`/`trunc`); the adapter matches via
  `operator_name_is(lhs, …)` and folds the comma-separated argument `Sequence` through the
  SAME `Call2` chain as the call spelling (`gcd(a, b, c)` → `gcd(gcd(a, b), c)`, associative).
  Two-or-more operands accepted; a single argument is a clean, explicit error. Pure **adapter**
  recognition: no engine, AST, or lowering change. The shared fold is factored into
  `latex_nary_fold`, now used by both the `\min(…)` `Call` path and the `\operatorname{min}(…)`
  juxtaposition path. +2 e2e unit tests (min/max/gcd/lcm word spellings; one-arg rejection).

## [0.34.0] - 2026-07-02 — `\operatorname{floor/ceil/round}` word spellings in `latex "…"`

### Added

- `latex "\operatorname{floor}(x)"`, `\operatorname{ceil}(x)`, and `\operatorname{round}(x)`
  now compute the **word-spelled roundings**, reaching the SAME `ComputeOp`s as their
  already-supported Unicode-bracket twins (`⌊x⌋`→`Floor`, `⌈x⌉`→`Ceil`, `⌊x⌉`→`Round`): a
  model that writes the operator *name* instead of the bracket lands on the identical node.
  `\operatorname{…}` is a TEXT command, so each parses as the operator-name juxtaposition
  `Bin(Mul, Text("floor"), (x))` — the same shape the adapter already recognises for
  `\operatorname{trunc}`/`\operatorname{sgn}` — and lowers via `operator_name_is(lhs, …)` to
  the existing `ExprAst::Floor`/`Ceil`/`Round` (dimension-preserving). Pure **adapter**
  recognition: no engine, AST, or lowering change. +1 e2e unit test (floor/ceil/round).

## [0.33.0] - 2026-07-02 — `\operatorname{sgn}` sign function in `latex "…"`

### Added

- `latex "\operatorname{sgn}(x)"` now computes the **sign** `sgn(x)` (`−1`/`0`/`+1`).
  New `ExprAst::Sign` lowers to the new `logic_engine::ComputeOp::Sign`. `\operatorname`
  is a TEXT command, so `\operatorname{sgn}(x)` parses as the operator-name
  juxtaposition `Bin(Mul, Text("sgn"), (x))` — the same shape the adapter already
  recognises for `\operatorname{trunc}` — and lowers via `operator_name_is(lhs, "sgn")`.
  Unlike the rounding ops, `sgn` collapses to a dimensionless `Scalar` while accepting a
  dimensioned operand, so `\operatorname{sgn}(a - b)` (the sign of a net quantity)
  computes to a clean ±1. +adapter and lower unit tests.

## [0.32.0] - 2026-07-02 — `\bmod` / `\pmod` modulo in `latex "…"`

### Added

- `latex "a \bmod b"` and `latex "a \pmod{b}"` now compute the **modulo** `a mod b`
  (the remainder with the sign of the dividend). New `ArithOp::Mod` lowers to the new
  `logic_engine::ComputeOp::Mod`. `\bmod`/`\pmod` are not in the frontend's operator
  tables, so they parse as a bare `Symbol("bmod")`/`Symbol("pmod")` inside an implicit
  multiplication — `a \bmod b` → `Bin(Mul, Bin(Mul, a, bmod), b)`. The adapter
  recognises that operator-name-juxtaposition shape (the same technique used for
  `\operatorname{trunc}(x)`) via a `mod_juxtaposition_lhs` helper and lowers it to
  `a mod b`. The arm sits above the general `Bin(Mul, …)` so genuine products still
  multiply; the congruence form `x \equiv y \pmod{n}` parses as a rejected `Rel(Equiv)`,
  so only the direct remainder computes — never a mis-lowered congruence. +adapter and
  lower unit tests.

## [0.31.0] - 2026-07-02 — n-ary `\min`/`\max`/`\gcd`/`\lcm` in `latex "…"`

### Changed

- `\min`/`\max`/`\gcd`/`\lcm` now accept **two OR MORE** comma-separated arguments
  (previously exactly two). Because these ops are associative, an n-ary call
  **left-folds** into a chain of the existing binary `ExprAst::Call2` —
  `\min(a, b, c)` becomes `min(min(a, b), c)` — so it reuses the native
  `ComputeOp::Min2`/`Max2`/`Gcd`/`Lcm` with **no engine change** and no n-ary op.
  A two-arg call is unchanged (a single `Call2`). `latex_two_args` (exactly-two)
  became `latex_nary_args` (≥ 2). A single-arg `\min(a)` is still a clean
  `UnsupportedLatexMath` error.

### Tests

- n-ary lower test: `\min(a,b,c,d)=2`, `\max(a,b,c,d)=9`, `\gcd(24,36,60)=12`,
  `\lcm(2,3,4)=12`. The wrong-arity test now asserts only the one-arg rejection
  (three-plus args are valid).

## [0.30.0] - 2026-07-02 — `\operatorname{trunc}(x)` truncation in `latex "…"`

### Added

- The `latex "…"` adapter now lowers **`\operatorname{trunc}(x)`** to the native
  `ComputeOp::Trunc` (truncate toward zero) via a new `ExprAst::Trunc`. Because
  `\operatorname{…}` is a TEXT command, the frontend parses
  `\operatorname{trunc}(x)` NOT as a function call but as an implicit
  multiplication (juxtaposition) — `Bin(Mul, Text("trunc"), (x))`. The adapter
  recognises that exact operator-name shape (a new `operator_name_is` helper)
  above the general `Bin(Mul, …)` arm, so a genuine product (`2x`) still
  multiplies; only a `trunc`-named text factor is intercepted. This is the last
  common scalar unary the surface was missing.

### Tests

- Two lower tests: `\operatorname{trunc}(a/b)` with `a=7, b=2` compute-and-decides
  to 3, and `\operatorname{trunc}((0 - a)/b)` to −3 (toward zero, NOT the floor −4).

## [0.29.0] - 2026-07-02 — binary `\gcd(a, b)` / `\lcm(a, b)` in `latex "…"`

### Added

- The `latex "…"` adapter now lowers **`\gcd(a, b)`** / **`\lcm(a, b)`** to the
  native `ComputeOp::Gcd` / `Lcm`, reusing the same two-argument binary-`Call`
  path as `\min`/`\max`: `BinFn` gains `Gcd`/`Lcm`, the adapter maps
  `Func::Gcd`/`Func::Lcm` → `ExprAst::Call2` (peeling the transparent
  `Group`/`Fenced` around the two-element `Sequence`, exactly-two enforced), and
  `lower_bin_fn` routes them to the engine ops. These are the `Func::Gcd`/`Lcm`
  variants the latex frontend already parsed but the adapter previously rejected.
  Only `Func::Det` and unknown `Other` remain unsupported.

### Tests

- One lower test: `\gcd(12, 18)` compute-and-decides to 6 and `\lcm(4, 6)` to 12.

## [0.28.0] - 2026-07-02 — binary `\min(a, b)` / `\max(a, b)` in `latex "…"`

### Added

- The `latex "…"` adapter now lowers the **binary** `\min(a, b)` / `\max(a, b)`
  to the native `ComputeOp::Min2` / `Max2` — the **first two-argument (binary)
  `Call` lowering**. The latex frontend parses the call's parenthesised argument
  as a two-element `Sequence`; the adapter peels the transparent `Group`/`Fenced`
  wrapper, requires **exactly two** operands, and builds the new
  `ExprAst::Call2(BinFn, …)` → `ComputeExpr::Bin(Min2/Max2, …)`. These are the
  `Func::Min`/`Func::Max` variants the frontend already parsed but the adapter
  previously rejected as "not yet supported".
- New surface AST: `BinFn { Min, Max }` and `ExprAst::Call2(BinFn, Box, Box)`,
  kept distinct from the single-argument `NamedFn`/`ExprAst::Call` (honest arity)
  and from the slot-reducing `AggOp::Min`/`Max`.
- A one-argument (`\min(a)`) or three-argument (`\min(a, b, c)`) call has no
  binary lowering and is a clean, explicit `UnsupportedLatexMath` error rather
  than a silent mis-lowering. `gcd`/`lcm`/`det`/`Other` remain unsupported.

### Tests

- Two lower tests: `\min(a, b)` / `\max(a, b)` compute-and-decide to the correct
  selected operand, and wrong-arity (`\min(a)`, `\min(a, b, c)`) is rejected.

## [0.27.0] - 2026-07-02 — rest of the trig family in `latex "…"` (`\arcsin`…/`\sinh`…/`\cot`/`\sec`/`\csc`)

### Added

- The `latex "…"` adapter now lowers the **inverse** (`\arcsin`/`\arccos`/`\arctan`),
  **hyperbolic** (`\sinh`/`\cosh`/`\tanh`), and **reciprocal** (`\cot`/`\sec`/`\csc`)
  trig functions to their native transcendental `ComputeOp`s via the existing
  `ExprAst::Call` / `NamedFn` mechanism — the `Func` variants the latex frontend
  already parses but the adapter previously rejected as "not yet supported". No
  latex-crate change; each is a `Scalar → Scalar` call.

### Notes

- The remaining `Func` variants are still a clean, explicit adapter error: the
  aggregate/multi-arg `min`/`max`/`gcd`/`lcm`/`det` (await a binary/variadic slice)
  and an unknown `Other` such as `\operatorname{trunc}` (truncation awaits faithful
  `\operatorname{…}(x)` juxtaposition handling).

## [0.26.0] - 2026-07-02 — named transcendental functions in `latex "…"` (`\sin`/`\cos`/`\tan`/`\ln`/`\log`/`\exp`)

### Added

- The `latex "…"` adapter now lowers a **named-function call** — `\sin(x)`,
  `\cos(x)`, `\tan(x)`, `\ln(x)`, `\log(x)`, `\exp(x)` (a `MathExpr::Call`) — to
  the matching native transcendental `ComputeOp` via a new `ExprAst::Call` and a
  `NamedFn` tag. This is the **named-function mechanism**: the first LaTeX consumer
  slice beyond the bracket-unary family (√ / ⁿ√ / Pow / Abs / Floor / Ceil /
  Round), and it reuses the `ComputeExpr::Unary` node — no new engine node type.
- Each is `Scalar → Scalar` (a transcendental of a dimensioned quantity is a
  category error, rejected by the engine) and drops the exact-rational sidecar.

### Notes

- Only the curated single-argument transcendental set is supported. The other
  `Func` variants (`min`/`max`/`gcd`/`lcm`/`det`, the inverse and hyperbolic trig,
  and an unknown `Other`) are a clean, explicit adapter error rather than a silent
  mis-lowering — `min`/`max` await the binary-op slice; truncation awaits
  `\operatorname{trunc}`.

## [0.25.0] - 2026-07-02 — round-to-nearest in `latex "…"` (lowers to `ComputeOp::Round`)

### Added

- The `latex "…"` adapter now lowers the standard **nearest-integer fence**
  `\left\lfloor x\right\rceil` (floor-left, ceil-right — a `MathExpr::Fenced` with
  delimiters `\lfloor`/`\rceil`) to the native `ComputeOp::Round` via a new
  `ExprAst::Round`. Round is nearest with **ties away from zero** (`⌊7/2⌉ = 4`).
- The asymmetric delimiters matter: `\lfloor…\rfloor` is floor, `\lfloor…\rceil`
  is round — the distinct *closing* delimiter selects the op, so the round arm sits
  beside the existing floor/ceil arms with no ambiguity. The latex frontend already
  surfaces these control-word delimiters as data, so no latex-crate change was
  needed.

### Notes

- Round is **dimension-preserving** (`⌊3.6 mmol⌉ = 4 mmol`) and the exact rational
  sidecar snaps to an integer. Truncation toward zero (`\operatorname{trunc}`) has
  no bracket notation and is deferred to the future named-function slice.

## [0.24.0] - 2026-07-02 — floor & ceiling in `latex "…"` (lower to `ComputeOp::Floor` / `ComputeOp::Ceil`)

### Added

- The `latex "…"` adapter now lowers a **floor fence** `\left\lfloor x\right\rfloor`
  and a **ceiling fence** `\left\lceil x\right\rceil` (a `MathExpr::Fenced` whose
  delimiters are `\lfloor`/`\rfloor` or `\lceil`/`\rceil`) to the native
  `ComputeOp::Floor` / `ComputeOp::Ceil` via new `ExprAst::Floor` / `ExprAst::Ceil`.
  This mirrors the absolute-value slice exactly: a delimiter pair → a unary op,
  reusing `ComputeExpr::Unary`. `⌊7/2⌋ = 3`, `⌈7/2⌉ = 4`.
- Any **other** delimiter pair (`|x|` abs, `(x)`, `[x]`, `\langle x\rangle`) keeps
  its existing meaning — only `\lfloor`/`\rfloor` and `\lceil`/`\rceil` carry the
  floor / ceiling meaning; the latex frontend already surfaces those control-word
  delimiters as data, so no latex-crate change was needed.

### Notes

- Floor and ceiling are **dimension-preserving** (`⌊3.7 mmol⌋ = 3 mmol`), so a
  dimensioned operand computes cleanly, and the exact rational sidecar snaps to an
  integer (`⌊7/2⌋ = 3/1`). Floor rounds toward −∞ (`⌊−7/2⌋ = −4`).

## [0.23.0] - 2026-07-01 — absolute value `|x|` in `latex "…"` (lowers to `ComputeOp::Abs`)

### Fixed / Added

- The `latex "…"` adapter now lowers an **absolute-value fence** `|x|` /
  `\left|x\right|` (a `MathExpr::Fenced` whose delimiters are `|`/`|`) to the
  native `ComputeOp::Abs` via a new `ExprAst::Abs`. **This fixes a latent
  correctness bug**: previously the `|…|` bars were silently dropped and `|a − b|`
  computed the *signed* difference — `|3 − 10|` returned `−7` instead of `7`.
- Any **other** delimiter pair (`(x)`, `[x]`, `\langle x\rangle`) is still
  presentation grouping and unwraps to the body's arithmetic exactly as before —
  only the `|`/`|` pair carries the absolute-value meaning.

### Notes

- Absolute value is **dimension-preserving** (`|−4 dollars| = 4 dollars`), so a
  dimensioned operand computes cleanly (unlike `\sqrt`, which needs a
  representable half-dimension). An abs *constraint* is piecewise-linear (not
  affine/polynomial), so the solver treats it as `Unknown`. adj-lang 0.22 → 0.23
  (logic-engine 0.28.0 / adj-constraint-solver 0.12.0).

## [0.22.0] - 2026-07-01 — nth root `\sqrt[n]{x}` in `latex "…"` (lowers to `x ^ (1/n)`)

### Added

- The `latex "…"` adapter now accepts an **nth root** `\sqrt[n]{x}` (a
  `MathExpr::Root` *with* an explicit degree), lowering it to `x ^ (1/n)` on the
  native `ComputeOp::Pow` — the reciprocal exponent `1/n` is computed once at
  adapt time and emitted as a single `Lit`. The cube root `\sqrt[3]{27}`
  computes `27 ^ (1/3) = 3`; the fourth root `\sqrt[4]{16}` computes `2`. This
  completes the radical family alongside the square root `\sqrt{x}` (0.21.0):
  both reuse the one power engine, so no new engine op is needed.
- The degree `n` must be a **positive integer** literal (`\sqrt[3]{…}`,
  `\sqrt[4]{…}`). A symbolic degree (`\sqrt[k]{…}`) has no numeric value and a
  zero degree would make `1/0` undefined, so both are rejected at adapt time
  (`UnsupportedLatexMath`), never silently mislowered.

### Notes

- As with the square root, a fractional-power base carries the engine's own
  dimensional rule: a `Scalar` (dimensionless) base computes cleanly, and a
  dimensioned base (a `\sqrt[3]{dollars}` has no representable third-dimension)
  is a `DimensionMismatch`, not a silently-wrong number. An nth-root *constraint*
  (`constrain latex "$\sqrt[3]{x} = 3$"`) is non-polynomial (a fractional
  exponent), so the solver treats it as `Unknown`, as before.

## [0.21.0] - 2026-07-01 — `\sqrt{x}` in `latex "…"` (lowers to `x ^ 0.5`)

### Added

- The `latex "…"` adapter now accepts a **square root** `\sqrt{x}` (a
  `MathExpr::Root` with no explicit degree), lowering it to `x ^ 0.5` on the
  native `ComputeOp::Pow` (adj-lang 0.20.0 / logic-engine 0.27.0). No new engine
  op: the power engine computes `√9 = 3` for a dimensionless (`Scalar`) base and
  cleanly rejects a dimensioned base (a `√dollars` has no representable
  half-dimension — a `DimensionMismatch`, not a silently-wrong number).

### Notes

- An **nth root** `\sqrt[n]{x}` (a `Root` with an explicit degree) is still
  unsupported — its `1/n` exponent needs a dedicated non-integer path — and
  falls through to the existing `UnsupportedLatexMath` error. A square-root
  constraint (`constrain latex "$\sqrt{x} = 3$"`) is non-polynomial (a
  fractional exponent), so the solver treats it as `Unknown`, as before.

## [0.20.0] - 2026-07-01 — native `^` power in `latex "…"` (emit `ComputeOp::Pow`)

### Changed

- The `latex "…"` adapter now lowers `x^n` to a **single native power node**
  (`ArithOp::Pow` → `logic_engine::ComputeOp::Pow`) instead of expanding it to a
  parse-time `x*x*…` chain. Two consequences:
  - The old **integer-exponent cap (0–8) is gone** — `latex "$x^{10}$"` (and
    higher) now compute; the engine computes the power and applies its own
    dimensional / overflow (`NonFinite`) rules.
  - The derivation tree shows one `^` step rather than a multiplication chain.
  The exponent must still be a **non-negative integer literal** (a symbolic
  exponent like `x^y` remains unsupported on this surface — a later slice).
- Added `ArithOp::Pow` to the `let`-formula AST (produced only by the latex
  adapter; the surface arithmetic grammar does not yet spell `^`). Removed the
  now-unused `expand_power` helper.

### Notes

- A latex `x^2 = 4` constraint still solves as a quadratic: the constraint
  solver's polynomial recogniser was taught to read `ComputeOp::Pow` (see
  adj-constraint-solver 0.11.0), so no solving capability regresses.

## [0.19.0] - 2026-06-27 — predicate RHS expressions

### Added

- Predicate evidence now accepts a full arithmetic expression on the right-hand
  side: `contributes 1000000 from answer == 3 / 10 to opt_a`. This lets a
  decomposer emit the printed option expression directly while ADJ evaluates and
  compares the values.
- Lowering now preserves the predicate RHS as a `ComputeExpr` through
  `logic_engine::PredicateContributionClause::from_lr_expr`.

## [0.18.0] - 2026-06-27 — native LaTeX math expressions and equations

### Added

- **`latex "<math>"` expression factors** — ADJ arithmetic now accepts LaTeX math
  anywhere an expression is already legal (`let`, ordinary `constrain`, objectives).
  The adapter parses the string with the repo's LaTeX `MathFrontend` and lowers the
  supported arithmetic subset into the existing `ExprAst`; callers do not normalize
  model output in host code.
- **`constrain latex "<equation>"`** — relation-shaped LaTeX (`$x^2 = 4$`,
  `x + y = 10`) lowers directly into the constraint system and flows through the
  existing native solver. Integer powers up to 8 lower to repeated multiplication, so
  the current polynomial root path handles equations like `x^2 = 4`.

### Changed

- `adj_lang.grammar` adds `latex_expr` and `constrain_latex_decl`; generated parser
  grammar regenerated.
- `adj_lang.tokens` now leaves string escapes raw so the adapter can preserve LaTeX
  commands while keeping existing provenance-string escape behavior.
- `adj-lang` now depends on the completed `latex`/`math-frontend` crates for native
  notation parsing.

## [0.17.0] - 2026-06-21 — multi-source corroboration (`cites … locator …`, ADJ-A9)

### Added

- **`cites "<source>" locator "<locator>"`** — a repeatable annotation that
  attaches a corroborating citation (a co-equal source for the *same* fact) to
  any clause (`prior`/`contributes`/`interacts`/`uncertain`/`relate`/rule). Each
  carries a required locator so the span is re-fetchable. Lowers onto
  `logic_engine::Provenance::corroborations` (documentary only — no change to the
  LR arithmetic). New `Annotation::Cites { source, locator }`; one new keyword
  `cites` (the existing `locator` keyword is reused as the separator, so no short
  `at` keyword is reserved).
- New lower tests: `cites_lowers_to_corroborations_in_order`,
  `cites_repeats_freely_unlike_at_most_once_source`.

### Changed

- `_lexer_grammar.rs` / `_parser_grammar.rs` regenerated (`cites` keyword +
  `cites_annotation` rule). The at-most-once checks for `source`/`locator`/
  `trust` are unchanged; only `cites` repeats.
- README §"awkwardness" updates A9 from "not yet covered" to covered.

## [0.16.1] - 2026-06-20 — string-literal escape hardening (byte-provenance)

### Added

- **`\t` (tab) escape** in string literals, completing the common escape set
  (`\"`, `\\`, `\n`, `\t`). Handled in `unquote_string`; the lexer regex
  `"([^"\\]|\\.)*"` already admitted it.

### Changed / hardened

- `unquote_string` now keeps a dangling trailing backslash verbatim instead of
  dropping it (defensive — the real lexer can't emit this, but the unescaper
  must never silently mutate a citation).
- Documented the full escape table on `unquote_string` and in `adj_lang.tokens`,
  spelling out *why* it is load-bearing: a `source "..."` provenance span must
  reproduce the cited page byte-for-byte after unescaping, so a span that itself
  contains a double quote (e.g. a histology page's `"Orphan Annie eye"` nuclei)
  is carried as `\"` and restored here. This unblocks grounding quote-containing
  verbatim spans that were previously (wrongly) treated as un-carryable.

### Tests

- Six new unit/round-trip tests pin every escape-table row, the
  unknown-escape-kept-verbatim rule, the dangling-backslash case, and an
  end-to-end `source "...\"...\""` → real-quote check through lexer+parser+adapter.

### Notes

- No grammar/AST/API change. `_lexer_grammar.rs` regenerated (line-number
  metadata only; the STRING pattern is unchanged).

## [0.16.0] - 2026-06-16 — context precedence surface (ADJ73 PR-B)

### Added

- **`rule { … context: <name> }`** — ground a rule in a CONTEXT (jurisdiction / guideline
  edition / specialty). Lowers to `logic_engine::Rule::with_context`.
- **`context_order { higher > lower, … }`** — a top-level statement declaring grounded context
  precedence edges; each `a > b` lowers to `KnowledgeBase::add_context_outranks`. The resolver
  consults this BEFORE the priority tier (lex superior): a rule in a greater context defeats a
  conflicting one in a lesser context regardless of tier.
- Grammar: `context_order_decl` added to `statement`; `rule_decl` gains a trailing
  `[ "context" COLON IDENT ]` (after `[ "priority" … ]`). `context_order`/`context` are
  IDENT-matched literals; `>` is the existing `GT` operator. `_parser_grammar.rs` /
  `_lexer_grammar.rs` regenerated.
- AST: `Statement::Rule` gains `context: Option<String>`; new `Statement::ContextOrder { edges }`.
  Adapter: a shared `ident_after_keyword` helper extracts both `priority:` and `context:`;
  `adapt_context_order` pairs the `IDENT > IDENT` edges (skipping the `>` Name token).

### Notes

Surface for logic-engine 0.19's grounded context precedence. A test compiles
`context_order { ninth_circuit > district_court }` + `context:`-tagged rules and verifies via
`enumerate_governing` that the higher-context rule governs **even with a lower priority tier**
(lex superior), and that multi-edge orders lower transitively. The grounded `context-precedence`
*rulebook* (byte-provenanced `outranks_context` edges + recency/appeal meta-rules) is the next
slice.

## [0.15.0] - 2026-06-16 — precedence surface syntax (ADJ73 PR-C)

### Added

- **`rule { … priority: <tier> }`** — optional named precedence tier on a derivation rule
  (`default` | `specific` | `authoritative` | `mandatory`). Lowers to
  `logic_engine::Rule::with_priority(Priority::…)`; absent ⇒ `Default`. An unknown tier is a
  clean `LowerError::UnknownPriorityTier`, not a silent default.
- **`functional <pred>(<arg>, …)`** — top-level declaration that a predicate is functional on
  its last argument (arg names are placeholders; only functor + arity matter). Lowers to
  `KnowledgeBase::declare_functional(functor, arity)`. Two derivations sharing the key but
  differing on the last arg then *conflict*, and the `priority:` tier picks the governing one
  (`logic_engine::govern::enumerate_governing`).
- Grammar: `functional_decl` added to `statement`; `rule_decl` gains a trailing
  `[ "priority" COLON IDENT ]`. `functional`/`priority` are IDENT-matched literals (no new lexer
  tokens). `_parser_grammar.rs` / `_lexer_grammar.rs` regenerated.
- AST: `Statement::Functional { functor, arity }`; `Statement::Rule` gains `priority: Option<String>`.

### Notes

This is the **surface** for the merged ADJ73 engine core (logic-engine 0.18 `Priority` /
`declare_functional` / `enumerate_governing`). Tests compile the new syntax and verify the full
path parse → lower → `enumerate_governing` (higher tier governs; equal tiers conflict;
non-functional predicates unaffected — back-compat). `adj-lang-cli` *governing* rendering is the
next slice (PR-3); the CLI's existing `decide`/recall paths are unchanged.

## [0.14.0] - 2026-06-16 — `rule { head: … when: … }` derivation rules (Datalog clauses)

### Added

- **`rule { head: <term>  when: <lit>, <lit> … }`** — a DERIVATION RULE with a body
  (a Horn clause / Datalog rule), the missing primitive that lets a `rulebook`
  express CONDITIONAL knowledge, not just ground `relate` edges + LR clauses. Where
  `relate` asserts a ground fact, a `rule` lets the engine DERIVE its head whenever
  the body holds under the current substitution — variables (`$D`) bind across head
  and body, and a literal prefixed `not` is negation-as-failure. Lowers to the
  existing `logic_engine::Rule { head, body }`, so `? head($X)` enumerates every
  derivable answer via the same SLD/unification machinery `relate` resolves through.
  This is the keystone for moving DOMAIN RULES (contraindications, step-therapy,
  formulary policy) out of host-code and into ADJ: each domain is authored once as a
  `rule`-bearing `rulebook`, byte-provenanced + gated into the CAS, and the engine
  derives consequences from per-case facts. Rules carry the same `source`/`locator`/
  `trust` annotations every grounded clause carries (new `Rule.provenance` field).
  Block form mirrors `rulebook { … }`; `head`/`when`/`not` are IDENT-matched literals.

## [0.13.0] - 2026-06-14 — relational queries pass vocabulary enforcement (MYCIN-2026 REL-3)

### Changed

- **`enforce_vocabulary` accepts a binding query over a defined `relation`.** A
  query whose functor is a `define`d relation (`? deficient_in(tay_sachs, $E)`) is
  now valid under a `use`d vocabulary, alongside the existing hypothesis queries —
  relational recall is the single-hop special case of the differential, so the
  vocabulary check accepts either. Previously a relational query inside a `use`
  scope was wrongly rejected as a non-hypothesis (`UndefinedTerm`).

## [0.12.0] - 2026-06-14 — relational recall: `relate` edges + binding queries (MYCIN-2026 REL-2)

### Added

- **`relate <rel>(<args>)`** — a ground RELATIONAL EDGE, the first-class fact
  type behind relational recall (the board-exam substrate). Asserts a typed edge
  in a knowledge graph (`relate deficient_in(tay_sachs, hexosaminidase_a)`),
  carrying the usual `source`/`locator`/`trust` annotations; the lowerer turns it
  into a `logic_engine::Fact` whose `provenance` carries the citation, so a
  binding query's answer can be returned WITH a proof. New `Statement::Relate`;
  grammar `relate_decl`.
- **Logic variables (`$Name`) + binding queries.** A `$`-prefixed `VAR` token
  may appear as a term argument in a query goal: `? deficient_in(tay_sachs, $E)`
  asks the engine to BIND `$E` to whatever the grounded edge holds (and the
  reverse `? deficient_in($D, hexosaminidase_a)` is free). Resolved by the
  existing SLD/unification machinery (`logic_engine::enumerate_all`). New
  `Term::Var`; repeated variables within one goal share identity. A ground
  hypothesis query (`? bacterial`) is unchanged — fact recall is the single-hop,
  zero-uncertainty special case of the same engine.
- **`entity` / `relation from <domain> to <range>` define kinds** — the
  controlled vocabulary can now declare graph NODE kinds (`disease`, `enzyme`)
  and typed EDGE kinds (`deficient_in : relation from disease to enzyme`). New
  `DefineKind::Entity` and `DefineKind::Relation { from, to }`.

### Notes

- Grammar/lexer regenerated (`_lexer_grammar.rs`, `_parser_grammar.rs`) from the
  updated `adj_lang.{tokens,grammar}`. Strict relation-argument type enforcement
  is a later slice; REL-2 lands the surface + lowering + resolution. Pairs with
  `logic-engine` 0.15.0 (`Fact::provenance`).

## [0.11.0] - 2026-06-12 — `import "path"` (MYCIN-2026 M3)

### Added

- **`import "<relative path>"`** — compose a program across files: a dictionary,
  a rulebook that imports + `use`s it, and a case that imports the rulebook can
  each be their own checked-in `.adj`. New `Statement::Import(String)`; grammar
  `import_decl`.
- **`resolve` module** — the import-graph policy, with **no filesystem I/O** (it
  drives an injected `ImportProvider`, so the graph logic is unit-testable
  without a disk and the FS trust boundary lives in the caller):
  - **relative** — `provider.resolve(importer, literal)` → canonical id.
  - **idempotent** — a `visited` set keyed by canonical id; a file merges once
    (diamond imports don't duplicate clauses).
  - **acyclic** — a DFS stack; re-entering a stacked id is `ImportError::Cycle`
    (cycle check precedes the idempotency check, so a cycle never masquerades as
    a harmless repeat). Self-import included.
  - **bounded** — `ImportLimits { max_depth, max_files }` (default 32 / 256);
    past either, `DepthExceeded` / `TooManyFiles`. Depth is checked on every
    descent, so the graph walk can't exceed `max_depth` frames on hostile input.
  - Merge order is depth-first **post-order** — an imported file's declarations
    precede the importer's, so a dictionary is in scope by the time the rulebook
    that `use`s it is merged.
- **`compile_with_imports(root_id, provider, limits)`** — resolve then lower, with
  a combined `CompileWithImportsError`. `lower` now rejects a stray unresolved
  `import` as `LowerError::UnresolvedImport` (never silently dropped).
- 8 resolver tests (3-file chain, diamond, direct + self cycle, depth + fan-out
  bounds, unresolvable path, importer-relative). Grammar regenerated. **M3
  completes the MYCIN-2026 language foundation (M0–M3).**

## [0.10.0] - 2026-06-12 — `rulebook` + `use` (MYCIN-2026 M2)

### Added

- **`rulebook <name> { … }`** — a named, reusable block of clauses
  (`prior`/`contributes`/`interacts`/`uncertain`), so a body of adjudicatable
  knowledge can be written once, checked in as code, and (M3) imported. A
  rulebook is a *container*, not a namespace: its clauses lower into the
  `KnowledgeBase` exactly as if written at top level (`flatten_clauses`). New
  AST `Statement::Rulebook { name, statements }`.
- **`use <dictionary>`** — binds a declared `dictionary` (by name) as the
  controlled vocabulary the enclosing scope's clauses are checked against. Legal
  at top level or inside a `rulebook`. New AST `Statement::Use(String)`.
- **Scoped vocabulary enforcement (M2).** When any `use` appears, enforcement
  becomes *per-scope*: a top-level `use D` checks the top-level clauses against
  `D`; a rulebook's own `use D'` checks that rulebook against `D'` (falling back
  to a top-level `use`). A scope with no `use` is unchecked — a rulebook opts in
  to checking by `use`-ing a dictionary. A `use` of a dictionary the program
  never declared is `LowerError::UndefinedDictionary`. When **no** `use` appears
  anywhere, M1 whole-program enforcement is unchanged (fully backward-compatible).
- **Rulebooks are flat.** A `rulebook` nested directly in another is a clean
  `LowerError::NestedRulebook` (nesting has no defined scoping semantics; the
  refusal also keeps clause-flattening non-recursive, so deeply-nested untrusted
  source cannot drive unbounded recursion in the lowerer).
- 8 tests (rulebook lowers like top-level; `use` checks/rejects terms; undefined
  dictionary; no-`use` rulebook unchecked; top-level `use` scoping; nested
  rulebook rejected). Grammar regenerated. Next (MYCIN-2026): M3 `import "path"`.

## [0.9.0] - 2026-06-12 — `dictionary` + `define` (MYCIN-2026 M1)

### Added

- **`dictionary <name> { … }` and `define`** — the controlled vocabulary as a
  first-class, named grammar construct (MYCIN-2026). `define <name> : hypothesis`
  registers a hypothesis; `define <name> : finding values [v…]` registers a
  finding functor with a *closed* value domain; `surface "…", "…"` lists the
  decomposer's surface forms. A `define` is valid bare or inside a `dictionary`
  block. New `LBRACK`/`RBRACK` tokens; grammar regenerated.
- AST: `Statement::Define(Define)` + `Statement::Dictionary { name, defines }`;
  `Define { name, kind: DefineKind::{Hypothesis | Finding{values}}, surfaces }`
  (exported). `LoweredProgram` gains `dictionary: Vec<Define>`.
- **Compile-time vocabulary enforcement** (replaces the prototype's side-car
  `dict_lint.py`): when a program declares a dictionary (≥1 `define`), every
  finding / hypothesis used in `prior`/`contributes`/`interacts`/`observe`/`?`
  must be defined, and a finding value must be in its declared domain — else
  `LowerError::UndefinedTerm` / `ValueNotInDomain`. The IR a decomposer emits and
  the rulebook it compiles against share one closed vocabulary by construction.
  A program with **no** dictionary is unchecked (backward-compatible). 6 tests.
- Next (MYCIN-2026): M2 `rulebook` + `use`, M3 `import`.

## [0.8.0] - 2026-06-11 — `minimize`/`maximize` LP objective (ADJ constraints track C2)

### Added

- **`minimize <expr>` / `maximize <expr>`** surface syntax — a linear-programming
  objective over the declared symbols. New grammar rule `optimize_decl` (like
  `solve`/`check`, the keywords are IDENT-matched literals, not lexer keywords),
  regenerated `_parser_grammar.rs`.
- AST: `Statement::Optimize { dir, objective }` + `OptDir { Minimize, Maximize }`
  (exported). Adapter `adapt_optimize`. The objective is kept as an unevaluated
  `ComputeExpr` (it mentions the symbols the LP solver assigns).
- `ConstraintSystem` gains `objective: Option<(OptDir, ComputeExpr)>`; `is_empty`
  accounts for it. The solver (`adj-constraint-solver` 0.6.0) reads it. 2 tests.

## [0.7.0] - 2026-06-11 — constraint sublanguage (symbols + constrain/solve/check, ADJ constraints B1)

### Added

- **`symbol <name> : <sort>`** — declare an unknown the engine will solve for
  (`sort` is a dimensional sort term: `scalar`, `money(usd)`, …).
- **`constrain <expr> <relop> <expr>`** — assert an (in)equality. `relop` is
  `>= <= > < == = !=`; operands reuse the `let` arithmetic `expr`, so a
  constraint may mention observed slots, earlier `let`s, and symbols. (Typed
  literals like `money(2000, usd)` are referenced via an `observe`d name, since
  constraint operands are arithmetic exprs.)
- **`solve for { a, b, … }`** — name the unknowns to solve for; **`check`** —
  ask whether the constraint set is satisfiable.
- New AST: `Statement::{Symbol, Constrain, SolveFor, Check}` + `RelOp`.
- **`ConstraintSystem`** (`symbols`, `constraints`, `solve_for`, `check`),
  exposed on `LoweredProgram.constraints`. The lowerer builds it, keeping each
  constraint's two sides as **unevaluated `ComputeExpr` trees** (they mention
  symbols the solver assigns). **No solving yet** — the reuse solver backends
  (`cas-solve` / `SatTactic` / `LiaTactic`) are wired in track B2.

### Grammar

- `.tokens`: added `COLON` (`:`) and `NE` (`!=`, listed before `>`/`<` for
  maximal munch).
- `.grammar`: `symbol_decl` / `constrain_decl` / `relop` / `solve_decl` /
  `check_decl`. Regenerated `_lexer_grammar.rs` / `_parser_grammar.rs`.

## [0.6.0] - 2026-06-11 — `let` + arithmetic (computed values, ADJ expansion step 3b)

### Added

- **`let <name> = <expr>`** — bind a value the engine **computes** on the CPU
  from a formula the model writes. `<expr>` supports `+ - * /` with standard
  precedence and parentheses, references to observed slots and earlier `let`s,
  numeric literals, and aggregations `sum/count/min/max/avg(slot)` over every
  observation of a slot. The lowerer evaluates the formula via
  `logic_engine::compute` against the facts seen so far and binds the resulting
  `Derived` (with its derivation tree) into the KB — so a **predicate fires
  over a computed value exactly as over an observed one**
  (`from csf_ratio <= 0.4 to bacterial`). The model never does the arithmetic.
- New AST: `Statement::Let { name, expr }`, `ExprAst` (`Ref/Lit/Bin/Agg`),
  `ArithOp`, `AggOp`. New `LowerError::ComputationFailed` (unknown slot,
  division by zero, empty aggregation, … surfaced cleanly, never a panic).

### Grammar

- `.tokens`: added `PLUS - * / =` (`EQUALS`). Two ordering disciplines:
  `EQUALS` after `EQEQ` (so `==` wins maximal munch), and the arithmetic
  operators after `NUMBER` so a negative literal `-5` still lexes as one
  `NUMBER(-5)` — a binary `-` only matches a `-` not glued to a digit, so a
  `let` formula must **space its operators** (`a - 5`, `total - discount`).
- `.grammar`: `let_decl` + the `expr` / `term_expr` / `factor` / `agg`
  precedence cascade. Regenerated `_lexer_grammar.rs` / `_parser_grammar.rs`.

## [0.5.0] - 2026-06-10 — predicate evidence + valued facts

### Added

- **Numeric predicate evidence in `contributes`** — first-class operator
  syntax: `contributes <lr> from <slot> >= <value> to <verdict>`. The five
  comparison operators `>= <= > < ==` lower to a
  `logic_engine::PredicateContributionClause`; a saturating `lr` makes the
  rule **deterministic** (deterministic = the saturating limit of a
  probabilistic LR, evaluated on the CPU at decision time — the model that
  authored the rulebook never ran the comparison).
- **Valued facts** — `observe gross_income(18000)`: numeric literals are now
  allowed as compound arguments (`term = IDENT [ LPAREN ( term | NUMBER )
  { COMMA ( term | NUMBER ) } RPAREN ]`). These are the facts predicates
  read. New `ast::Term::Num(f64)`.
- New AST types: `ast::Evidence { Term | Predicate { slot, op, value } }`
  (the evidence side of `contributes`) and `ast::CmpOp`. `Statement::Contributes`
  now carries `evidence: Evidence` instead of `evidence: Term`.

### Grammar

- `.tokens`: added comparison-operator tokens `GE LE EQEQ GT LT`
  (two-character operators listed before single-character ones so maximal
  munch tokenises `>=` before `>`).
- `.grammar`: `contributes_decl` now takes `evidence = predicate | term`;
  the `predicate | term` alternation relies on the parser's full
  backtracking (both start with `IDENT`). Regenerated
  `_lexer_grammar.rs` / `_parser_grammar.rs`.

## [0.4.0] - 2026-06-10 — differential decision over `?` queries

### Added

- **`decide(&LoweredProgram) -> Differential`** and
  **`compile_and_decide(src) -> Differential`** — treat a program's `? h`
  query lines as the set of *competing hypotheses* and run the new
  `logic_engine::differential` over them: rank by posterior, pick the
  argmax, report the between-hypothesis margin, and kick back when an open
  uncertainty could flip the ranking. A multi-`?` adj-lang program is now
  directly a differential (the natural reading); a single-`?` program
  yields a determinate single-hypothesis result. No grammar change — the
  competing set is already expressible as multiple `?` lines.

## [0.3.0] - 2026-06-02 — grammar-driven frontend

### Changed

- **Replaced the hand-written lexer and parser with the repo's
  grammar-driven infrastructure.** Conformant with every other
  language frontend in the codebase (csharp-lexer, dot-parser,
  dartmouth-basic, css, javascript, java, ruby, sql, …).
- Grammars now live in
  [`code/grammars/adj_lang.tokens`](../../../grammars/adj_lang.tokens)
  and
  [`code/grammars/adj_lang.grammar`](../../../grammars/adj_lang.grammar)
  as the canonical source-of-truth. Any future language port of the
  Adj-Lang frontend (Elixir / Go / Python / Ruby / Swift /
  TypeScript) regenerates from the same files.
- `src/_lexer_grammar.rs` and `src/_parser_grammar.rs` are
  auto-generated by `grammar-tools compile-tokens` /
  `compile-grammar`. **DO NOT EDIT — regenerate from source.**
- New `src/adapter.rs` maps the generic
  `parser::grammar_parser::GrammarASTNode` tree into the typed
  `ast::Statement` enum the lowerer already consumes. Keeping the
  typed AST means the lowerer (`src/lower.rs`) is unchanged.
- `src/lib.rs` shrinks to a thin `parse()` / `compile()` wrapper
  over `GrammarLexer + GrammarParser + adapt_program + lower`.

### Removed

- `src/lexer.rs` (371 LOC hand-written).
- `src/parser.rs` (535 LOC hand-written recursive descent + depth guard).

### Preserved features

- `uncertain { e1, e2, ... } for <conclusion>` statement (added in
  0.2.0 via #4933 / #4935) — now expressed via the declarative
  grammar (`uncertain_decl` in `adj_lang.grammar`, `KwUncertain` +
  `LBRACE` + `RBRACE` in `adj_lang.tokens`, `adapt_uncertain` in
  `adapter.rs`).

### Why

The original 0.1.0 frontend was hand-written despite the repo
having `grammar-tools`, `lexer`, and `parser` crates for exactly
this purpose. The user (correctly) flagged this as off-pattern.
The grammar-driven approach gives us:

- **Cross-language portability** — the same `.tokens` / `.grammar`
  files drive Elixir / Go / Python / Ruby / Rust / Swift /
  TypeScript implementations.
- **Versioning** — `@version 1` in the grammars enables multiple
  Adj-Lang versions to coexist.
- **Language-server support** (LS02).
- **Browser-compatible builds** via WASM (LANG63).
- **Single source of truth** — the grammar files are the spec.

## [0.2.0] - 2026-06-02

### Added

- New `uncertain { e1, e2, ... } for <conclusion>` statement,
  lowering to `logic_engine::UncertaintyMarker`. Accepts the
  standard `source`/`locator`/`trust` annotations.
- New lexer tokens: `KwUncertain`, `LBrace`, `RBrace`.
- 2 new parser tests (basic + annotated forms), 1 new lower test
  (`uncertain_statement_produces_voi_report_on_aggregation`
  confirms end-to-end: surface syntax compiles, runs through
  `SearchMode::LRAggregate`, and produces a non-empty
  `uncertainties` vector with the right VOI logit range).

Total tests: 29 (was 26 in 0.1.0).

### ADJ46 awkwardness items dissolved by 0.2.0

- **A5** (uncertainty markers) — fully addressed at the surface
  layer. The IR pipeline can now hand off "no clear precipitator"
  as a structured `uncertain { … } for …` clause, and the engine
  surfaces it back to the user as a VOI report.

## [0.1.0] - 2026-06-02

### Added

- Initial release. Hand-written lexer + recursive-descent parser +
  lowering pass for the v0.1 surface syntax described in `README.md`.
- Grammar covers: `prior`, `contributes`, `interacts`, `observe`,
  `?`-prefix queries; `source`/`locator`/`trust` annotations;
  identifiers, compound terms with multi-arg arity, line comments,
  string literals with backslash escapes.
- Lowering produces a `LoweredProgram { kb, queries }` where `kb` is
  populated with `Fact`, `PriorClause`, `ContributionClause`, and
  `JointContributionClause` entries ready for
  `logic_engine::search`.
- Lowerer enforces: at most one `prior` per conclusion, at most one
  `source`/`locator`/`trust` annotation per statement.
- Default trust tier when a `source` annotation is present but no
  `trust` is given is `Authoritative`. When neither is given, the
  tier is `Unattributed`.
- 24 tests across lexer (9), parser (8), and lower (6+1 headline).
- **Headline test**:
  `lowers_full_acs_rulebook_and_reproduces_adj36_posterior` compiles
  the ACS rulebook end-to-end (parse + lower + search via
  `SearchMode::LRAggregate`) and reproduces ADJ36's 28.1% posterior.

### ADJ46 awkwardness items dissolved

- **A4** — `interacts` is syntactically distinct from `contributes`,
  so the proof DAG can name the interaction term explicitly.
- **A10** — the rulebook surface is now a domain-expert-readable
  DSL, not hand-written Rust.

### Not yet shipped

- A5 (uncertainty markers), A7 (kickback variant), A8 (counterfactual
  queries), A9 (multi-source aggregation). Each is a small additive
  grammar extension; the parser is structured so each new clause kind
  adds one arm to `parser::parse_statement` and one variant to the
  lowerer.
