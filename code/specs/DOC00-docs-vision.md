# DOC00 — Documentation Site Cluster Vision

> **Status:** v0 cluster complete. All eleven DOC00 packages ship.
> **Layer:** DOC00 (new top-level cluster, parallel to FM00).
> **Depends on:** `commonmark-parser`, `gfm-parser`,
> `document-ast`, `document-ast-to-html` (already shipped) +
> the entire FM00 v0 emitter cluster (just completed).

## 1. Purpose

Every package in this repo needs documentation. Today there are
hundreds of packages across many languages, and the set keeps
growing. DOC00 provides the capability-free transforms used to
assemble documentation sites; it is not a turnkey site generator.
Callers compose the stages and own source discovery, CSS, browser
UI wiring, bundling, and I/O. The shipped pieces are:

- **Composable** — focused stages transform frontmatter, ASTs,
  navigation, search data, rendered pages, and bundle entries.
- **Trivial to host** — pure HTML, CSS, and JS.  Drop into S3,
  Cloudflare Pages, GitHub Pages, a fileserver.  No runtime
  server logic.  No external services.
- **Searchable client-side** — build deterministic index shards
  and run exact-token queries without Algolia, Elasticsearch, or
  another hosted search service. The caller owns the results UI.
- **Small by construction** — no framework or search runtime is
  bundled by the cluster. Product callers measure their final
  browser bundle and page-load budgets after supplying UI assets.
- **Composable with FM00** — the cluster's terminal package
  produces a `PageBundleConfig` for
  `forme-aot-page-bundle-emitter`, so docs sites flow through
  the same deploy manifest path as everything else.

DOC00 is the layer above FM00.  FM00 produces individual HTML
documents and packs them for deploy; DOC00 produces the
specific kind of multi-page documentation site that turns a
package's markdown source into a navigable, searchable static
site.

## 2. Why not Docusaurus / Mintlify / readthedocs / Pagefind

### 2.1 Docusaurus

- React + MDX + Webpack — heavy runtime, big learning surface,
  ships React to every visitor.
- Search via **Algolia DocSearch** — external service, free tier
  capped, requires approval, recurring vendor dependency.
- Doesn't compose with anything else in this repo.

### 2.2 Mintlify

- Hosted SaaS.  Pretty, but you're renting your docs site.
- Search is part of the hosted offering.
- No.

### 2.3 ReadTheDocs

- Closest in spirit to what we want — minimal, two-column,
  hosted for free.  But: the hosted version requires their
  build pipeline (Python-centric), the self-hosted version
  is a server, not a static site, and search is
  Elasticsearch.
- The **aesthetic** we want (RTD's two-column layout, fast,
  no-frills).

### 2.4 Pagefind

The closest existing static-site-search solution.  Rust +
WASM, builds sharded indexes, decent UX.  ~50KB + WASM
runtime.  **Best current option** if we were going to use
existing tooling.

But:
- This repo's whole ethos is build-from-scratch with literate
  programming.  Search engines are exactly the canon-CS
  territory worth implementing ourselves — tokenization,
  inverted indexes, deterministic sharding, bounded caches, and
  ranking are educational gold.
- Pagefind doesn't fit the FM00 contract (pure-transform
  packages with `[]` capabilities).  Wrapping it is awkward;
  re-implementing the same ideas in TypeScript fits.
- Pagefind ships WASM. DOC00 instead exposes capability-free
  TypeScript search primitives and lets the product caller decide
  how to bundle and load them.

The **principle** is: every dependency we own is a dependency
we understand.  Once docs sites are widespread, search bugs in
DOC00 are bugs we can debug ourselves — not opaque black-box
behaviour from a vendored binary.

## 3. Architecture overview

DOC00 follows the same layered shape as FM00: many small
pure-transform packages, each owning one transform stage,
composed via type contracts.  No giant monolithic "docs
generator" package.

```
┌─ Content pipeline ─────────────────────────────────────┐
│  forme-doc-frontmatter                                  │
│  forme-doc-heading-anchors                              │
│  forme-doc-toc-extractor                                │
│  forme-doc-code-block-decorator                         │
│  forme-doc-syntax-highlighter                           │
└─────────────────────────────────────────────────────────┘
            ↓ (AST + metadata per .md file)
┌─ Site structure ────────────────────────────────────────┐
│  forme-doc-sidebar-builder                              │
│  forme-doc-page-shell                                   │
└─────────────────────────────────────────────────────────┘
            ↓ (HTML chunks per page, with chrome)
┌─ Search engine ─────────────────────────────────────────┐
│  forme-doc-search-tokenizer                             │
│  forme-doc-search-index-builder    ← build-time         │
│  forme-doc-search-client-js        ← browser-ready logic│
└─────────────────────────────────────────────────────────┘
            ↓ (manifest + shards + optional caller-bundled client JS)
┌─ Site assembly ─────────────────────────────────────────┐
│  forme-doc-site-emitter                                 │
└─────────────────────────────────────────────────────────┘
            ↓ (PageBundleConfig for forme-aot-page-bundle-emitter)
┌─ FM00 deploy path (already built) ──────────────────────┐
│  forme-aot-page-bundle-emitter                          │
│  forme-aot-deploy-manifest-emitter                      │
│  forme-deploy-runner                                    │
└─────────────────────────────────────────────────────────┘
```

