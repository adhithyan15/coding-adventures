# @coding-adventures/forme-collect-by-author

Group an array of documents by author for author archive pages.
FM00 v0 §5.4 collector.

Pure transform: items + `authorOf` accessor (returns
`string | string[] | null | undefined`) → `Map<normalisedAuthor,
items[]>` plus a sorted `authorNames` list.  Mirrors
[`forme-collect-by-tag`](../forme-collect-by-tag)'s shape but
accepts co-author arrays so posts with multiple contributors
land in every author's bucket.

Tenth FM00 v0 stage package — joins the FM00 v0 cluster.

## Quick start

```ts
import { collectByAuthor } from "@coding-adventures/forme-collect-by-author";

const { byAuthor, authorNames } = collectByAuthor(posts, {
  authorOf: (p) => p.author ?? p.authors,
  sortBy: (a, b) => b.pubDate.localeCompare(a.pubDate),  // newest first
  includeAnonymous: true,
});

for (const author of authorNames) {
  const items = byAuthor.get(author)!;
  renderAuthorArchive(author, items);
}
```

## Single author vs co-authors

The `authorOf` accessor returns either form:

```ts
authorOf: (p) => p.author              // "Ada Lovelace"
authorOf: (p) => p.authors             // ["Ada", "Charles"]
authorOf: (p) => p.author ?? p.authors // mix of both shapes in your data
```

A post with `authors: ["Ada", "Charles"]` appears in BOTH the
`ada` and `charles` buckets.  Same author across co-author and
single-author posts collide into one bucket — same case /
punctuation normalisation as
[`forme-collect-by-tag`](../forme-collect-by-tag).

## Why a separate collector?

Same reason `forme-collect-by-tag` exists separately from
`forme-transforms.groupBy`: author archive pages are a common
enough site feature that they deserve a typed, validated,
security-hardened helper.  Splitting authors from tags avoids
forcing one shape onto callers — many posts have one author but
many tags; treating them with the same accessor would make the
API noisy.

## API

### `collectByAuthor(items, options): { byAuthor, authorNames }`

Generic over `T`.  Options:

```ts
interface CollectByAuthorOptions<T> {
  readonly authorOf: (item: T) => string | readonly string[] | null | undefined;
  readonly sortBy?: (a: T, b: T) => number;
  readonly includeAnonymous?: boolean;        // default false
  readonly anonymousBucketName?: string;      // default "anonymous"
}
```

Returns:

```ts
interface CollectByAuthorResult<T> {
  readonly byAuthor: ReadonlyMap<string, readonly T[]>;
  readonly authorNames: readonly string[];    // alphabetically sorted
}
```

### `normaliseAuthor(name): string`

Exposed sub-helper.  Same single-pass character-loop slugifier
used in `forme-collect-by-tag` and
`forme-transform-autolink-headings`.

- Lowercase
- Strip ASCII control bytes
- Drop everything outside `[a-z0-9 -]`
- Collapse whitespace + hyphen runs
- Trim leading/trailing hyphens
- Empty input or all-stripped → empty string

Output guarantees: matches `/^[a-z0-9-]*$/`; safe to
interpolate into HTML attributes / URL slugs without escaping.

## Behavioural contract

| Aspect                          | Behaviour                              |
|---------------------------------|----------------------------------------|
| Input items array               | Never mutated                          |
| Co-author arrays                | Never mutated                          |
| Output buckets                  | Fresh arrays per call                  |
| Output map storage              | `Map` (not plain `Object`)             |
| Single-string author            | Single bucket                          |
| Co-author array                 | One bucket per author; item in each    |
| Non-string array elements       | Dropped defensively                    |
| Empty string in array           | Dropped                                |
| Per-item dedup                  | Same normalised key inserts item once  |
| Authors normalising to empty    | Silently dropped                       |
| `null` / `undefined` / `""` / `[]` author | Treated as anonymous       |
| Anonymous items                 | Drop by default; opt-in via `includeAnonymous` |
| Within-bucket order             | Input order; `sortBy` overrides        |
| `byAuthor` iteration order      | First-seen-bucket order                |
| `authorNames` order             | Alphabetically sorted                  |

## Reproducibility (FM03)

Same `items` + same `authorOf` + same `sortBy` → byte-identical
output.

## Security posture

Four concerns explicitly addressed (pre-push review):

- **Prototype pollution via attacker author names.**  `Map` /
  `Set` used throughout — never plain-Object property
  assignment.  `__proto__` becomes `"proto"` (underscores
  stripped); even an attacker bypassing the normaliser cannot
  pollute `Object.prototype` because `Map` keys live in an
  internal slot.  Test pins both vectors.
- **HTML metacharacter injection.**  `normaliseAuthor` strips
  everything except `[a-z0-9 -]`; bucket keys are safe to
  interpolate into archive URLs / HTML attributes without
  escaping.
- **Co-author array hostile inputs.**  Non-string entries
  (`null`, `undefined`, numbers, objects) and empty strings in
  a co-author array are silently dropped — they never reach
  `normaliseAuthor`, never become bucket keys.  Pinned by the
  "non-string array elements" test.
- **No regex / ReDoS.**  Single-pass `charCodeAt` loop —
  trivially passes CodeQL polynomial-regex analysis.

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell, no env, no fs.

## Tests

56 tests across 2 files:

- `normalise.test.ts` (22) — basic shape, security (angle
  brackets, quotes, ampersand, control bytes, underscores
  including `__proto__`, non-ASCII CJK/emoji, path-traversal),
  output regex guarantee, empty/whitespace/punctuation-only
  fallback, defensive non-string coercion, deterministic.
- `collect.test.ts` (34) — basic grouping (Map by normalised
  key, case variants merge, anonymous dropped by default,
  co-author items in every bucket), string vs array accessor
  (single-string, single-element array, co-author split,
  non-string array elements dropped, empty strings dropped),
  sorted `authorNames`, within-bucket sort, anonymous bucket
  policy (off by default, opt-in via `null`/`undefined`/empty/
  all-stripped, custom name, explicit off), per-item dedup
  (same-key inserts once, different-key separate buckets),
  prototype-pollution defence (normalised + bypass paths,
  `constructor`), purity / determinism (no input mutation, no
  co-author array mutation, byte-identical output, fresh
  buckets, first-seen vs alphabetical, empty input),
  authors-normalising-to-empty drop policy.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only).

## Spec adherence

Implements FM00 v0 §5.4 `collect-by-author`.  No spec
divergences.

## v0 simplifications

- **No author identity reconciliation.**  Different spellings
  of the same author (`Ada Lovelace` vs `Ada Augusta King,
  Countess of Lovelace`) get separate buckets.  Reconciliation
  is a manifest-level concern, not a collector concern.
- **No primary-vs-secondary author distinction.**  Co-authors
  are equal; the item appears in every bucket equally.  A
  future v1 might expose a `primaryAuthor` field separately.
- **No author-count summaries / popular-authors cap.**  Caller
  computes `byAuthor.get(author)!.length` and slices
  `authorNames` themselves.
- **No locale-aware case folding.**  Always `String.prototype.
  toLowerCase()`.  Matches the rest of the FM00 v0 cluster.
