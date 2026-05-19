# Changelog — @coding-adventures/forme-collect-by-author

## 0.1.0 — 2026-05-19

Initial release.  Tenth FM00 v0 stage package — third concrete
§5.4 collector (joins `forme-collect-chronological` and
`forme-collect-by-tag`).  Groups documents by author for author
archive pages.

Mirrors `forme-collect-by-tag`'s shape but the `authorOf`
accessor accepts `string | string[]` so co-author posts cleanly
land in every contributor's bucket.

### Added

- `collectByAuthor<T>(items, options): { byAuthor, authorNames }`
  — main entry.  Generic over the item type; only constraint is
  caller-supplied `authorOf` accessor.
- `normaliseAuthor(name): string` — exposed sub-helper.
- `CollectByAuthorOptions<T>`, `CollectByAuthorResult<T>`,
  `AuthorOf<T>` types.

### Spec adherence

Implements FM00 v0 §5.4 `collect-by-author`.  No spec
divergences.

### Behavioural notes

- **`authorOf` shape.**  Returns `string | string[] | null |
  undefined`.  `null` / `undefined` / `""` / `[]` are all
  treated as anonymous.
- **Co-author splitting.**  A post with
  `authors: ["Ada", "Charles"]` appears in BOTH the `ada` and
  `charles` buckets.  Same-author posts across single-author
  and co-author shapes collide into one bucket via
  normalisation.
- **Per-item dedup.**  `["Ada", "ada", "ADA"]` inserts the
  item into the `ada` bucket exactly once.  Done via a per-
  item `Set<string>` of normalised keys.
- **Hostile array elements defensively dropped.**  Non-string
  entries (numbers, `null`, `undefined`, objects) and empty
  strings in a co-author array are silently dropped — never
  reach `normaliseAuthor`, never become bucket keys.
- **Normalisation.**  Same single-pass `charCodeAt` slugifier
  as `forme-collect-by-tag` and
  `forme-transform-autolink-headings`.  Lowercase + strip
  `[^a-z0-9 -]` + collapse runs + trim.  Output matches
  `/^[a-z0-9-]*$/`.
- **Anonymous policy.**  Default `includeAnonymous: false`.
  Opt-in routes anonymous items to a bucket whose name
  defaults to `"anonymous"` (overridable via
  `anonymousBucketName`).
- **Two output orderings.**  `byAuthor` Map iterates in
  first-seen-bucket order; `authorNames` is alphabetically
  sorted.  Both exposed because either alone forces callers to
  redo work.

### Security posture

Four concerns explicitly addressed (pre-push review):

- **Prototype pollution.**  `Map` / `Set` used throughout —
  never plain-Object property assignment.  Tag `__proto__`
  becomes bucket key `"proto"` (underscores stripped); even an
  attacker bypassing the normaliser cannot pollute
  `Object.prototype` because `Map` keys live in an internal
  slot.  Two test cases pin this.
- **HTML metacharacter injection.**  `normaliseAuthor` strips
  everything except `[a-z0-9 -]`; bucket keys are safe to
  interpolate into archive URLs / HTML attributes without
  escaping.
- **Hostile co-author array entries.**  Non-string entries
  (`null`, `undefined`, numbers, objects) dropped before
  normalisation — they never become bucket keys.  Test pinned.
- **No regex / ReDoS.**  Single-pass `charCodeAt` loop —
  zero backtracking surface.

### Capabilities

`[]` — pure transform.  No I/O, network, fs, shell, env.

### Tests

56 tests across 2 files:

- `normalise.test.ts` (22) — basic shape (lowercase, hyphen
  replacement, run collapse, trim, digit preservation,
  idempotence); security (angle brackets, quotes, ampersand,
  control bytes, underscores → `__proto__` becomes `proto`,
  non-ASCII CJK / emoji → empty, path-traversal sequences);
  output always matches `/^[a-z0-9-]*$/`; empty / whitespace /
  punctuation-only / non-ASCII-only fallback; defensive
  non-string coercion; deterministic.
- `collect.test.ts` (34) — basic grouping (Map by normalised
  key, case variants merge, anonymous dropped by default,
  co-author items in every bucket); string vs array accessor
  (single-string author handled, single-element array, co-
  author split into multiple buckets, non-string array
  elements dropped defensively including `null`/`undefined`/
  numbers, empty strings dropped); sorted `authorNames` parity;
  within-bucket sort (default input-order, sortBy newest-first,
  applied to every bucket); anonymous policy (off by default,
  opt-in via `null`/`undefined`/all-stripped, custom name,
  explicit off); per-item dedup (same-key inserts once,
  different-key separate buckets); prototype-pollution defence
  (`__proto__` tag AND synthetic bypass, `constructor`);
  purity / determinism (no input mutation, no co-author array
  mutation, byte-identical output, fresh buckets, `byAuthor`
  first-seen vs `authorNames` alphabetical, empty input);
  authors-normalising-to-empty drop policy.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only declarations).

### v0 simplifications (documented)

- **No author identity reconciliation.**  `Ada Lovelace` and
  `Ada Augusta King, Countess of Lovelace` are unrelated
  buckets.  Reconciliation belongs at the manifest layer.
- **No primary-vs-secondary author distinction.**  Co-authors
  are equal; the item appears in every bucket equally.
- **No author-count summaries / popular-authors cap.**  Caller
  computes / slices themselves.
- **No locale-aware case folding.**  Always
  `String.prototype.toLowerCase()`.
