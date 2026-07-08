# Changelog — regex-engine

## 0.2.0 — Unreleased

Phase D1 of the Engram zero-dependency program: **Unicode-aware character
classes**, so the engine matches the `regex` crate in its default (Unicode) mode.

### Added

- Unicode `\d \w \s` (and negations) — on by default, matching `regex`; `(?-u)`
  selects the ASCII sets.
- Unicode general-property classes `\p{Alphabetic}`, `\p{Mark}` (a.k.a. `\p{M}`),
  `\p{Nd}`, and their `\P{…}` negations — usable standalone and inside `[...]`.
- `\b` word boundaries use the Unicode word set in Unicode mode.
- Inline `(?u)` / `(?-u)` flag toggling (including in combined groups like `(?i-u)`).
- Generated `unicode_tables.rs` (WORD/DIGIT/SPACE/ALPHABETIC/MARK/ND ranges),
  produced once from the `regex` crate then frozen; class membership is by binary
  search over sorted, merged ranges.

### Verified

- Cross-checked against the live `regex` crate in **default Unicode mode** (no
  `(?-u)`) across **80k+ random (pattern, input) pairs** using Unicode classes and
  non-ASCII inputs (accented Latin, combining marks, CJK, Greek, Arabic-Indic /
  Devanagari digits, non-breaking space) — zero `is_match` divergences. The
  ASCII-mode cross-check from D0 still passes.

### Not yet included

- **Unicode case folding**: `case_insensitive` still folds ASCII only. Non-ASCII
  case pairs (e.g. `é`/`É`) are added when the case-insensitive search uses are
  wired (Phase D2). Match **extents** (`find`/`captures`/`replace_all`) remain
  deferred to D3.

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
