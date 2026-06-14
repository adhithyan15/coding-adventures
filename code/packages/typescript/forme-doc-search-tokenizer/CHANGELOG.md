# Changelog — @coding-adventures/forme-doc-search-tokenizer

## 0.1.0 — 2026-05-22

Initial release.  Eighth concrete DOC00 v0 package — text →
tokens pipeline for the documentation-site search index.
Lowercase, strip non-alphanumeric, split on whitespace,
optional stop-word filter, optional Porter stemmer.

Pure transform: `string → string[]`.  Capabilities `[]`.  **Zero
runtime dependencies.**

### Added

- `tokenize(text, options?): string[]` — main entry.  Runs the
  full pipeline (steps 1-5).
- `normaliseToTokens(text): string[]` — pipeline steps 1-3 only
  (lowercase, strip, split).  Standalone export for callers
  that want raw normalised tokens without filtering/stemming.
- `porterStem(word): string` — standalone Porter stemmer.
- `STOP_WORDS: ReadonlySet<string>` — built-in ~35-word
  English stop-word set.
- `TokenizeOptions` type:
  `{ filterStopWords?, stem?, customStopWords? }`.

### Spec adherence

Implements DOC00 v0's `forme-doc-search-tokenizer` per
`code/specs/DOC00-docs-vision.md` exactly:

> Text → tokens.  Pipeline:
> 1. Lowercase.
> 2. Strip punctuation (keep alphanumerics, drop everything else).
> 3. Split on whitespace.
> 4. Filter stop-words (optional — small built-in list).
> 5. (Optional) Porter stemmer — small, well-known algorithm.

All five steps implemented; steps 4 and 5 are opt-in via
options.

### Behavioural notes

- **Pure transform.**  Inputs (incl.
  `options.customStopWords` Set) are never mutated.  Verified
  by snapshot test.
- **Locale-independent.**  Uses `toLowerCase` (NOT
  `toLocaleLowerCase`) — search indexes must be stable across
  machines, and Turkish `'I' → 'ı'` would break cross-locale
  recall.
- **Underscore preserved.**  `setup_guide` / `__proto__` /
  `MY_CONSTANT` all stay as single tokens.  Splitting on
  underscores hurts code-block search recall.
- **Unicode-aware.**  `\p{L}` (any Unicode letter) and `\p{N}`
  (any Unicode number) are preserved.  `café`, `中文`,
  `Привет`, `Καλημέρα` all tokenise correctly.  Emoji and
  punctuation are stripped.
- **Deterministic.**  Same input bytes → identical output
  bytes.  Stable across machines.

### Porter stemmer

Faithful port of the canonical Martin Porter (1980) algorithm:
five sequential suffix-stripping steps (1a/1b/1b'/1c, 2, 3, 4,
5a/5b), each gated by Porter's "measure" m(W) — the number of
consonant-vowel transitions in W.

Coverage matches the published reference test vectors (lifted
from Porter's 1980 paper examples + the Snowball project's
English reference outputs).

Porter is well-documented English-only.  Non-ASCII tokens pass
through unchanged.

### Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver** —
  pure data construction.
- **No `+`-quantified regex on user input.**  Every scan in
  `normalise.ts` uses an explicit character-by-character index
  loop.  The only regex in the package is the single-char-class
  `/^[\p{L}\p{N}_]$/u` in `isTokenChar` (matches exactly one
  code point, no quantifier — not subject to polynomial-time
  concerns).  Background: previous DOC00 packages
  (`forme-doc-sidebar-builder`, `forme-doc-page-shell`) hit
  CodeQL's `js/polynomial-redos` query on simple
  `+`-quantified anchored regexes.  Explicit loops are now the
  project-wide standard for any user-facing trim/strip/scan.
- **No I/O** — capabilities `[]`.  Zero runtime dependencies.
- **No mutation** of inputs (verified).
- **Deterministic** — locale-independent case-folding,
  insertion-ordered Set iteration per ES spec, stable sort
  not needed (output is in source order).

### Tests

127 tests across 3 files:

- `normalise.test.ts` (23) — basic tokenisation, lowercasing
  (incl. tr-TR stability), punctuation stripping, hyphen/URL
  splitting, run collapsing, Unicode preservation
  (Latin/Chinese/Cyrillic/Greek), emoji stripping, edge cases
  (empty/punctuation-only/whitespace-only/single-char/trailing
  token/leading separator).
- `porter.test.ts` (85) — short-word passthrough, every step
  (1a, 1b/1b'/1c, 2, 3, 4, 5a/5b), common docs words
  (running/indexing/indexed/indexes), determinism.
- `tokenize.test.ts` (19) — defaults, stop-word filter
  (built-in/custom/empty), Porter stemming (alone, combined
  with filter, ordering), realistic queries and doc bodies,
  determinism, immutability, STOP_WORDS contents.

Coverage: **100% line / 95.6% branch / 100% function** on all
source files with logic (`types.ts` is type-only).  The
remaining branches are defensive length-guards that the public
entry `porterStem` already rejects upstream.

### v0 simplifications (documented)

- **English-only stemmer.**  Porter is English-only by
  definition.  Non-ASCII tokens pass through `porterStem`
  unchanged.  Multi-language sites disable stemming globally
  OR shard indexes by language.
- **No phrase tokens / n-grams.**  Each token is independent.
- **No synonyms.**  v1 may add a synonym-expansion option.
- **No language detection.**  v1 may add per-language pipeline
  routing.
- **No fuzzy matching.**  Strict exact-token match; typo
  tolerance is a query-time concern for
  `forme-doc-search-client-js`.
