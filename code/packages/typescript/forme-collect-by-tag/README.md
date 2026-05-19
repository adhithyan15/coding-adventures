# @coding-adventures/forme-collect-by-tag

Group an array of documents by tag for tag archive pages.  FM00
v0 §5.4 collector.

Pure transform: items + `tagsOf` accessor →
`Map<normalisedTag, items[]>` plus a sorted `tagNames` list.
Tags are normalised (lowercase + slugified) so case /
punctuation variants collide into a single bucket.

Ninth FM00 v0 stage package — joins the FM00 v0 cluster.

## Quick start

```ts
import { collectByTag } from "@coding-adventures/forme-collect-by-tag";

const { byTag, tagNames } = collectByTag(posts, {
  tagsOf: (p) => p.tags,
  sortBy: (a, b) => b.pubDate.localeCompare(a.pubDate),  // newest first
  includeUntagged: true,
});

for (const tag of tagNames) {
  const items = byTag.get(tag)!;
  renderTagArchive(tag, items);
}
```

## Why a separate collector?

Authors are inconsistent — `TypeScript`, `typescript`,
`type-script`, `Type Script` should all collide.  This package
normalises tags through a single-pass slugifier (lowercase,
strip non-`[a-z0-9 -]`, collapse runs, trim) so the resulting
tag cloud has one bucket per concept rather than per spelling.

It's a generic helper rather than a stage-framework collector
because the same operation is useful in many shapes — site-
level tag archives, intra-page filtering widgets, "related
posts" sidebars.  Pair with the orchestrator if you want a
proper Stage.

## API

### `collectByTag(items, options): { byTag, tagNames }`

Generic over `T`.  Options:

```ts
interface CollectByTagOptions<T> {
  readonly tagsOf: (item: T) => readonly string[] | undefined | null;
  readonly sortBy?: (a: T, b: T) => number;
  readonly includeUntagged?: boolean;        // default false
  readonly untaggedBucketName?: string;      // default "untagged"
}
```

Returns:

```ts
interface CollectByTagResult<T> {
  readonly byTag: ReadonlyMap<string, readonly T[]>;
  readonly tagNames: readonly string[];      // alphabetically sorted
}
```

### `normaliseTag(tag): string`

Exposed sub-helper.  Applies the same slugification rules used
internally:

- Lowercase
- Strip ASCII control bytes
- Drop everything outside `[a-z0-9 -]` (punctuation, non-ASCII,
  control chars)
- Collapse whitespace + hyphen runs to single `-`
- Trim leading/trailing `-`
- Empty input or all-stripped input → empty string

Useful for callers building related tooling (e.g. resolving a
URL slug back to a tag name).

## Behavioural contract

| Aspect                       | Behaviour                              |
|------------------------------|----------------------------------------|
| Input items array            | Never mutated                          |
| Item tag arrays              | Never mutated                          |
| Output buckets               | Fresh arrays per call                  |
| Output map storage           | `Map` (not plain `Object`) — proto-safe |
| Tag normalisation            | GitHub-style slug rules                |
| Per-item dedup               | Same normalised key inserts item once  |
| Tags that normalise to empty | Silently dropped (no synthetic bucket) |
| Untagged items               | Drop by default; opt-in bucket via `includeUntagged` |
| Within-bucket order          | Input order; `sortBy` overrides        |
| `byTag` iteration order      | First-seen-bucket order                |
| `tagNames` order             | Alphabetically sorted                  |

## Reproducibility (FM03)

Same `items` + same `tagsOf` + same `sortBy` → byte-identical
output.  `Map` insertion order is preserved by V8; `tagNames`
is a deterministic alphabetical view.

## Security posture

Three concerns explicitly addressed (pre-push review):

- **Prototype pollution via attacker tag names.**  `Map` /
  `Set` used throughout — never plain-Object property
  assignment.  Tag `"__proto__"` becomes the bucket key
  `"proto"` (underscores stripped by `normaliseTag`); even an
  attacker who bypasses the normaliser cannot pollute
  `Object.prototype` because `Map` keys live in an internal
  slot.  Test pins both vectors.
- **HTML metacharacter injection.**  `normaliseTag` strips
  everything except `[a-z0-9 -]`, so bucket keys are safe to
  interpolate into archive page slugs / URLs without escaping.
  `<script>` becomes `script`; `<` / `>` / `&` / quotes never
  survive.
- **No regex / ReDoS.**  Normalisation uses a single
  forward `for` loop with `charCodeAt` — same pattern as
  `forme-transform-typography` and the rewritten
  `forme-transform-autolink-headings`.  Zero regex
  backtracking surface.

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell, no env, no fs.

## Tests

53 tests across 2 files:

- `normalise.test.ts` (23) — basic shape (lowercase, hyphen
  replacement, run collapse, trim, digit preservation,
  idempotence); security (angle brackets / quotes / ampersand /
  control bytes / underscores → `__proto__` becomes `proto` /
  non-ASCII / path-traversal sequences); empty / collapsed
  input fallback; defensive coercion of non-string inputs;
  deterministic.
- `collect.test.ts` (30) — basic grouping (Map by normalised
  key, variants merge, untagged dropped by default, items
  multi-appear per tag), sorted `tagNames` matches `byTag.size`,
  within-bucket sort (default input-order, sortBy newest-first,
  applied to every bucket), untagged bucket policy (off by
  default, `includeUntagged: true`, empty tags array, all-tags-
  normalise-to-empty, custom bucket name, null tags), per-item
  dedup (same-normalised-key inserts once; different-normalised
  variants get separate buckets), prototype-pollution defence
  (`__proto__` tag and synthetic attacker key both safe),
  purity / determinism (no input mutation, no item tag-array
  mutation, byte-identical output, fresh bucket arrays per call,
  `byTag` first-seen vs `tagNames` alphabetical, empty input),
  tags-normalising-to-empty (drop by default, route to untagged
  if `includeUntagged`).

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only declarations).

## Spec adherence

Implements FM00 v0 §5.4 `collect-by-tag`.  Generic over the
item type rather than tied to a specific `Document` shape;
matches the spec intent while keeping the package usable across
the FM00 stage cluster.  No spec divergences.

## v0 simplifications

- **No tag hierarchy.**  `foo/bar` and `foo/baz` are two
  unrelated buckets.  Hierarchical tag taxonomies (where `foo/bar`
  rolls up into `foo`) would need a hierarchy spec — deferred.
- **No multi-tag intersection / union queries.**  Caller
  composes themselves: `[...byTag.get("typescript")!].filter(p =>
  byTag.get("react")!.includes(p))`.
- **No tag-count summaries.**  Caller computes
  `byTag.get(tag)!.length` themselves; we don't pre-materialise
  a `Map<tag, count>`.
- **No "popular tags" cap.**  Caller slices `tagNames` themselves.
- **No locale-aware case folding.**  Always uses
  `String.prototype.toLowerCase()` — Turkish-İ kind of
  surprises pass through.  Matches the pattern used by the
  rest of the FM00 v0 cluster's slugifiers.
