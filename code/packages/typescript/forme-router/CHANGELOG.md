# Changelog — @coding-adventures/forme-router

## 0.1.0 — 2026-05-16

Initial release. Standalone Forme route-derivation stage,
extracting the slug + route logic that was duplicated across
`forme-collect-chronological` and `forme-render-static` v0.

### Added

- `router` default-exported stage:
  - `consumes: streamOf(Kinds.ContentNode)`
  - `produces: streamOf(Kinds.ContentNode)`
  - `capabilities: []` (pure transform)
  - `configSchema: { routeTemplate?: string; slugField?: string }`
- Per-node pipeline:
  1. Read `node.frontmatter[slugField]`; use as slug if non-empty string.
  2. Otherwise compute `slugify(node.sourcePath)`.
  3. Format `route` via `formatRoute(routeTemplate, slug)`.
  4. Emit a new `ContentNode` with `route` set (revision and identity
     preserved — route is metadata, not content).
- `slugify(sourcePath)` — basename, strip markdown extension,
  lowercase, normalize separators, drop non-`[a-z0-9-]`, collapse
  repeats, trim, fallback `"untitled"`. POSIX and Windows path
  separators both handled.
- `formatRoute(template, slug)` — `{slug}` substitution only in v0.
- Cancellation honoured between input nodes.

### Spec adherence

No deliberate divergences from FM00 §5.4. v0 simplifications:

- Only `{slug}` template substitution. `{year}` / `{month}` /
  `{section}` deferred to a future stage that knows about dates and
  collection metadata structurally.
- Only string-typed slug frontmatter values supported (matches the
  forme-parse-markdown v0 surface).
- No collection-aware route assignment — the router treats each
  node independently. A future "collection-aware" router can
  consume a `Collection` value and emit routes that respect the
  collection's `discriminant` (e.g. per-tag or per-year subpaths).

### What's NOT updated by this PR

- `forme-render-static` still derives its own routes. A follow-up
  PR will update it to read `ContentNode.route` (set by this
  stage) when present, falling back to local derivation when not.
- `forme-collect-chronological` still derives its own routes
  (because it can't depend on `forme-router`, which would create
  a cycle — both currently consume `Stream<ContentNode>`). The
  collector's local derivation stays in place; eventually the
  collector can simply READ `node.route` rather than re-derive.

### Why a separate package, not a shared utility

The slug rules are stage-shaped, not utility-shaped — they have a
config schema, they touch a `ContentNode`, they have to plug into
the orchestrator's DAG. A shared utility would split the data
shape (in `forme-types`) from the policy (in some new
`-text-utils`), and stages would re-import both. Cleaner to keep
the policy where it gets used: as a stage.

### Notes

- The duplicated `slug.ts` files in `forme-collect-chronological`
  and `forme-render-static` are NOT removed in this PR. They stay
  in place until those packages are updated to read from
  `ContentNode.route`. Each removal will be a focused follow-up.