Eleven new packages.  Every one a pure transform; capabilities
`[]`.  Each ships with the same template as FM00 (BUILD,
README, CHANGELOG, required_capabilities.json, ≥95% line
coverage tests, pre-push security review).

## 4. Per-package outline

### Content pipeline

#### `forme-doc-frontmatter`
Strip YAML or TOML frontmatter from a `.md` source string;
return `{ body: string, frontmatter: Record<string, unknown> }`.
The body goes to `commonmark-parser` next; the frontmatter is
metadata (title, sidebar position, draft flag, etc.).

#### `forme-doc-heading-anchors`
Walk a `DocumentNode` AST; for every heading, generate a
URL-safe slug ID and inject it as a heading attribute.
Deterministic slug derivation (no random suffix); the first
collision gets `-1`, the next `-2`, and so on.

#### `forme-doc-toc-extractor`
Walk a `DocumentNode` AST → table of contents tree (heading
text + slug + depth).  Output is a plain JSON-able structure
the sidebar / in-page TOC widget can render.

#### `forme-doc-code-block-decorator`
AST transform that decorates fenced code blocks with metadata for:
- A copy-button hook.
- A language label.
- An optional filename badge (from `// file:foo.ts` style
  hints).
- A line-number-gutter flag if requested.

#### `forme-doc-syntax-highlighter`
**Interface-first AOT highlighter transform.** v0 walks code
blocks, preserves their metadata, and attaches a single deterministic `plain` span.
That span covers the complete source. The exported span and token
types are the stable downstream contract. An informational
hint set names TypeScript, JavaScript, Python, Ruby, Go, Rust,
Bash, JSON, HTML, CSS, and Markdown; v0 does not branch on it.

Grammar-backed colouring and themes are v1 work. They can replace
the plain-span implementation without changing the v0 signatures.

### Site structure

#### `forme-doc-sidebar-builder`
Take a directory layout (file paths) + each file's
frontmatter (sidebar position, title overrides, group
metadata) → sidebar nav structure.  Output is a plain
JSON-able tree the page shell renders to HTML.

#### `forme-doc-page-shell`
Wrap content in the RTD-minimal two-column shell: sidebar on
the left, content in the middle, optional in-page TOC on the
right.  Header bar at the top with site title, GitHub link,
search input.  Footer with version / "edit this page" link /
copyright.

Outputs HTML chunks suitable for
`forme-aot-html-doc-emitter`'s `head` + `body` fields.

### Search engine

#### `forme-doc-search-tokenizer`
Text → tokens.  Pipeline:
1. Lowercase.
2. Strip punctuation (keep alphanumerics, drop everything else).
3. Split on whitespace.
4. Filter stop-words (optional — small built-in list).
5. (Optional) Porter stemmer — small, well-known algorithm.

Pure transform.  Same tokenization runs at index time (in
Node) and at query time (in browser).  Determinism between
the two is critical.

#### `forme-doc-search-index-builder`
Build-time. Takes `{ id, title?, body }` entries and returns an
in-memory inverted index plus bootstrap manifest. Tokens are
sharded by a deterministic prefix (two characters by default).
Each posting contains `pageId`, frequency, and whether the token
occurred in the title. The manifest records sorted page and shard
keys plus the tokenization options needed by the client.

The builder caps pages, tokens per page, and postings per token.
It performs no I/O or serialization; `forme-doc-site-emitter`
turns the manifest and shards into JSON bundle entries.

#### `forme-doc-search-client-js`
Pure browser-ready search logic with no built-in I/O. A caller
supplies the manifest and a shard-fetch callback, so network or
filesystem authority stays outside the package. Queries use the
same tokenizer settings as the index, fetch only known prefix
shards, cache them with a bounded LRU, and deduplicate concurrent
fetches.

