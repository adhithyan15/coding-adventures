# DOC00 — Documentation Site Cluster Vision

> **Status:** v0 vision document.  Per-package specs and
> implementations follow.
> **Layer:** DOC00 (new top-level cluster, parallel to FM00).
> **Depends on:** `commonmark-parser`, `gfm-parser`,
> `document-ast`, `document-ast-to-html` (already shipped) +
> the entire FM00 v0 emitter cluster (just completed).

## 1. Purpose

Every package in this repo needs documentation.  Today there
are ~hundreds of packages across a dozen languages; soon there
will be more.  Each one should have a documentation site that's:

- **Trivial to build** — one config object + a directory of
  markdown files → a complete deployable static site.
- **Trivial to host** — pure HTML, CSS, and JS.  Drop into S3,
  Cloudflare Pages, GitHub Pages, a fileserver.  No runtime
  server logic.  No external services.
- **Searchable client-side** — type a query, see results, click
  through.  No Algolia.  No Elasticsearch.  No "first-class
  hosted search."  The search engine is part of the build.
- **Fast** — under-100ms search latency, sub-second page loads
  on commodity hardware, < 50KB total JS on the wire.
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
  territory worth implementing ourselves — inverted indexes,
  BM25, bloom filters, base64-packed posting lists are
  educational gold.
- Pagefind doesn't fit the FM00 contract (pure-transform
  packages with `[]` capabilities).  Wrapping it is awkward;
  re-implementing the same ideas in TypeScript fits.
- Pagefind ships WASM.  We can do it in ~30KB of plain JS by
  picking different tradeoffs (sharded index + bloom filters
  + simpler tokenizer).

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
│  forme-doc-search-client-js        ← shipped to browser │
└─────────────────────────────────────────────────────────┘
            ↓ (search shards + ~30KB JS + UI HTML)
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
Deterministic slug derivation (no random suffix); collisions
within one document get `-2`, `-3`, etc. suffixes.

#### `forme-doc-toc-extractor`
Walk a `DocumentNode` AST → table of contents tree (heading
text + slug + depth).  Output is a plain JSON-able structure
the sidebar / in-page TOC widget can render.

#### `forme-doc-code-block-decorator`
AST transform that decorates fenced code blocks with:
- A "copy" button hook (data attribute the JS shim attaches to).
- A language label.
- An optional filename badge (from `// file:foo.ts` style
  hints).
- Line-number gutter markup if requested.

#### `forme-doc-syntax-highlighter`
**AOT (build-time) syntax highlighter.**  Themes are baked
into the output HTML at build time; zero JS shipped to the
browser for syntax colouring.  v0 supports a handful of
languages: TypeScript, JavaScript, Python, Ruby, Go, Rust,
Bash, JSON, HTML, CSS, Markdown.  More languages added as
needed.

The highlighter uses TextMate-style grammars (the same format
VS Code uses) — these are well-documented, well-tested, and
cover essentially every language anyone writes in practice.
v0 grammar set is hand-curated; future versions can load
arbitrary grammars at build time.

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
Build-time.  Takes a list of `{ docId, title, url, body }`
entries; produces:

1. **Inverted index**, sharded by term-prefix into ~50–100KB
   compressed chunks.  Each shard maps `term → [{docId, freq,
   positions}]`.
2. **Bloom filter per shard** (~1KB each, 1% false-positive
   rate).  Client can skip shards without fetching.
3. **Forward index** for snippet generation: `docId →
   {title, url, body-excerpt}`.  Also sharded.
4. **Manifest** (~5KB): all bloom filters compactly serialised
   + total doc count + total term count + shard URLs.

Trigram side-index for fuzzy / typo-tolerant search (extra
~10KB per 1000 documents).

All outputs are JSON or binary blobs the deploy bundle can
serve as ordinary static files.

#### `forme-doc-search-client-js`
Single TypeScript file compiled to a single ~30KB minified JS
file.  Lazy-loaded on first user interaction (input focus),
not on page load.

Runtime:
1. Fetch manifest.
2. User types query → tokenize (same code as build-time).
3. For each query term, consult bloom filters → identify
   candidate shards.
4. Fetch only candidate shards (typically 1–3 per term).
5. Score matching documents with BM25.
6. Sort, dedupe, return top-K.
7. Fetch snippet shards for matched docs; highlight matched
   terms inline.

No external dependencies.  Uses `fetch` + `URLSearchParams` +
DOM APIs only.

### Site assembly

#### `forme-doc-site-emitter`
Top-level composer.  Takes:
- Per-page rendered content (from the content pipeline).
- Sidebar tree (from `forme-doc-sidebar-builder`).
- Search-engine outputs (shards + client JS + UI HTML).
- Site config (title, base URL, theme variables).

Produces a `PageBundleConfig` for
`forme-aot-page-bundle-emitter`.  Every doc page becomes a
route; every search shard becomes an `extraFile`.

This is the **glue package** that turns DOC00's per-stage
outputs into something the FM00 deploy chain consumes.

## 5. Search engine — detailed design

The search engine is the most novel piece.  Design notes:

### 5.1 Sharded inverted index

A monolithic inverted index for a 1000-page site is ~5MB
uncompressed.  Shipping all of that to every visitor on every
search is wasteful.

