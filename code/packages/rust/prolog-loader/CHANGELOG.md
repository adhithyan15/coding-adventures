# Changelog

All notable changes to this project will be documented in this file.

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
