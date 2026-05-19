# Changelog — @coding-adventures/forme-collect-by-tag

## 0.1.0 — 2026-05-19

Initial release.  Ninth FM00 v0 stage package — second concrete
§5.4 collector (joins `forme-collect-chronological`).  Groups an
array of documents by tag for tag archive pages.

Pure transform: items + `tagsOf` accessor →
`Map<normalisedTag, items[]>` plus a sorted `tagNames` list.

### Added

- `collectByTag<T>(items, options): { byTag, tagNames }` —
  main entry.  Generic over the item type; only constraint is
  caller provides a `tagsOf` accessor.
- `normaliseTag(tag): string` — exposed sub-helper for callers
  building related tooling (URL slug → tag name resolution,
  filter input sanitation, etc.).
- `CollectByTagOptions<T>`, `CollectByTagResult<T>`, `TagsOf<T>`
  types.

### Spec adherence

Implements FM00 v0 §5.4 `collect-by-tag`.  No spec divergences.
Generic over the item type (rather than tied to a specific
`Document` shape) so the helper works across the FM00 cluster.

### Behavioural notes

- **Tag normalisation.**  `TypeScript`, `typescript`,
  `type-script`, `Type Script` all collide into the same bucket
  via single-pass slugification (lowercase, strip non-
  `[a-z0-9 -]`, collapse runs, trim).
- **`Map`-based grouping.**  Bucket storage uses `Map<string,
  T[]>`, not a plain `Object` — protects against `__proto__`
  pollution from attacker tag names.  Normaliser would already
  strip underscores (`__proto__` → `proto`) but the Map
  defence is independent of normaliser behaviour.
- **Per-item dedup.**  An item with
  `tags: ["TypeScript", "typescript"]` lands in the
  `"typescript"` bucket exactly once (not twice).  Done via a
  per-item `Set<string>` of normalised keys.
- **Tags normalising to empty are dropped.**  A tag like
  `"@@@"` or `"日本語"` strips to empty; the collector silently
  ignores it rather than synthesising a phantom bucket.  An
  item whose tags ALL normalise to empty is treated as
  untagged.
- **Untagged policy.**  Default `includeUntagged: false` — items
  with no tags vanish from the output.  Opt-in `true` routes
  them into a bucket whose name defaults to `"untagged"` but
  can be overridden via `untaggedBucketName`.
- **`tagsOf` null/undefined/[] all treated as untagged.**
  Defensive — caller's frontmatter parsing might produce any of
  these for "no tags declared".
- **Within-bucket sort.**  Default is input order.  Supply a
  `sortBy` comparator to apply per-bucket sorting (e.g.
  newest-first).
- **Two output orderings.**  `byTag` iterates in first-seen-
  bucket order (matches input traversal); `tagNames` is a
  separate alphabetically-sorted array (deterministic across
  reruns regardless of input shuffle).  Either alone would
  force callers to redo work.

### Security posture

Three concerns explicitly addressed (pre-push review):

- **Prototype pollution.**  `Map` / `Set` used throughout —
  never plain-Object property assignment.  Tag `"__proto__"`
  becomes bucket key `"proto"` (underscores stripped); even an
  attacker bypassing the normaliser cannot pollute
  `Object.prototype` because `Map` keys live in an internal
  slot.  Two test cases pin this — normalised path AND
  synthetic attacker-key path.
- **HTML metacharacter injection.**  `normaliseTag` strips
  everything except `[a-z0-9 -]`; bucket keys are safe to
  interpolate into archive URLs / HTML attributes without
  escaping.  `<script>` → `script`; `<` / `>` / `&` / quotes
  never survive.  Path-traversal sequences (`../../`) also
  stripped clean.
- **No regex / ReDoS.**  Normalisation uses a single forward
  `for` loop with `charCodeAt` — same pattern as
  `forme-transform-typography` and the rewritten
  `forme-transform-autolink-headings`.  Zero regex
  backtracking surface; trivially passes CodeQL.

### Capabilities

`[]` — pure transform.  No I/O, network, fs, shell, env.

### Tests

53 tests across 2 files:

- `normalise.test.ts` (23) — basic shape (lowercase, hyphen
  replacement, run collapse, trim, digit preservation,
  idempotence); security (angle brackets, quotes, ampersand,
  control bytes, underscores → `__proto__` becomes `proto`,
  non-ASCII CJK/emoji → empty, path-traversal sequences);
  output always matches `/^[a-z0-9-]*$/`; empty / whitespace /
  punctuation-only / non-ASCII-only → empty; defensive
  non-string coercion; deterministic.
- `collect.test.ts` (30) — basic grouping (Map by normalised
  key, case/punctuation variants merge, untagged dropped by
  default, items appear in every tag bucket they qualify for);
  sorted `tagNames` matches `byTag.size`; within-bucket sort
  (default input-order, sortBy newest-first, applied to every
  bucket); untagged policy (off by default, opt-in with various
  trigger shapes — no tags field, empty tags array, all-tags-
  normalise-to-empty, custom bucket name, null `tagsOf`
  return); per-item dedup (same-normalised-key inserts once;
  different-normalised variants get separate buckets);
  prototype-pollution defence (`__proto__` tag AND synthetic
  attacker key); purity / determinism (no input mutation, no
  item tag-array mutation, byte-identical output across calls,
  fresh bucket arrays per call, `byTag` first-seen vs
  `tagNames` alphabetical, empty input); tags-normalising-to-
  empty drop policy (silently dropped if untagged off; routed
  to untagged bucket if on).

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only declarations).

### v0 simplifications (documented)

- **No tag hierarchy.**  `foo/bar` and `foo/baz` are unrelated
  buckets.  Hierarchical taxonomies deferred to a future spec.
- **No multi-tag intersection / union queries.**  Caller
  composes themselves.
- **No tag-count summaries / popular-tags cap.**  Caller
  computes `byTag.get(tag)!.length` and slices `tagNames`
  themselves.
- **No locale-aware case folding.**  Always uses
  `String.prototype.toLowerCase()` — Turkish-İ surprises pass
  through.  Matches the rest of the FM00 v0 cluster.
