# Changelog

All notable changes to this project will be documented in this file.

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
