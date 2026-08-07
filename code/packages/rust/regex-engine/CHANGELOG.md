# Changelog — regex-engine

## 0.6.0 — Unreleased

Adds **`replace_all`** and the match **iterators** — the last engine capability
before engram-core can point its media-tag `replace_all` at this engine and drop
the third-party `regex` crate entirely (Phase D4).

### Added

- `Regex::replace_all(text, rep) -> Cow<str>` (borrows unchanged when there is no
  match). `rep` is a [`Replacer`]: a closure `FnMut(&Captures) -> String` (the form
  engram's media replacement uses), or a replacement **string** with `$N`/`${N}`
  numbered-group references and `$$` for a literal `$`.
- `Regex::find_iter` / `Regex::captures_iter` — iterators over the leftmost,
  **non-overlapping** matches. Iteration matches the `regex` crate: resume at the
  previous match's end, and skip an empty match sitting exactly at that end (so an
  empty-capable pattern inserts between characters without doubling at the seams).

### Verified

- Cross-checked vs the live `regex` crate: **84k** iteration checks (the sequence
  of match byte ranges agrees, incl. the empty-match non-overlap rule) and **84k**
  `replace_all` output comparisons — wherever the two engines iterate identically,
  the replaced output (with a `$0`/`$1`/`$$` replacement string) is byte-identical.
  5 new unit tests cover the closure and string replacers, empty-match semantics,
  and both iterators. Full `cargo test -p regex-engine` green; clippy + fmt clean.

## 0.5.0 — Unreleased

Adds **capture groups** — `Regex::captures` — the second half of Phase D3, on the
way to `replace_all` and dropping the `regex` crate from engram-core (D4).

### Added

- `Regex::captures(text) -> Option<Captures>` and the `Captures` type
  (`get(i)` → the `i`-th group's `Match` (0 = overall), `len`, `is_empty`).
  Group boundaries use the same leftmost-first Pike-VM priority as `find`;
  non-participating groups report `None`. Byte offsets throughout.
- Capturing groups now compile to `Save` instructions bracketing the body
  (slots `2g`/`2g+1`; slots `0`/`1` are the overall match). Threads on the
  `captures` run carry a **copy-on-write** (`Rc`) slot vector — branches share one
  allocation until a `Save` writes — so the common case stays cheap. `Save` is an
  epsilon no-op for `is_match`/`find`, so those paths are unchanged.

### Changed / guarded

- A pattern with more than **1000 capturing groups** is now rejected at build time
  (`MAX_GROUPS`) — a DoS guard on per-thread slot state, analogous to the existing
  nesting/repeat/program-size caps. Engram's media-tag regex has 3 groups.

### Verified

- Cross-checked against the live `regex` crate: **72k** existence checks (a match
  is reported iff `regex` matches) and **39k** full-group comparisons — wherever
  the two engines agree on the overall span, *every* capturing group's byte range
  agrees, across greedy/lazy quantifiers, alternation, nested groups, and multibyte
  input. (Group comparison is skipped only where `regex`'s own unanchored search
  reports a non-leftmost overall match — the corner already characterized for
  `find`.) 8 unit tests incl. non-participating branches, quantified-group
  last-iteration capture, the media-pattern shape, and the group-count reject.

## 0.4.0 — Unreleased

Adds the overall **match extent** — `Regex::find` — the first half of Phase D3
(match extents), on the way to `captures`/`replace_all` and dropping the `regex`
crate from engram-core entirely (D4).

### Added

- `Regex::find(text) -> Option<Match>` — the leftmost match's byte range and
  substring (`Match::start`/`end`/`as_str`/`range`). Leftmost-first with greedy
  quantifiers preferring more (the `regex` crate's default); reports **byte**
  offsets that index directly into the searched `&str`.

### Changed

- Star/plus compilation now yields the `regex` crate's extent for **nullable
  loops**. `e{n,}` (n ≥ 1) loops back to the body start; `e*` with a nullable body
  compiles as an optional-plus so an empty iteration routes to the loop exit at the
  correct priority (`(a?)*`/`(a*)*` on a run ⇒ the whole run; `(a??)*`/`(a??)+` ⇒
  the empty match — all matching `regex`). Purely a thread-priority change; the
  accepted language, and thus `is_match`, is unchanged.

### Verified

- `find` is checked against the live `regex` crate by its *defining properties*
  rather than a byte-identical span — on adversarial patterns the `regex` crate's
  own unanchored `find` returns non-leftmost results that its anchored matcher
  contradicts, so its `find` is the wrong oracle. Using its **anchored** matcher as
  an independent oracle, **40k+** random cases (greedy *and* lazy quantifiers,
  alternation, nested groups, nullable loops; multibyte inputs) confirm every
  reported span is a **valid** match at the **leftmost** start, and every `None`
  means no match anywhere. A separate **35k+** boolean cross-check confirms
  `is_match` agrees with `regex` across the same full construct space. Exact greedy
  extents — including the nullable-loop fixes (`(a?)*`⇒whole run, `(a??)*`⇒empty) —
  are pinned by 16 hand-verified unit tests.

## 0.3.1 — Unreleased

Adds the small `escape` helper engram-core's glob/search-pattern builder needs,
completing the API surface for Phase D2 (pointing engram-core's boolean search
at this engine).

### Added

- `regex_engine::escape(text) -> String` — escapes every regular-expression
  metacharacter so `text` matches literally. Mirrors `regex::escape` (same
  metacharacter set as `regex-syntax`), so an interleaved glob source built from
  escaped literals plus `*`/`_` wildcard fragments is byte-identical whether the
  old (`regex`) or new (`regex_engine`) path builds it. Unit-tested for
  metacharacter neutralization and literal round-trip.

## 0.3.0 — Unreleased

Adds **Unicode simple case folding** to `(?i)` matching — the last engine
capability needed before pointing engram-core's case-insensitive search
(`re:` / whole-word / glob) at this engine (Phase D2).

### Added

- `case_insensitive` / `(?i)` now uses **Unicode simple case folding** in Unicode
  mode (ASCII folding under `(?-u)`), matching the `regex` crate. Handles the
  tricky orbits a naive upper/lower closure misses — Greek `σ`/`ς`/`Σ`, the
  Kelvin (U+212A) and Ångström (U+212B) signs, long-s `ſ`, titlecase digraphs.
- Generated `casefold.rs` (1454 case-fold orbits over 2938 cased characters),
  produced once via the `regex` crate as oracle then frozen; orbit lookup is a
  binary search, and class folding checks each of the (small) orbit's members.

### Verified

- New cross-check vs the live `regex` crate's `(?i)` (Unicode) across **60k+
  random (pattern, input) pairs** built from cased characters incl. the tricky
  orbits — zero `is_match` divergences. The D0 ASCII and D1 Unicode cross-checks
  still pass. 13 unit tests; clippy + fmt clean.

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
