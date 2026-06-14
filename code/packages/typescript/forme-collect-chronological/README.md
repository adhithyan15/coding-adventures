# @coding-adventures/forme-collect-chronological

Forme collector stage: `Stream<ContentNode>` → single `Collection`,
sorted chronologically (newest first) with a derived URL route on every
entry.

Third Forme stage of the blog v0 effort. Sits between
`forme-parse-markdown` (which produces a `Stream<ContentNode>`, one per
file) and the renderer stages (which walk the collection to emit pages
and an index).

## Stage shape

```ts
import collect from "@coding-adventures/forme-collect-chronological";

collect.consumes      // streamOf(Kinds.ContentNode)
collect.produces      // Kinds.Collection
collect.capabilities  // []  ← pure transform
collect.configSchema  // { name?, dateField?, slugField?, routeTemplate? }
```

## What it does

1. **Buffer** the incoming stream (a collector needs every node before it
   can sort).
2. For each `ContentNode`, derive `{ dateStr, slug, route }`:
   - `dateStr` = `frontmatter[dateField]` (default key `"date"`), or
     the sentinel `"0000-01-01"` if absent — a warning is logged.
   - `slug` = `frontmatter[slugField]` (default key `"slug"`) if a
     non-empty string, else `slugify(sourcePath)`.
   - `route` = `routeTemplate` with `{slug}` substituted
     (default template `"/blog/{slug}.html"`).
3. **Sort** by `dateStr` descending; tie-break by `sourcePath`
   ascending (deterministic across runs).
4. **Emit** one `Collection`:
   ```ts
   {
     name: config.name ?? "posts",
     entries: CollectionEntry[],   // identity + revision + route + orderKey + overlay
     discriminant: "chronological",
     meta: {},
   }
   ```

## Entry overlay

Each `CollectionEntry.overlay` carries the values the index renderer
needs to display a card:

- `date` — the resolved date string (sentinel when missing)
- `slug` — the resolved slug
- `title` — `frontmatter.title` if present, else the slug
- `excerpt` — `frontmatter.excerpt` if present (omitted when absent)

The entry stores `identity` + `revision` references back to the
`ContentNode` rather than the node itself — collections of millions of
entries must stay cheap to construct, hash, and diff (FM00 §5.4).

## Date format convention

Use **ISO-8601** (`YYYY-MM-DD` or `YYYY-MM-DDTHH:MM:SSZ`). The collector
compares dates as strings, so any fixed-width prefix format sorts
correctly. Free-form dates ("May 15, 2026") will sort, but lexicographic
order won't match chronological order — that's a content bug, not a
collector bug.

## Posts without a date

A missing or empty `frontmatter[dateField]` is treated as **soft
warning**, not a fatal error:

- A warning is emitted on `ctx.logger.warn` (one per dateless post).
- The entry is given the sentinel date `"0000-01-01"` so it sorts
  reliably to the END of a descending list.
- The post still appears in the output — the renderer can decide what
  to do with it.

Rationale: dropping silent is a sharp tool; sort-to-back is recoverable
and the warning surfaces in the build log.

## Config

```ts
interface CollectChronologicalConfig {
  name?:          string;   // collection name, default "posts"
  dateField?:     string;   // frontmatter key for date, default "date"
  slugField?:     string;   // frontmatter key for explicit slug, default "slug"
  routeTemplate?: string;   // route template, default "/blog/{slug}.html"
}
```

`undefined` config is treated identically to `{}`.

## Slug derivation rules

`slugify(sourcePath)` (the fallback when frontmatter doesn't supply
one):

1. Take the basename (last path segment; splits on `/` and `\`).
2. Strip a trailing `.md` / `.mdx` / `.markdown` (case-insensitive).
3. Lowercase.
4. Replace whitespace / `_` with `-`.
5. Drop characters outside `[a-z0-9-]`.
6. Collapse repeated `-`; trim leading/trailing `-`.
7. Fall back to `"untitled"` if the result would be empty.

## v0 simplifications (documented)

- Only string-typed frontmatter values are supported (the parser-markdown
  v0 only produces strings anyway).
- Route templates only support `{slug}` substitution. `{year}` /
  `{month}` / `{day}` / `{section}` etc. are deferred to a future
  collector that knows about dates structurally.
- Single fixed `discriminant: "chronological"`. A separate collector
  variant will own per-tag / per-year discriminants.

## Dependencies

- `@coding-adventures/forme-types` — `Kinds`, `streamOf`,
  `ContentNode`, `Collection`, `CollectionEntry`, `OrderKey`.
- `@coding-adventures/forme-stage` — `defineStage`, `StageContext`.

## Tests

```
npx vitest run --coverage
```

Coverage target 90%+ line. See `tests/slug.test.ts` for the slug
derivation matrix and `tests/stage.test.ts` for the full collector
suite.
