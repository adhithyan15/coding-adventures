# ADJ deterministic-via-probabilistic — build plan (resumable)

## Principle (locked)
Deterministic decision-making is a **special case of probabilistic** — ONE engine. A verdict is a
HYPOTHESIS; a hard rule "when C then V" is `contributes <saturating LR> from C to V`;
DETERMINATE/INDETERMINATE/CONFLICT **fall out of the existing `differential`** (leader / kickback /
insufficient-evidence). Do NOT build a second engine. See memory
`feedback_deterministic_is_probabilistic_special_case`.

## The only real gap: predicates (first-class operator syntax, user's choice)
Express `contributes 1e6 from gross_income >= 14600 to required_to_file` over a valued fact
`observe gross_income(18000)`. The engine evaluates the predicate against the observed numeric value
on CPU. "Deterministic" = a saturating LR.

## Build slices (PRs)

### Slice 0 — regen tooling (LANDED)
`adj-lang/src/bin/regen_grammars.rs` — reads `code/grammars/adj_lang.tokens` + `.grammar`, calls
`grammar_tools::compiler::compile_token_grammar` / `compile_parser_grammar` (after
`parse_token_grammar` / `parse_parser_grammar`), writes `src/_lexer_grammar.rs` /
`src/_parser_grammar.rs`. The codegen entry point the crate previously lacked. **Verified idempotent**
on the unchanged grammar (reproduces the committed files exactly). Run:
`cargo run -p adj-lang --bin regen_grammars`.

### Slice 1 — predicate feature (ONE coupled PR — NOT splittable)
**Finding (verified):** the adapter navigates the parse tree by rule/position
(`adapter.rs` looks for the "conclusion term" child of `contributes_decl`). Inserting an `evidence`
non-terminal changes the tree shape and breaks the adapter — so the grammar, AST, adapter, lower, and
engine MUST change together in one PR (a grammar-only change leaves 8 adj-lang tests red). Steps:
- **Tokens:** add `GE = ">="  LE = "<="  EQEQ = "=="  GT = ">"  LT = "<"` (multi-char before single).
  NUMBER already supports `1e6`.
- **Grammar:** `contributes_decl = "contributes" NUMBER "from" evidence "to" term { annotation }` ;
  `evidence = predicate | term` ; `predicate = IDENT (GE|LE|GT|LT|EQEQ) NUMBER`. Regenerate.
- **ast.rs:** `Statement::Contributes` evidence → `Evidence::Term(Term) | Evidence::Predicate{slot,op,value}`.
- **adapter.rs:** handle the `evidence` node (unwrap to term, or build a predicate); update the
  `contributes_decl` child navigation so existing term contributions still adapt.
- **lower.rs:** lower a predicate-gated contribution to a new logic_engine clause `(slot, op, value, logit)`.
- **logic-engine (`lr_aggregate`):** `PredicateContributionClause`; when aggregating a conclusion,
  find the observed valued fact `slot(V)` (V = `Term::Num`), evaluate the predicate, add `logit_delta`
  if true; `DerivationOrigin::FromPredicateContribution` cites the clause + observed value. Valued fact
  `observe gross_income(18000)` already lowers to `Compound{"gross_income",[Num(18000)]}`.
- **adj-lang-cli:** render the predicate step in the proof DAG JSON.
- Tests at adapter / lower / engine / CLI layers; `cargo build --workspace` (ignore pre-existing
  `uefi`); revert `cargo fmt` churn outside touched files.

### Slice 2 — the Haiku run (separate PR, after the language lands)
Haiku decomposes each run100/run100b case (policy → saturating predicate rules, scenario → valued
facts), emit unified `.adj`, `adj-lang-cli` executes on CPU (0 answer-time calls), score vs gold
(DETERMINATE/INDETERMINATE via the differential). Proves dumb-model + CPU-bound + auditable at scale.

## Verification
`cargo run -p adj-lang --bin regen_grammars` (regenerates, idempotent), `cargo test -p adj-lang`,
`cargo test -p logic-engine`, a golden `adj-lang-cli` run on a predicate program, `cargo build --workspace`
(ignore the pre-existing `uefi` failure). Revert `cargo fmt` churn outside touched files.
