# @coding-adventures/forme-doc-site-emitter

> **Eleventh and FINAL DOC00 v0 package.**  The glue that turns
> the DOC00 per-stage outputs (rendered pages, sidebar tree,
> search manifest + shards + client JS) into a `PageBundleConfig`
> the FM00 deploy chain consumes unchanged.

Pure transform.  Capabilities: `[]`.  Per the DOC00 spec
(section 8: "Every DOC00 package has
`required_capabilities.json` → `capabilities: []`.  **No
exceptions in v0.**"), site-emitter never instantiates any I/O
primitive.  It returns a data structure.  The actual disk writes
happen downstream in `forme-aot-page-bundle-emitter` →
`forme-aot-deploy-manifest-emitter` → `forme-deploy-runner`,
which already own the relevant capabilities.

## What it does

```ts
import { emitSite } from "@coding-adventures/forme-doc-site-emitter";
import { generatePageBundle } from "@coding-adventures/forme-aot-page-bundle-emitter";

const bundle = emitSite({
  pages: [
    { route: "/",           html: indexHtml,  lastmod: "2026-05-22" },
    { route: "/guide/setup", html: setupHtml, lastmod: "2026-05-22" },
    { route: "/api",        html: apiHtml,    lastmod: "2026-05-22" },
  ],
  sidebar,                       // from forme-doc-sidebar-builder
  search: {                      // from forme-doc-search-index-builder
    manifest,
    shards,
    clientJs: builtSearchClientBundle,  // caller bundles for browser
  },
  extras: [
    { route: "/favicon.ico", content: faviconBytesString, contentType: "image/x-icon" },
    { route: "/robots.txt", content: "User-agent: *\nAllow: /\n", contentType: "text/plain" },
  ],
  baseUrl: "https://docs.example.com",
});

// PageBundleConfig — consumable by the FM00 deploy chain.
const manifestJson = generatePageBundle(bundle);
```

## Capability flow

```
DOC00 cluster (all []):                       FM00 cluster:

 frontmatter, heading-anchors, ...
              ▼
 page-shell  → DocPage[] ─┐
 sidebar-builder ─────────┼──► forme-doc-site-emitter (this, [])
 search-index-builder ────┘                ▼
                                   PageBundleConfig
                                           ▼
                          forme-aot-page-bundle-emitter ([])
                                           ▼
                       forme-aot-deploy-manifest-emitter ([])
                                           ▼
                              forme-deploy-runner (owns fs:write,
                                                   net:* for upload)
                                           ▼
                                    site on disk / S3 / CDN
```

The capability boundary is between the deploy runner and
everything above it.  DOC00 v0 entirely lives in the
"capabilities: `[]`" half.

## Inputs

| Field         | Type                              | Purpose                                                                          |
|---------------|-----------------------------------|----------------------------------------------------------------------------------|
| `pages`       | `DocPage[]`                       | Per-page rendered HTML (from `forme-doc-page-shell`).                            |
| `sidebar`     | `SidebarTree?`                    | Sidebar nav tree (from `forme-doc-sidebar-builder`).  Emitted as `/sidebar.json`. |
| `sidebarPath` | `string?`                         | Override default `/sidebar.json` route.                                          |
| `search`      | `SearchAssets?`                   | Manifest + shards + (optional) client JS.  Emitted under `/search/`.             |
| `extras`      | `ExtraFile[]?`                    | Favicons, `robots.txt`, `CNAME`, copied assets — arbitrary extra routes.         |
| `baseUrl`     | `string?`                         | Canonical site URL.  Forwarded into `PageBundleConfig.baseUrl`.                  |
| `maxPages`    | `number?` (default `100_000`)     | Cap on total pages — defends against pathological inputs.                        |
| `maxShards`   | `number?` (default `10_000`)      | Cap on shard count.                                                              |
| `maxExtras`   | `number?` (default `10_000`)      | Cap on extras count.                                                             |

## Route layout (default)

```
/                              ── pages[0]
/guide/setup                   ── pages[1]
/api                           ── pages[2]
...
/sidebar.json                  ── sidebar
/search/manifest.json          ── search.manifest
/search/<key>.json             ── search.shards[<key>], one per shard
/search/client.js              ── search.clientJs (if provided)
/favicon.ico                   ── extras[0]
/robots.txt                    ── extras[1]
...
```

Override the sidebar path with `sidebarPath`, the search root
with `search.basePath`.

## Route validation (defence-in-depth)

Every emitted route is checked against the same rules
`forme-aot-page-bundle-emitter.validateRoute` enforces — so a
malformed route is caught at the boundary closest to the user's
input, not deep inside the deploy chain:

- Must start with `/`.
- Must not contain `\` (Windows path separator).
- Must not contain `//` (protocol-relative URL hint —
  `//evil.example.com/path` parses as cross-origin).
- Must not contain `..` segments (path traversal).
- Must not contain control chars (`< 0x20` or `0x7f`).
- Length cap: 8192 chars.

All checks use explicit `charCodeAt` index loops — no regex on
user input.  This matches the project-wide convention
established after CodeQL's `js/polynomial-redos` flagged
`+`-quantified regex even when not actually polynomial.

## Duplicate-route detection

If two inputs would land on the same route — e.g. a page at `/`
and an `ExtraFile` also at `/`, or a sidebar at
`/search/manifest.json` colliding with a search manifest — the
emitter throws.  Silent collision-resolution would surface as
mysterious missing pages at deploy time.

## Determinism

For a fixed input, `emitSite` produces a byte-identical
`PageBundleConfig`.  This matters for content-addressed deploys
where unchanged builds should hash identically:

