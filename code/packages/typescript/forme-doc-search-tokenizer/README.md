# @coding-adventures/forme-doc-search-tokenizer

> Eighth DOC00 v0 package — text → tokens pipeline for the
> documentation-site search index. Lowercase, strip
> non-alphanumeric, split on whitespace, optional stop-word
> filter, optional Porter stemmer.

Pure transform. Capabilities: `[]`. **Zero runtime dependencies.**

## What it does

```ts
import { tokenize } from "@coding-adventures/forme-doc-search-tokenizer";

tokenize("Hello, World!");
// → ["hello", "world"]

tokenize("the quick brown fox", { filterStopWords: true });
// → ["quick", "brown", "fox"]      (drops "the")

tokenize("running and walking", { stem: true });
// → ["run", "and", "walk"]

tokenize("running and walking", { filterStopWords: true, stem: true });
// → ["run", "walk"]
```

## Pipeline

| Step                       | Default                       |
|----------------------------|-------------------------------|
| 1. Lowercase               | always (locale-independent)   |
| 2. Strip non-alphanumeric  | always (keep `\p{L}` `\p{N}` `_`) |
| 3. Split on whitespace     | always                        |
| 4. Stop-word filter        | opt-in (`filterStopWords: true`) |
| 5. Porter stemmer          | opt-in (`stem: true`)         |

**Locale-independent lowercasing** uses `toLowerCase` (NOT
`toLocaleLowerCase`) — same reasoning as
`forme-doc-heading-anchors`. Under tr-TR, `'I' → 'ı'`, which
would break cross-locale search recall. Default Unicode
case-folding is the right call for stable indexes.

**Underscore preserved inside tokens** so identifier-shaped
strings like `setup_guide` and `__proto__` survive intact. Code
blocks in docs often contain identifiers, and splitting them
hurts search recall.

## Public API

| Export                   | Purpose                                                                  |
|--------------------------|--------------------------------------------------------------------------|
| `tokenize(text, opts?)`  | Main entry — runs the full pipeline.                                     |
| `normaliseToTokens(text)`| Pipeline steps 1-3 only (lowercase, strip, split). No filtering/stemming.|
| `porterStem(word)`       | Standalone Porter stemmer for a single word.                             |
| `STOP_WORDS`             | Built-in ~35-word English stop-word set.                                 |
| `TokenizeOptions`        | `{ filterStopWords?, stem?, customStopWords? }`.                         |

## Stop-word list

Curated ~35-word set, deliberately small for technical
documentation context. Includes high-frequency function words
(articles, pronouns, common prepositions, copulas) but
**keeps** negation/question words (`not`, `no`, `how`, `what`,
`when`, `why`, `where`) since they carry meaningful search
signal in API descriptions and FAQ queries.

Override the built-in list with `options.customStopWords` if
your domain calls for it (e.g. drop common framework-specific
boilerplate like `function`, `class`, `import` for
JavaScript-only docs).

## Porter stemmer

The textbook Martin Porter (1980) English stemmer — five
sequential suffix-stripping steps, each gated by Porter's
"measure" m(W) (number of consonant-vowel transitions).
Implementation faithfully follows the published reference;
deviation produces non-portable stems.

Examples:
- `running` / `runs` → `run`
- `happy` / `happiness` / `happily` → `happi`
- `relational` → `relat` (`-ational` → `-ate`, then `-ate` step 4)
- `airliner` → `airlin` (`-er` step 4)
- `replacement` → `replac` (`-ment` step 4)

Stemming is **optional** because:
1. It hurts recall for exact-keyword queries (`"running" != "run"` for case-sensitive code searches).
2. It increases index complexity (the index-builder must agree with the query-tokeniser).
3. For short docs sites with English vocabulary, the stem-collapse benefit is modest.

When you DO enable it, both the index-builder
(`forme-doc-search-index-builder`) and the query tokeniser
(`forme-doc-search-client-js`) must use `stem: true` — or
queries won't match the index.

## Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver.**
  Pure data construction.
- **No `+` quantifiers on user-controllable regex.** Every
  scan in `normalise.ts` uses an explicit character-by-character
  index loop. The only regex in the package is the
  single-char-class `/^[\p{L}\p{N}_]$/u` in `isTokenChar`,
  which matches exactly one code point (no quantifier) and is
  therefore not subject to polynomial-time concerns.
  - Background: previous DOC00 packages (`forme-doc-sidebar-builder`,
    `forme-doc-page-shell`) hit CodeQL's `js/polynomial-redos`
    query on simple `+`-quantified anchored regexes. The
    explicit-loop approach is now the project-wide standard
    for any user-facing trim/strip/scan.
- **No I/O.** Capabilities `[]`. Zero runtime dependencies.
- **Deterministic.** Same input bytes → identical output bytes.
  Stable across machines (locale-independent case-folding,
  insertion-ordered Set iteration per ES spec).
- **No input mutation.** Verified by test (custom stop-word
  set snapshot).

## Tests

127 tests across three files:

- `normalise.test.ts` (23) — basic tokenisation, lowercasing
  (including tr-TR stability), punctuation stripping, hyphen
  splitting, URL splitting, run collapsing, Unicode preservation
  (Latin/Chinese/Cyrillic/Greek), emoji stripping, edge cases
  (empty/punctuation-only/whitespace-only/single-char/trailing
  token/leading separator).
- `porter.test.ts` (85) — short-word passthrough, every step
  (1a plurals, 1b past participles + -ing, 1b' post-strip
  adjustments, 1c -y → -i, 2 -ational/-tional/etc., 3
  -icate/-ative/-alize, 4 -al/-ance/-ence/-er/etc. + -ion
  after s/t, 5a -e cleanup, 5b -ll → -l), common docs words
  (running, indexing, indexed, indexes), determinism.
- `tokenize.test.ts` (19) — defaults, stop-word filter
  (built-in / custom / empty), Porter stemming (alone,
  combined with filter, ordering), realistic queries and
  doc bodies, determinism, immutability, STOP_WORDS contents.

Coverage: **100% line / 95.6% branch / 100% function** on all
source files with logic (`types.ts` is type-only). The
remaining branches are defensive guards for word lengths the
public entry (`porterStem`) already rejects.

## How it fits in the stack

Eighth concrete DOC00 v0 package. Sits in the search-engine
layer, used by both the build-time index builder and the
runtime query tokeniser:

```
.md bodies + frontmatter  ──►  search-tokenizer (build time)
                                       ↓
                              search-index-builder  ──►  index shards
                                                              ↓
.md body in browser  ──►  search-tokenizer (runtime)        / disk
        ↓                          ↓                          ↓
   user query  ──►   search-client-js  ◄──  loaded shards    ↓
        ↓                          ↓                          ↓
        └──────── matched documents ranked + displayed
```

Next DOC00 v0 packages: `forme-doc-search-index-builder`,
`forme-doc-search-client-js`, `forme-doc-site-emitter`.

## v0 simplifications (documented)

- **English-only.** Porter stemmer is documented English-only.
  Non-ASCII tokens (`café`, `中文`) pass through `porterStem`
  unchanged. Multi-language sites should disable stemming
  globally OR shard their indexes by language.
- **No phrase tokens.** Each token is independent. "machine
  learning" tokenises to `["machine", "learning"]` with no
  bigram tracking. v1 may add n-gram support for phrase
  queries.
- **No synonyms.** "color" and "colour" are different tokens.
  Sites with regional vocabulary should pre-normalise.
- **No language detection.** `tokenize` runs the same pipeline
  on all input. Multi-language sites must route through
  per-language tokenisers themselves.
- **No fuzzy matching.** Strict exact-token match. Typo
  tolerance (Levenshtein distance, etc.) is a query-time
  concern for `forme-doc-search-client-js`.