v0 ranks exact-token matches by frequency with a configurable
title-hit boost, then applies a stable page-id tiebreak. It returns
matched token data for a caller-owned results UI. Failed or malformed
shards are skipped so partial search can continue.

### Site assembly

#### `forme-doc-site-emitter`
Top-level composer.  Takes:
- Complete rendered pages (typically page-shell chunks wrapped by
  `forme-aot-html-doc-emitter`).
- An optional sidebar tree.
- Optional search manifest, shards, and caller-bundled client JS.
- Optional extra string assets plus a base URL.

Produces a `PageBundleConfig` for
`forme-aot-page-bundle-emitter`. Every doc page and auxiliary
asset becomes a validated `PageEntry`; collisions and configured
count limits fail before the deploy chain.

This is the **glue package** that turns DOC00's per-stage
outputs into something the FM00 deploy chain consumes.

## 5. Search engine — shipped v0 design

### 5.1 Deterministic prefix shards

The builder groups each token by `token.slice(0, shardPrefix)`,
using a two-character prefix by default. Short tokens use their
complete value. Sorted page and shard keys make repeated builds
byte-stable after serialization. Callers may choose another
positive prefix width for their corpus; v0 does not promise a
compressed byte size because compression belongs to deployment.

### 5.2 Bounded postings and simple ranking

Each posting records a page ID, term frequency, and title-hit flag.
There are no positions, snippets, forward indexes, bloom filters,
or fuzzy side indexes in v0. Explicit caps bound pages, unique
tokens per page, and postings per token.

The client ranks by frequency with a configurable title-hit boost.
It sorts by descending score and then ascending
page ID, returning a bounded top-K result with matched tokens.
There is no TF-IDF, BM25, phrase search, or fuzzy matching in v0.

### 5.3 Injected fetch and bounded cache

The client accepts a caller-injected shard fetcher. This keeps the
package capability-free and lets browser, filesystem, and in-memory
callers share the same search logic. Known shards load on demand,
concurrent fetches deduplicate, and a bounded LRU cache prevents
unbounded retention. A failed or malformed shard degrades to partial
results rather than failing the complete query.

### 5.4 Deferred search features

Positions and phrase queries, fuzzy matching, richer weighting,
snippet generation, search-result highlighting, analytics, service
worker caching, and any WASM acceleration remain explicit v1+
possibilities rather than shipped v0 behavior.

## 6. Aesthetic: RTD-minimal

The page shell ships the semantic structure needed for an
RTD-minimal presentation:

- A header with site title, optional GitHub link, and optional
  search input markup.
- A layout containing a sidebar, main article, and optional
  in-page TOC.
- A footer with optional version, edit link, and copyright.
- Stable class names, active-page `aria-current`, escaped text,
  and an allowlisted URL boundary.

CSS and browser UI wiring are caller-owned in v0. The package does
not ship a theme, column widths, typography, colours, collapsible
navigation, search-result rendering, or client-side interactions.

**Explicitly NOT in v0**:
- Dark mode toggle (deferred — would need a theme system).
- Hero sections, cards, splashy landing pages.
- Heavy animations.

The goal is "doc site that looks credible by 2026 standards
without trying to be Mintlify."

## 7. v0 scope and exclusions

### Included

- Markdown → HTML pipeline (uses existing
  `commonmark-parser` + `gfm-parser`).
- Heading anchors (deep-linking).
- TOC extraction (sidebar + in-page).
- Code-block presentation metadata plus the stable AOT highlight
  span contract; v0 emits one plain span per block.
- Sidebar nav from directory layout.
- RTD-minimal two-column shell.
- Exact-token client-side search core with injected shard loading
  and deterministic frequency/title-hit ranking; the caller owns
  the browser UI.
- Site composer → FM00 page bundle.

### Deferred to v1+

- **Multi-version docs** (`/v1/`, `/v2/` parallel trees).
  Complicates sidebar + index sharding.  Many projects don't
  need this until 2.0.
- **i18n** (multi-language).  Per-locale indexes,
  language selector.  Substantial complexity.
- **MDX** (JSX-in-markdown).  Requires a JSX parser + a
  React-like runtime shim.  Pure markdown covers ~95% of
  documentation needs.
- **Dev server** (watch mode, hot reload).  Not pure transform
  — would need `fs:watch` capability + a separate program
  outside the DOC00 cluster.
- **Dark mode toggle.**  Tiny but needs a CSS-variable theme
  system + JS for toggle persistence.  Deferred.
- **Grammar-backed syntax colouring and themes.** v0 exposes the
  stable span contract and language hints but emits plain spans.
- **Advanced search.** Positions, phrases, BM25, fuzzy matching,
  snippets, and result highlighting remain v1+ work.
