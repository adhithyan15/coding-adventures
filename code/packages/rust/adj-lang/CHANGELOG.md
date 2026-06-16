# Changelog

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
