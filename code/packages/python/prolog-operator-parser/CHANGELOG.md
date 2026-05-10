# Changelog

## Unreleased

- Added `ParsedOperatorTerm` and `parse_operator_named_term_tokens(...)` so
  dialect frontends can parse one term while retaining named variable bindings.

## 0.1.0

- Added a token-level operator-aware Prolog parser shared by dialect frontends
- Added optional directive parsing and operator-table overrides for dialect source parsing
- Added file-scoped `op/3` execution so operator tables can evolve while parsing one source
- Added shared DCG rule expansion for `-->` clauses, including braced grammar goals