Solution: shard by first 1–2 characters of the term.  Each
shard:
- Maps `term → posting list` for terms starting with that
  prefix.
- Compressed (gzip Content-Encoding from the host).
- ~50–100KB per shard, ~26 shards for an ASCII corpus.

Client fetches **only the shards needed** for the active
query.  A 2-term query typically pulls 2–3 shards (~150KB
total) on the first run; subsequent searches reuse cached
shards.

### 5.2 Bloom filters per shard

Bloom filters answer "is this term possibly in this shard?"
with no false negatives.  Stored in the manifest (which is
small and fetched once).  Without bloom filters, the client
has to fetch every shard that *might* contain a query term;
with them, ~99% of irrelevant fetches are skipped.

Bloom filter parameters:
- ~1KB per shard at 1% false-positive rate.
- ~5KB manifest total for 26 shards.
- One round-trip for the manifest, then surgical shard fetches.

### 5.3 BM25 ranking

Standard text-retrieval relevance function — well-understood,
parameterisable.  Computed client-side in JS.  Inputs from
the inverted-index posting lists (term frequency in doc,
document length); doc count + average doc length from the
manifest.

### 5.4 Trigram fuzzy pre-filter

For typo tolerance (`"foramt" → "format"`), naïve Levenshtein
across all terms is too slow.  Trigram approach:

1. Index each term's set of trigrams (every 3-char window).
   `"format"` → `{for, orm, rma, mat}`.
2. Query: tokenize, decompose into trigrams.
3. Look up trigrams in a small index → candidate terms.
4. Compute Levenshtein only on candidates.
5. Substitute close-distance matches and re-run.

Extra ~10KB per 1000 docs.  Sub-millisecond client-side.

### 5.5 Why no WASM

Could use SIMD-accelerated WASM for ranking — but JIT-warm
V8 BM25 over a few hundred posting-list entries is already
under 10ms.  WASM saves ~5ms at the cost of an extra ~30KB
binary and a runtime initialisation step.  Not worth it.

### 5.6 Why no service worker

Could cache shards aggressively via service worker for offline
search — and we might add that in v1+.  For v0: browser HTTP
cache + the same `<link rel="prefetch">` hints we'd add to
any static site cover 95% of the benefit with zero code.

## 6. Aesthetic: RTD-minimal

User-confirmed choice.  Specifics:

- **Two columns**: sidebar (left, ~280px), content (centre,
  ~720px max-width for readability).  Optional third column on
  wide screens for in-page TOC.
- **Header bar**: site title, version selector slot (deferred
  for v0), GitHub link, search input.
- **Sidebar**: scrollable nav tree.  Active page highlighted.
  Expandable/collapsible groups.
- **Footer**: copyright, "edit this page on GitHub", build
  timestamp.
- **Typography**: system font stack (no web fonts in v0).
  Generous line-height, comfortable measure.
- **Colours**: light theme only in v0.  Neutral greys.  One
  accent colour configurable per site.
- **Code blocks**: monospace.  Copy button top-right of each
  block.  Language label top-left.  Subtle border + background.
- **Inline code**: monospace, lightly tinted background.
- **Tables**: zebra-striped.
- **Callouts** (note / warning / tip blocks from GFM):
  coloured left border + icon.

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
- Code block decoration + AOT syntax highlighting.
- Sidebar nav from directory layout.
- RTD-minimal two-column shell.
- **Client-side search** (the headline feature).
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
documented separately in FM05).

## 9. Integration with FM00

The terminal package, `forme-doc-site-emitter`, produces a
`PageBundleConfig` (the same input shape
`forme-aot-page-bundle-emitter` consumes).  Every doc page
becomes a route; every search shard becomes an `extraFile` with
its own content type (`application/json` or
`application/octet-stream`).

The resulting page bundle JSON flows through the FM00 chain
unchanged: `forme-aot-deploy-manifest-emitter` composes it
with sitemap.xml / robots.txt / extras, and the FM05 deploy
runner applies the result to whatever target.

Net effect: **docs sites are FM00 deployments**.  Same trust
boundaries, same atomicity guarantees, same content-addressed
hashes, same diff-mode incremental deploys.

## 10. Implementation order

Recommended PR sequence (each one independently testable):

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

Twelve PRs in total (counting this vision doc as #1).  At the
rate the FM00 v0 cluster shipped, that's ~12 babysit cycles.

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

## 12. Open design decisions for v0

Before implementation starts, a few decisions need confirming
(the user has answered most of these — repeating them here for
the spec record):

| Decision | v0 choice | Confirmed? |
|---|---|---|
| Markdown engine | use existing `commonmark-parser` / `gfm-parser` | ✓ |
| Advanced features | search engine ONLY (defer multi-version, i18n, MDX, dev server, dark mode) | ✓ |
| Theme aesthetic | RTD-minimal (two-column, no-frills) | ✓ |
| Timing | start after FM00 v0 finishes | ✓ |

Two remaining decisions to confirm during implementation:

1. **Syntax highlighter grammar set for v0.** Proposed:
   TypeScript, JavaScript, Python, Ruby, Go, Rust, Bash, JSON,
   HTML, CSS, Markdown.  Add others on demand.
2. **Search shard size budget.**  Proposed: 50–100KB
   compressed per shard, ~26 shards by alphabet prefix.
   Re-tune based on real-world corpus sizes once one site is
   built.
