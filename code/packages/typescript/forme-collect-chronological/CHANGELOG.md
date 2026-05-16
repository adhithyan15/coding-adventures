# Changelog — @coding-adventures/forme-collect-chronological

## 0.1.0 — 2026-05-15

Initial release. Third Forme stage of the blog v0 effort.

### Added

- `collectChronological` default-exported stage:
  - `consumes: streamOf(Kinds.ContentNode)`
  - `produces: Kinds.Collection`
  - `capabilities: []` (pure transform)
  - `configSchema: { name?, dateField?, slugField?, routeTemplate? }`
- Buffer → derive `{ dateStr, slug, route }` → sort descending by date
  with `sourcePath` ascending tiebreak → emit single `Collection` with
  `discriminant: "chronological"`.
- `slugify(sourcePath)` — basename, strip markdown extension, lowercase,
  whitespace/`_` → `-`, drop non-`[a-z0-9-]`, collapse repeats, trim,
  fallback `"untitled"`. Handles both POSIX and Windows path separators.
- `formatRoute(template, slug)` — substitutes `{slug}` (only).
- Missing-date posts: warning via `ctx.logger.warn`, sentinel date
  `"0000-01-01"`, sorted to end of descending list (still emitted).
- Cancellation honoured between input nodes.

### Spec adherence

No deliberate divergences from FM00 §5.4.

### v0 simplifications (documented)

- **Only string-typed frontmatter values** supported (matches the
  parser-markdown v0 surface).
- **Route templates only support `{slug}`.** `{year}` / `{month}` /
  `{day}` / `{section}` deferred to a future collector that knows
  about dates structurally.
- **Single fixed `discriminant: "chronological"`.** Per-tag /
  per-year discriminants will live in a separate collector variant.
- **Missing dates are warned, not errored.** Posts still appear in the
  output, sorted to the end. Documented in README rationale.

### Notes

- Dates are compared as strings — ISO-8601 (`YYYY-MM-DD`) sorts
  correctly because of fixed-width numeric prefixes. The README
  documents the convention; free-form dates ("May 15, 2026") will
  sort lexicographically (which is not what users want).
- `CollectionEntry` stores `identity` + `revision` references back to
  the `ContentNode`, NOT the node itself — per FM00 §5.4, collections
  of millions of entries must stay cheap to hash and diff.
- Tie-break by `sourcePath` is *ascending* (not date) so the output is
  byte-deterministic across runs regardless of stream arrival order.
