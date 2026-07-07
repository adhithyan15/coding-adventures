# Changelog — regex-engine

## 0.1.0 — Unreleased

Initial release: the ASCII-mode `is_match` core of a zero-dependency,
linear-time regular-expression engine, created to remove the third-party `regex`
crate from the Engram stack (Engram zero-dependency program,
`code/specs/engram-zero-dep-plan.md`, Phase D0).

### Added

- Parser (`ast`): literals, `.`, `\d \D \w \W \s \S` (ASCII) + escaped
  metacharacters, `[...]`/`[^...]` classes with ranges, `(...)`/`(?:...)`,
  `|`, `* + ?` and `{m}`/`{m,}`/`{m,n}` (greedy or lazy), `^ $`, `\b \B`, and
  leading inline flags `(?i)`/`(?s)`.
- Compiler + **Pike VM** (`program`): Thompson-NFA simulation with leftmost-first
  priority — **O(pattern × input)**, immune to the catastrophic backtracking that
  afflicts naive engines on user-supplied `re:` patterns. `is_match` tracks only
  program counters (no per-thread capture vectors), so a pattern with many groups
  cannot blow up memory.
- **DoS hardening** (from security review): a parser nesting-depth limit (250)
  rejects `(((…)))` bombs instead of overflowing the stack; a `{m,n}` bound
  ceiling (100 000) plus in-loop compile-size checks reject `a{0,4000000000}`
  and `a{4000000000}` at parse time instead of allocating/spinning; the
  epsilon-closure is iterative (no recursion); and a compile-size cap bounds the
  program. Regression tests reproduce each attack pattern.
- Public API: `Regex::new`, `RegexBuilder` (`case_insensitive`,
  `dot_matches_new_line`), and `is_match`.

### Verified

- Cross-checked against the live `regex` crate (in `(?-u)` ASCII mode) across
  **100k+ randomly-generated (pattern, input) pairs** plus a case-insensitive
  sweep — zero `is_match` divergences. (`regex` is a dev-dependency for the gate
  only.)

### Not yet included

- Match **extents** (`find`/`captures`/`replace_all`): getting exact boundaries
  right for *nullable loops* (e.g. `(a?)*`) is a separate sub-problem, added in a
  later, separately-verified change. Engram needs extents only for one fixed
  pattern (the media-tag replacement).
- Unicode-aware classes (`\w`/`\d`/`\s` in Unicode mode, `\p{Alphabetic}` etc.):
  a follow-up adds generated Unicode tables. Today classes are ASCII.