- **Search analytics** (track popular queries, no-result
  queries).  Privacy + capability concerns; punt for now.
- **Versioned content snapshots** (link from "this page" to
  "this page as of v1.x").

### Out of scope (probably ever)

- Comments / discussion (not what doc sites are for).
- WYSIWYG editor (markdown source is the source of truth).
- Authentication / paywalls (anti-pattern for docs).

## 8. Capability budget

Every DOC00 package has `required_capabilities.json` →
`capabilities: []`.

No exceptions in v0.  The dev server (v1+) is the only
component that would need capabilities (`fs:watch`,
`net:listen`), and it sits outside the DOC00 cluster as a
separate program.

The deploy step uses the FM00 deploy runner (capabilities
documented separately in [FM08](FM08-forme-deploy-runner.md)).

## 9. Integration with FM00

The terminal package, `forme-doc-site-emitter`, produces a
`PageBundleConfig` (the same input shape
`forme-aot-page-bundle-emitter` consumes). Every doc page,
sidebar JSON file, search manifest, search shard, optional client
bundle, and caller-provided extra becomes a `PageEntry` with an
explicit route and content type.

The resulting page bundle JSON flows through the FM00 chain
unchanged: `forme-aot-deploy-manifest-emitter` composes it
with sitemap.xml / robots.txt / extras, and the FM08 deploy
runner applies the result to whatever target.

Net effect: **docs sites are FM00 deployments**.  Same trust
boundaries, same atomicity guarantees, same content-addressed
hashes, same diff-mode incremental deploys.

## 10. Completed implementation order

The cluster shipped in this independently testable sequence:

1. `DOC00-docs-vision.md` (this document).  Spec-only PR.
2. `forme-doc-frontmatter` — smallest, narrowest scope.
3. `forme-doc-heading-anchors` — depends only on `document-ast`.
4. `forme-doc-toc-extractor` — depends on heading-anchors.
5. `forme-doc-code-block-decorator` — independent.
6. `forme-doc-syntax-highlighter` — depends on
   code-block-decorator.
7. `forme-doc-sidebar-builder` — depends on frontmatter.
8. `forme-doc-search-tokenizer` — independent.
9. `forme-doc-search-index-builder` — depends on tokenizer.
10. `forme-doc-search-client-js` — depends on tokenizer (same
    tokenization logic, packaged for the browser).
11. `forme-doc-page-shell` — depends on TOC + sidebar.
12. `forme-doc-site-emitter` — depends on everything above +
    forme-aot-page-bundle-emitter.

Twelve delivery steps in total (counting this vision doc as #1).
All are complete.

## 11. Future work (v1+)

Brief catalogue of things v1+ should consider, ordered by
likely user demand:

- **Dark mode** — high demand, modest complexity.
- **Multi-version docs** — required for any project that
  ships breaking changes.
- **Versioned permalinks** — "this page as of vX.Y".
- **Search filters** — restrict to a section, version,
  language.
- **i18n / multi-language** — per-locale indexes + UI.
- **MDX** — for cases where pure markdown isn't expressive
  enough (interactive demos, embedded playgrounds).
- **Dev server** — watch + reload + open browser, as a
  separate program with `fs:watch` + `net:listen`.
- **Search analytics** — privacy-preserving popular-queries
  tracking.
- **Service worker offline mode** — full doc-site PWA.
- **PDF export** — print stylesheet + chapter aggregation.
- **API reference auto-generation** — for TypeScript / Rust /
  Python packages, parse declarations into doc-page input.

## 12. Resolved design decisions for v0

The implementation resolved every v0 decision as follows:

| Decision | Shipped v0 choice | Status |
|---|---|---|
| Markdown engine | Existing `commonmark-parser` / `gfm-parser` | shipped |
| Advanced features | Search engine only; multi-version, i18n, MDX, dev server, and dark mode remain explicit v1+ work | shipped |
| Theme aesthetic | RTD-minimal semantic shell; CSS and browser UI wiring are caller-owned | shipped |
| Timing | Implement after FM00 v0 | shipped |
| Syntax highlighting | Expose the eleven-language hint inventory, preserve decorated block metadata, and emit deterministic plain spans; grammar-backed colouring remains v1 work | shipped |
| Search sharding | Deterministic two-character term-prefix shards by default, configurable to other positive prefix widths; no compressed-byte target is imposed by the pure in-memory builder | shipped |

The last two choices deliberately narrowed the original proposal. v0 keeps the
language contract without bundling TextMate grammars, and it makes shard
partitioning deterministic without coupling a pure transform to deploy-time
compression measurements.
