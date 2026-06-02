# Changelog

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
