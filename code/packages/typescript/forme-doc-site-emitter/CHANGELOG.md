# Changelog — @coding-adventures/forme-doc-site-emitter

## 0.1.0 — 2026-05-22

Initial release.  **Eleventh and FINAL DOC00 v0 package** —
closes the DOC00 v0 cluster.  Glue package that composes
DOC00 per-stage outputs (rendered pages + sidebar tree + search
manifest/shards/client.js + extras + baseUrl) into a single
`PageBundleConfig` the FM00 deploy chain consumes unchanged.

Pure transform.  Capabilities: `[]`.  The disk-write capability
lives with `forme-deploy-runner` downstream — DOC00 itself
never instantiates an I/O primitive.

### Added

- `emitSite(input): PageBundleConfig` — the main entry.
- Default constants: `DEFAULT_SIDEBAR_PATH` (`/sidebar.json`),
  `DEFAULT_SEARCH_BASE_PATH` (`/search`), `DEFAULT_MAX_PAGES`
  (100_000), `DEFAULT_MAX_SHARDS` (10_000), `DEFAULT_MAX_EXTRAS`
  (10_000), `CONTENT_TYPE_JSON`, `CONTENT_TYPE_JS`.
- Types: `DocPage`, `ExtraFile`, `SearchAssets`, `SiteEmitInput`,
  `SidebarTree`, `PageBundleConfig` (re-exported from the
  upstream emitter so callers don't need a separate import).

### Spec adherence

Implements DOC00 v0's `forme-doc-site-emitter` per
`code/specs/DOC00-docs-vision.md` (sections 247 and 419) —
"top-level composer" that produces a `PageBundleConfig` for
`forme-aot-page-bundle-emitter`.

**Spec divergence flagged for clarity (no behavioural deviation):**

The babysit-cron prompt that triggered this implementation
guessed the capability would be `[fs:write]` since the package
"writes the final site to disk".  Re-reading the spec section 8
shows the correct answer is `[]` — site-emitter does NOT write
to disk, it returns a `PageBundleConfig` data structure; the
disk writes happen downstream in `forme-deploy-runner` (which
already owns `fs:write` per FM05).  The spec is explicit: "Every
DOC00 package has `required_capabilities.json` → `capabilities:
[]`.  **No exceptions in v0.**"  This package follows the spec.
The cron prompt's guess was wrong; this implementation does NOT
follow the cron prompt, it follows the spec.

### Behavioural notes

- **Pure transform.**  Capabilities `[]`.  No I/O.
- **Route validation up-front** — every emitted route is checked
  against the same rules `forme-aot-page-bundle-emitter`
  enforces: must start with `/`, no `\`, no `//`, no `..`
  segment, no control chars, ≤ 8192 chars.  All checks use
  explicit `charCodeAt` index loops (no regex on user input —
  matches the project-wide ReDoS-prevention convention).
- **Duplicate-route detection** — if two inputs would land on
  the same route, throw rather than silently let one shadow
  the other.
- **Deterministic output** — given the same input, the bundle
  serialises to byte-identical JSON.  Shard tokens are emitted
  in sorted order; shard iteration follows the manifest's
  sorted `shardKeys` array (not the input `Map`'s insertion
  order).
- **`Object.create(null)` for shard `postings` output** —
  defends downstream consumers from accidentally inheriting
  `toString` / `hasOwnProperty` when they `obj[token]` the
  result.
- **Manifest tolerance** — if a caller hands us a manifest with
  no `shardKeys` array, we emit `manifest.json` but skip shard
  emission rather than throwing on iterator.

### Security posture

- **No `eval` / `new Function` / `JSON.parse`-with-reviver** —
  pure data manipulation.
- **No I/O primitives instantiated.**  Capabilities `[]`.
- **Numeric option validation at the boundary** —
  `maxPages` / `maxShards` / `maxExtras` checked with
  `Number.isFinite` AND `Number.isInteger`.  NaN, Infinity,
  negatives, and floats all throw.  `>= 0` (not `>= 1`) is
  intentional — `maxPages: 0` is a legitimate assertion.
- **Route validation up-front** — same rules as the downstream
  emitter; no regex on user input.
- **Shard-key validation** — non-empty, ≤ 256 chars, no `/`,
  no `\`, no control chars.  A shard key with `/` would
  silently change which directory the shard ends up in.
- **No prototype pollution** — Map-based lookups throughout;
  shard `postings` output via `Object.create(null)`.
- **Duplicate-route detection** — fail-fast.
- **Bounded input sizes** — three count caps prevent
  pathological inputs from amplifying into bundle bloat.

### Tests

67 tests in `tests/emitter.test.ts`:

- Input shape (6 validation throws).
- Route shape (every failure mode + accepted dotted routes).
- Numeric options (NaN/Infinity/negative/non-integer for all
  three caps + exceed-cap detection + `maxPages: 0` legitimate
  case).
- Pages (single, multiple, preserved order, lastmod forwarded,
  baseUrl forwarded, duplicate-route throw).
- Sidebar (default path + override + non-array throw + invalid
  path throw + collision throw).
- Search (manifest + shards + clientJs with/without + basePath
  override + trailing-slash rejection + every shard-validation
  throw + sorted-token determinism + tolerates missing
  `shardKeys` array).
- Extras (happy + all field validations + duplicate-route
  detection).
- Determinism (byte-identical bundle JSON on repeat call).
- Round-trip (output feeds cleanly into `generatePageBundle`).
- Default constants (sanity check).
- Realistic (3-page docs site with sidebar + search + favicon
  + robots).

Coverage: **100% line / 100% branch / 100% function** across
all source files with logic (`types.ts` is type-only).

### v0 simplifications (documented)

- **No `bytes` variant for extras** — binary files ship as
  strings (the upstream `PageBundleConfig.PageEntry` uses
  strings throughout).  v1 may add `Uint8Array`.
- **No theme / asset bundling** — caller pre-bundles CSS/JS.
- **No sitemap.xml / RSS generation** — composed downstream by
  `forme-aot-deploy-manifest-emitter`.
- **No dev-server integration** — separate program, per spec.
- **String content only** — no streams.

### DOC00 v0 cluster: COMPLETE

This is the eleventh and final DOC00 v0 package.  Cluster
membership:

1. `forme-doc-frontmatter`
2. `forme-doc-heading-anchors`
3. `forme-doc-toc-extractor`
4. `forme-doc-code-block-decorator`
5. `forme-doc-syntax-highlighter`
6. `forme-doc-sidebar-builder`
7. `forme-doc-page-shell`
8. `forme-doc-search-tokenizer`
9. `forme-doc-search-index-builder`
10. `forme-doc-search-client-js`
11. **`forme-doc-site-emitter`** ← this PR

Every one a pure transform; capabilities `[]`; the single
capability boundary in the entire DOC00 pipeline lives in the
already-shipped `forme-deploy-runner` (FM00 v0).
