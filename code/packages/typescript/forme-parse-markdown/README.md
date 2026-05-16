# @coding-adventures/forme-parse-markdown

Forme parse stage: `Stream<ContentSource>` element → `ContentNode`.
Wraps [`@coding-adventures/gfm-parser`](../gfm-parser) and extracts a
v0 subset of YAML-style frontmatter.

This is the second Forme stage of the blog v0 effort.  Sits between
`forme-source-fs` (which produces `ContentSource` per file on disk) and
the collector stages (which sort and assign routes).

## Stage shape

```ts
import parseMarkdown from "@coding-adventures/forme-parse-markdown";

parseMarkdown.consumes      // Kinds.ContentSource
parseMarkdown.produces      // Kinds.ContentNode
parseMarkdown.capabilities  // []  ← pure transform
parseMarkdown.configSchema  // { type: "object", properties: { gfm: {type:"boolean"} } }
```

## What it does

For each input `ContentSource`:

1. **Decode** `bytes` as UTF-8 (BOM is stripped if present).
2. **Split** off any leading frontmatter block (see grammar below).
3. **Parse** the remaining body with `gfm-parser` → `DocumentNode`.
4. **Recompute revision** = `blake2b({ documentJson, frontmatter, sourcePath })`.
5. **Emit** a `ContentNode` carrying through `identity` and
   `sourcePath`, with `route: null` and `assetRefs: []` (those are
   later-stage concerns).

## Frontmatter grammar (v0)

Intentionally minimal — hand-rolled, no `js-yaml` dependency.

```
---
<key>: <value>
<key>: <value>
---
<markdown body...>
```

Rules:

- Opening fence (`---\n`) must begin at byte 0.
- Closing fence is `---` on a line by itself.
- Each interior line is `<key>: <value>` (value is everything after the
  first colon, trimmed).
- **All values are strings.**  No quoted strings, no numbers, no
  booleans, no arrays, no nested maps.  Consumers parse if they want.
- Blank interior lines are ignored.
- Any malformed line (missing colon, empty key) invalidates the WHOLE
  block — we fall back to "no frontmatter" and feed the original text
  to the parser verbatim.  This matches Jekyll's behaviour.
- Missing closing fence is treated the same way.

Anything richer (quoted strings, arrays, nested maps, dates as native
values) is deferred to a future sibling stage that wraps a real YAML
parser — kept separate so this stage stays zero-dependency.

## Identity & revision discipline

- **`identity`** is passed through from the source unchanged.  It's
  the document's persistent name — re-parsing the same source yields
  the same identity.
- **`revision`** is recomputed.  It hashes `{ documentJson, frontmatter,
  sourcePath }`, so:
  - editing the body changes the revision (AST changes);
  - editing frontmatter changes the revision;
  - moving the file to a new path changes the revision (collectors
    usually key off path).
  Two parses of the same input from the same path are byte-identical
  revisions, so the cache layer short-circuits cleanly.

## Config

```ts
interface ParseMarkdownConfig {
  gfm?: boolean;  // defaults true; currently no-op (gfm-parser is GFM-only)
}
```

The `gfm` flag is reserved for forward compatibility — `gfm-parser`
doesn't yet support disabling extensions, but accepting the flag now
means a future change is a parser-internal diff, not a config-surface
break.

## v0 simplifications

- `route` is always `null` — route assignment is a collector's job.
- `assetRefs` is always `[]` — asset extraction is a separate future
  stage that walks the document AST.
- Frontmatter values are always strings (see grammar above).
- `gfm: false` is accepted but ignored.

## Dependencies

- `@coding-adventures/forme-types` — `Kinds`, `ContentSource`,
  `ContentNode`, `AssetRef`, `JsonValue`.
- `@coding-adventures/forme-stage` — `defineStage`.
- `@coding-adventures/forme-identity` — `computeRevisionId`.
- `@coding-adventures/gfm-parser` — the actual Markdown engine.
- `@coding-adventures/document-ast` — `DocumentNode` type only.

## Tests

```
npx vitest run --coverage
```

Coverage target: 90%+ line.  See `tests/frontmatter.test.ts` for the
edge-case matrix and `tests/stage.test.ts` for the integration suite.
