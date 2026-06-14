# Changelog

All notable changes to this project will be documented in this file.

## [0.4.0] - 2026-05-11

### Added

ProbLog source syntax — `0.7 :: clause.` now parses and loads end-to-end.

- New `ProgramItem::ProbabilisticFact { term, probability }` and
  `ProgramItem::ProbabilisticRule { head, body, probability }` variants
  (added in `prolog-parser` v0.2 — see that crate's changelog).
- `load_program_items` translates the new variants into
  `Fact::with_probability` and `Rule::with_probability` (the latter
  added in `logic-engine` alongside the ProblogProgram builder in
  v0.3). The result lands in the same `KnowledgeBase` shape the WMC
  backend already understands, so a single `load_source` + `search`
  round-trip computes the right probability.
- New `LoaderError::ProbabilityOutOfRange { value: f64 }` —
  surfaced when a parsed probability is NaN or outside `[0, 1]`.
  Range-checked at load time so the engine never sees a nonsense
  weight.

### Tests

10 new integration tests in
`prolog-loader/tests/integration_problog_source.rs`:
- single probabilistic fact returns its probability
- conjunctive rule over independent probabilistic facts multiplies
- probabilistic rule gating a certain premise
- boundary probabilities `0.0` and `1.0` behave correctly
- unprovable query returns `0.0`
- out-of-range probability (`1.5 :: …` and `2 :: …`) surfaces as
  `ProbabilityOutOfRange`
- mixed deterministic and probabilistic clauses in one program
- interleaved order works

## [0.3.0] - 2026-05-11

### Added

- `problog` module — builder API for probabilistic Prolog programs.
  Bridges the (working) engine half of ProbLog —
  `Fact::with_probability`, `Rule::with_probability`, the WMC
  backend in `logic_engine::wmc` — and the (not-yet-built) source
  half (the ISO-Prolog grammar doesn't recognise `0.7::fact.` yet).
- `ProblogProgram` builder with chainable
  `with_fact` / `with_prob_fact(p, term)` /
  `with_rule(head, body)` / `with_prob_rule(p, head, body)` /
  `with_query(goals)` methods, plus `build_kb()` (no execute),
  `execute()` (AutoDetect), and `execute_with_mode(mode)`.
- Multi-goal queries on the ProbLog path use the same synthetic-head
  rewrite as the deterministic loader — head atom
  `__problog_query_N`, unsourcable because `_`-prefixed identifiers
  tokenise as variables.
- Re-exported at the crate root: `prolog_loader::ProblogProgram`.
- 9 new unit tests + 9 new integration tests in
  `tests/integration_problog_e2e.rs`. Coverage: single probabilistic
  fact, conjunctive rule over independent probabilistic facts (WMC
  multiplies), probabilistic rule gating a certain premise, mixed
  deterministic/probabilistic programs, unprovable queries return
  0.0, boundary probabilities 0.0 and 1.0, multi-goal queries via
  the synthetic-head rewrite, deterministic-only program through the
  ProbLog builder, `build_kb()` without executing.

### Notes

Engine half of the ProbLog E2E story; the source half — accepting
`0.7::edge(a, b).` syntax — is gated on grammar work. When that
lands, a future `load_problog_source(src)` will be a thin wrapper
translating parsed clauses into the same `ProblogProgram` builder
calls these tests exercise.

### Companion change in logic-engine

`Rule::with_probability(head, body, p)` added to mirror
`Fact::with_probability`. Purely additive.

## [0.2.0] - 2026-05-11

### Added

- `execute(src, mode) -> Result<(KnowledgeBase, Vec<QueryRun>), LoaderError>`
  — one-call end-to-end runner. Parses a Prolog source string, builds
  the KB, runs every `?-` query the file contains, and returns the KB
  plus a `QueryRun` per query.
- `run_all_queries(&mut LoadedProgram, SearchMode) -> Vec<QueryRun>`
  — runs queries against an already-loaded program. Mutates the KB
  with one synthetic rule per multi-goal query
  (`__query_N :- g1, g2, ...`) so the engine sees a single head term
  to search; the canonical Prolog top-level rewrite, routed through
  `BodyLiteral` so existing conjunction handling does the work.
- `QueryRun { goals, searched, result }` — engine answer per query.
  `succeeded()` returns `true` for both `FindFirstResult(Some)` and
  any `EnumerateAllResult` with `probability > 0.0`. `probability()`
  returns LP19-style `1.0` / `0.0` for the deterministic case and the
  WMC value for the probabilistic case.
- New integration test crate `tests/integration_e2e.rs` exercises the
  full pipeline (prolog-lexer → prolog-parser → prolog-loader →
  logic-engine) on 14 scenarios: empty program, single fact, ground /
  variable queries, compound facts, rule chains via conjunction,
  recursive ancestor, multi-goal queries via the synthetic-rule
  rewrite, line comments, multi-query source-order preservation,
  parse-error path, and `EnumerateAll` mode on a certain-only KB
  returning probability 1.0.

### Notes

NAF (`\+`) parsing from source is not yet wired through the ISO-Prolog
grammar; the loader's `naf_or_pos` lowering is unit-tested against a
hand-built compound term. Re-enable the source-level E2E NAF test
once the grammar grows the production. Block comments
(`/* ... */`) are similarly grammar-side work.

## [0.1.0] - 2026-05-11

### Added

- `load_source(src)` — full pipeline: parse Prolog text, build a
  `logic_engine::KnowledgeBase` from the facts and rules, return the
  KB plus a `Vec<Vec<Term>>` of the file's top-level queries.
- `load_program_items(items)` — same lowering applied to a
  pre-parsed `Vec<ProgramItem>` from `prolog-parser`. Useful for
  callers that have already parsed the source.
- `LoaderError` enum: `ParseFailed(GrammarParseError)`,
  `EmptyConjunctionBody` (a Rule with `:-` and no goals — currently
  rejected as malformed).
- `LoadedProgram { kb, queries }` — the value returned by
  `load_source`. The KB is ready to call `search` / `find_first` on;
  `queries` is a flat list of conjunction-bodies, one per `?-` in
  the source.
- Negation-as-failure recognition: when a body literal is the
  compound `'\+'(G)`, it is lowered to
  `BodyLiteral::Neg(G)`. Anything else stays as `BodyLiteral::Pos(_)`.
- 9 tests covering: bare-atom facts; compound facts; rules with
  one and two body goals; multiple queries from a single source;
  the family-relations example end-to-end through search; NAF
  recognition; an empty body rejected; an error case for
  syntactically invalid input.

### Notes

This crate is the last hop in the Rust Prolog pipeline:

```
   text -> prolog-lexer -> prolog-parser -> prolog-loader -> logic-engine
```

It mirrors the role of the Python `prolog-loader` package. The
loader does not yet handle module declarations, operator directives,
or DCG expansion — those are planned follow-ups tied to specific
Python-parity work.