- Page order in the output preserves input order.
- Sidebar JSON: stable `JSON.stringify` (no `replacer`, no
  `indent`).
- Shard JSON: tokens are emitted in **sorted** order — we don't
  trust the Map's insertion order.
- Search-shard iteration order follows the manifest's sorted
  `shardKeys` array, not the input `Map`'s key-iteration order.

## JSON serialisation choices

- **No `indent` argument** to `JSON.stringify` — compact JSON
  ships smaller and is more diff-stable.
- **No `replacer`** — avoids the "replacer can mutate / throw on
  circular" footguns.
- **`Object.create(null)` for shard `postings`** maps — defends
  downstream consumers from accidentally inheriting `toString`,
  `hasOwnProperty`, etc. when they `obj[token]` the result.
  `JSON.stringify` handles null-prototype objects identically to
  plain ones.

## Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver** —
  pure data manipulation.
- **No I/O primitives instantiated.**  Capabilities `[]`.
- **Numeric option validation at the boundary** —
  `maxPages` / `maxShards` / `maxExtras` checked with
  `Number.isFinite` AND `Number.isInteger`.  NaN, Infinity,
  negatives, and floats all throw.
- **Route validation up-front** — same rules as the downstream
  emitter, applied at the closest-to-user-input point.  No regex
  on user input (explicit `charCodeAt` loops).
- **Shard-key validation** — non-empty, ≤ 256 chars, no `/`, no
  `\`, no control chars.  A shard key with `/` would change
  which directory the shard ends up in.
- **No prototype pollution** — Map-based lookups throughout;
  shard `postings` objects built via `Object.create(null)`.
- **Duplicate-route detection** — fail-fast rather than letting
  one entry silently shadow another.
- **Bounded input sizes** — all three count caps prevent
  pathological inputs from amplifying into bundle bloat.
- **Defensive `shardKeys: undefined`** — if a caller hands us a
  partial manifest with no `shardKeys` array, we emit the
  manifest but skip shard emission rather than throwing.

## Tests

67 tests in `tests/emitter.test.ts`:

- **Input shape** — happy + null + non-object + non-array pages
  + non-string html / lastmod / baseUrl.
- **Route shape** — every failure mode (non-string, empty, no
  leading slash, backslash, `//`, `..`, control chars, DEL,
  length cap) + accepted dotted routes.
- **Numeric options** — NaN / Infinity / negative / non-integer
  for all three caps, plus exceed-cap detection.
- **Pages** — single + multiple + preserved order + lastmod
  forwarded + baseUrl forwarded + duplicate-route throw.
- **Sidebar** — default path + override + non-array throw +
  invalid sidebar path throw + collision-with-page throw.
- **Search** — manifest + shards + clientJs (with and without)
  + basePath override + trailing-slash rejection + every
  validation throw (non-Map, missing shard, non-string
  shardKey, `/` in shardKey, empty shardKey, 256+ char
  shardKey, exceed-maxShards, null search, null manifest,
  non-Map postings, non-string shardKey on shard, null shard,
  non-string token keys) + tolerates missing `shardKeys`
  array + sorted token iteration.
- **Extras** — happy + all field validations + duplicate-route
  detection.
- **Determinism** — byte-identical bundle JSON on repeat call.
- **Round-trip** — output feeds cleanly into `generatePageBundle`
  with all expected `outputPath`s derived correctly.
- **Constants** — default values sanity check.
- **Realistic** — 3-page docs site with sidebar + search +
  favicon + robots.

Coverage: **100% line / 100% branch / 100% function** on all
source files with logic (`types.ts` is type-only).

## How it fits in the stack

Final DOC00 v0 package.  The cluster after this:

| Package                              | Layer    | Capability |
|--------------------------------------|----------|------------|
| `document-ast`                       | shared IR| `[]`       |
| `forme-doc-frontmatter`              | content  | `[]`       |
| `forme-doc-heading-anchors`          | content  | `[]`       |
| `forme-doc-toc-extractor`            | content  | `[]`       |
| `forme-doc-code-block-decorator`     | content  | `[]`       |
| `forme-doc-syntax-highlighter`       | content  | `[]`       |
| `forme-doc-sidebar-builder`          | structure| `[]`       |
| `forme-doc-page-shell`               | structure| `[]`       |
| `forme-doc-search-tokenizer`         | search   | `[]`       |
| `forme-doc-search-index-builder`     | search   | `[]`       |
| `forme-doc-search-client-js`         | search   | `[]`       |
| **`forme-doc-site-emitter`**         | **glue** | **`[]`**   |
| `forme-aot-page-bundle-emitter`      | FM00     | `[]`       |
| `forme-aot-deploy-manifest-emitter`  | FM00     | `[]`       |
| `forme-deploy-runner`                | FM00     | `fs:write`, `net:*` |

**Eleven new DOC00 packages, all `capabilities: []`.**  The
single capability boundary in the entire docs pipeline lives in
the deploy runner.

## v0 simplifications (documented)

- **No `bytes` variant for extras** — binary files (favicons,
  images) ship as strings.  Adequate for v0; the upstream
  `PageBundleConfig.PageEntry` schema uses strings throughout.
  v1 may add `Uint8Array` support.
- **No theme/asset bundling** — caller pre-bundles CSS/JS and
  passes them as `extras` or as part of the `html` body.
- **No sitemap.xml / RSS generation** — caller composes those
  as `extras` if wanted.  (`forme-aot-deploy-manifest-emitter`
  downstream already supports them as separate inputs.)
- **No dev-server integration** — out of scope per DOC00 spec
  section 8; a separate `fs:watch` + `net:listen` program.
- **String content only** — no streams / no async generators.
  Docs sites are small enough that the entire bundle fits in
  memory comfortably.
