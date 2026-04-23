# FM00 — Forme: A Universal Authoring Pipeline

> **Status:** Vision document. Not yet a build-ready spec.
> **Working name:** "Forme" — a printer's term for the locked block of type
> ready to go on the press. Provisional; can be renamed in one pass.
> **Spec prefix:** `FM`. Also provisional.
> **Scope:** Full vision, covering v0 through long-term. Companion per-package
> specs (`FM01…FMnn`) will be written as each package moves to implementation.

## 0. Preface

This document describes a system for authoring structured content once and
rendering it into any number of targets — blog posts on a CDN, a book as a
PDF, a documentation site with search, a newsletter for web and email, an
EPUB, a slide deck, a scientific paper typeset with LaTeX. The system is a
**typed DAG of composable packages**, deliberately designed so that what we
commonly call "a blog platform" or "a static site generator" or "a
typesetting system" are all configurations of the same underlying parts.

It is a companion to **BR01 Venture** (the browser). Where Venture turns URLs
into pixels, Forme turns authored content into URLs, print, email, or any
other medium.

---

## 1. The Observation Behind the Design

Look at WordPress, Substack, Squarespace, Jekyll, Astro, Docusaurus, Medium,
Blogger, Ghost, LaTeX, Typst, Pandoc, InDesign, Quark. They appear to be
wildly different products serving different audiences. They aren't.

Every one of them does the same six things:

1. **Source** — gather content from somewhere (a filesystem, a database, a
   hosted CMS, a git repo, a notebook)
2. **Parse** — turn raw input into a structured, typed representation
3. **Collect** — group and order individual documents (chronological posts,
   chaptered book, tagged topics, sidebar-ordered docs)
4. **Transform** — walk the structure and enrich it (syntax highlighting,
   table of contents, autolinks, cross-references, footnote resolution)
5. **Render** — produce the output medium from the enriched structure
6. **Emit** — deliver the rendered output to its destination

They differ in *where each stage runs*, *what intermediate representation the
stages agree on*, and *what backend stage 5 targets*. WordPress runs the full
pipeline per request against a MySQL source. Jekyll runs it once at build
time against a filesystem source. Substack runs it against a hosted source
and emits to web and email simultaneously. LaTeX runs it against `.tex` files
and emits PDF. Every product welded the pipeline to its backend and hid the
intermediate representations inside itself, and the result is that you cannot
reuse any piece across products. You buy the whole stack or none of it.

**Forme is the observation that this welding is an accident, not a
requirement.** If the content, style, and interactivity intermediate
representations are made explicit and shared, the pipeline becomes a
composition of small packages, and the backend becomes just another stage
you swap in. Every named product in the list above becomes a one-line
configuration, not a separate project.

The insight extends further: once the pipeline and the backend are separate,
the **smallest backend wins**. A page with no interactivity ships zero
bytes of JavaScript. A page with one interactive widget ships the bytes
for that widget and nothing more. The pipeline becomes a Ahead-Of-Time
compiler that emits the minimum artifact sufficient to serve a particular
piece of content on a particular medium.

---

## 2. Goals

1. **Separate content from presentation from behavior** via three
   independent, typed intermediate representations that all stages agree on.
2. **Compose small packages into arbitrary pipelines** so that a blog, a
   book, a docs site, a newsletter, or a scientific paper is a one-line
   configuration of the same parts.
3. **Target any backend** from the same upstream stages: HTML + CSS + JS
   islands for web, LaTeX for print-quality typesetting, direct PDF, EPUB
   for ebooks, HTML email for newsletters, terminal for CLI output, image
   for social cards.
4. **Ship the minimum artifact** the chosen backend needs. Pages with no
   interactivity get no JS. Pages with one island get the bytes for that
   island. Static content gets static files. Dynamic content gets handlers.
5. **Make the authoring surface pluggable** so first parties and third
   parties both extend it through the same API. The default editor is a
   plugin. Themes are plugins. Publish workflows are plugins.
6. **Keep the user's data in the user's hands.** Content lives where the
   user chose to put it (local filesystem, Google Drive, GitHub, Dropbox,
   S3), and passes browser → user's hosting without ever entering
   infrastructure Forme operates.
7. **Be useful to a developer and usable by a non-developer.** The same
   system powers a dev's GitHub-Pages-published blog and a parent's
   point-and-click personal site. The former drives the latter through
   dogfooding.
8. **Reuse the existing coding-adventures primitives** where they fit —
   `document-ast` as the Content IR, `directed-graph` as the orchestrator
   skeleton, `content_addressable_storage` for identity, `compiler-ir`
   patterns where applicable, `commonmark-parser` / `gfm-parser` /
   `asciidoc-parser` for parsing.

## 2.1 Non-Goals

1. **Not a full CMS-as-a-service.** Forme is a pipeline and an authoring
   tool, not a hosted multi-tenant platform (though a platform could be
   built on top). We never run customer infrastructure by default.
2. **Not a replacement for the browser.** Forme produces web output;
   Venture consumes it. They are complementary, not overlapping.
3. **Not a backend framework.** Forme can emit dynamic handlers for
   Cloudflare Workers or similar, but it is not a web framework for
   building apps. The sweet spot is **content-shaped systems**:
   documents, posts, articles, books, decks, reports.
4. **Not content-format preservation.** We do not try to losslessly
   round-trip every source format. The Content IR is a semantic
   representation; some source-specific fluff is deliberately dropped.
5. **Not real-time collaborative editing in v0/v1.** It is compatible with
   the architecture (via CRDT + E2EE relay) but is a significant
   undertaking and an explicit v2+ concern.

---

## 3. The Three Parallel Intermediate Representations

A document in Forme is a triple:

```
Document = (Content, Style, Interactivity)
```

Each element is its own typed IR. Each flows through its own stages in the
pipeline. Each is consumed by the backend, which decides how to realise or
omit it for the target medium. Separating them is the single most important
architectural decision in the system.

Why three? Because the dimensions of a document are independent: *what it
says* (content), *what it looks like* (style), and *what happens when you
touch it* (interactivity). Different backends care about different subsets.
Print cares about content and style but not interactivity. Terminal cares
about content with minimal style. Web cares about all three. A single monolithic
representation would force each backend to carry fields it cannot use
and would couple every change in one dimension to every other.

```
┌─────────────────────────────────────────────────────────────────────┐
│                     Content IR (document-ast)                       │
│    Paragraphs, headings, lists, code, images, tables, links, …      │
│    Format-agnostic. Semantic, not notational.                       │
└─────────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────────┐
│                     Style IR (new)                                  │
│    Design tokens, selectors, rules, media contexts, themes.         │
│    Compiles to CSS, LaTeX style commands, PDF style dicts, …        │
└─────────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────────┐
│                     Interactivity IR (new)                          │
│    Triggers, effects, state, bindings.                              │
│    Compiles to event listeners + JS, PDF form JS, EPUB navigation,  │
│    or nothing at all for print-only backends.                       │
└─────────────────────────────────────────────────────────────────────┘
```

### 3.1 Content IR — `document-ast` (exists)

The coding-adventures monorepo already contains a mature, MIT-licensed
Content IR in `@coding-adventures/document-ast`. Its design principles
match exactly what Forme needs:

1. Semantic, not notational — nodes carry meaning, not syntax
2. Resolved, not deferred — link references resolved before IR
3. Format-agnostic — `RawBlockNode` / `RawInlineNode` carry a `format` tag
4. Immutable and typed — all fields are `readonly`
5. Minimal and stable — only universal document concepts

Current node vocabulary:

- **Block nodes**: `document`, `heading`, `paragraph`, `code_block`,
  `blockquote`, `list` (ordered/unordered, tight/loose), `list_item`,
  `task_item`, `thematic_break`, `raw_block`, `table`, `table_row`,
  `table_cell`
- **Inline nodes**: `text`, `emphasis`, `strong`, `strikethrough`,
  `code_span`, `link`, `image`, `autolink`, `raw_inline`, `hard_break`,
  `soft_break`

**Forme adopts `document-ast` as the Content IR without modification.**
Where Forme needs concepts the current vocabulary does not cover —
embeds (YouTube, Twitter, custom widgets), callouts, pull quotes,
footnotes-with-backrefs, cross-references, transclusions — it adds them
through a disciplined extension mechanism described in §3.4, not by
forking the AST.

### 3.2 Style IR — new, to be designed

Style is not CSS. CSS is a serialization of style for one backend. The
Style IR is the abstract content that compiles *to* CSS, to LaTeX style
commands, to PDF style dictionaries, to RTF character/paragraph styles,
to terminal ANSI codes. It has to be higher-level than CSS or LaTeX to
survive being translated to either.

Proposed shape:

```typescript
type StyleDocument = {
  tokens:       TokenSet        // design tokens (colors, scales, families)
  rules:        StyleRule[]     // what properties apply to what
  contexts:     ContextGroup[]  // print-only, screen-only, dark-mode, etc.
  theme:        string | null   // named theme reference
}

type TokenSet = {
  colors:       Record<string, Color>
  typography:   { families, scale, weights, leading, tracking }
  space:        number[]        // spacing scale
  radii:        number[]        // corner-radius scale
  shadows:      Shadow[]
  // …
}

type StyleRule = {
  selector:     Selector        // "all heading level 2", "tag=warning", …
  properties:   StyleProperty[] // typed, backend-abstract
  context?:     string          // which ContextGroup this belongs to
}

type Selector =
  | { kind: "node-type"; type: BlockNode["type"] | InlineNode["type"] }
  | { kind: "node-type-level"; type: "heading"; level: number }
  | { kind: "custom-kind"; kind: string }   // e.g. "callout", "youtube-embed"
  | { kind: "tag"; tag: string }            // matches frontmatter tag
  | { kind: "id"; id: string }
  | { kind: "role"; role: string }          // semantic role
  | { kind: "nth"; of: Selector; n: number }
  | { kind: "and"; all: Selector[] }
  | { kind: "or"; any: Selector[] }

type StyleProperty =
  | { kind: "color";        value: ColorRef | Color }
  | { kind: "background";   value: ColorRef | Color }
  | { kind: "font-family";  value: FamilyRef | string }
  | { kind: "font-size";    value: ScaleRef | Length }
  | { kind: "font-weight";  value: WeightRef | number }
  | { kind: "leading";      value: number }
  | { kind: "space-before"; value: ScaleRef | Length }
  | { kind: "space-after";  value: ScaleRef | Length }
  | { kind: "indent";       value: ScaleRef | Length }
  | { kind: "column-break"; value: "before" | "after" | "avoid" }
  | { kind: "page-break";   value: "before" | "after" | "avoid" }
  | { kind: "align";        value: "start" | "end" | "center" | "justify" }
  // … extension points for custom properties declared by plugins
```

Each backend declares how it interprets each property kind. Backends MAY
ignore properties they cannot represent (a print backend ignores
`hover-color`; a terminal backend ignores `border-radius`). A property
that no backend understands is a build warning, not an error — it may be
consumed by a plugin backend not yet loaded.

The Style IR is deliberately **declarative and context-sensitive**, not
imperative. "What happens when you nest X inside Y" is expressed through
selectors, not through cascading-inheritance-as-side-effect the way CSS
does it. This makes it tractable to compile to LaTeX (which does not have
CSS-style cascade) without losing expressiveness.

### 3.3 Interactivity IR — new, to be designed

Interactivity is the smallest of the three IRs for most documents — many
posts have none at all — but it is the one that costs the most bytes when
it is present. Getting it right is what makes the AOT-compiler thesis
work: if interactivity is explicit in the IR, the compiler can know which
pages need any JavaScript and which do not.

Proposed shape:

```typescript
type InteractivityDocument = {
  state:        StateDeclaration[]    // named state variables
  bindings:     Binding[]             // state ↔ content/style
  handlers:     Handler[]             // trigger → effect rules
  capabilities: Capability[]          // what islands this document needs
}

type StateDeclaration = {
  name:         string
  scope:        "block" | "document" | "site" | "session"
  type:         "boolean" | "number" | "string" | "enum" | "list" | "object"
  initial:      unknown
  persist?:     "none" | "session" | "local"
}

type Binding = {
  when:         Predicate             // expression over state
  target:       NodeRef               // which block/attribute is affected
  apply:        BindingEffect         // hide, show, swap-class, swap-attr, …
}

type Handler = {
  on:           Trigger               // click, focus, visible, scroll, timer, custom
  target:       NodeRef
  effect:       Effect                // toggle-state, navigate, fetch, submit, …
}

type Trigger =
  | { kind: "click" }
  | { kind: "focus" }
  | { kind: "blur" }
  | { kind: "hover" }
  | { kind: "visible"; threshold?: number }
  | { kind: "scroll"; passed?: NodeRef }
  | { kind: "timer"; after: number }
  | { kind: "form-submit" }
  | { kind: "custom"; name: string }

type Effect =
  | { kind: "set-state";    state: string; to: ValueExpr }
  | { kind: "toggle-state"; state: string }
  | { kind: "navigate";     to: string }
  | { kind: "fetch";        url: string; into?: NodeRef | string }
  | { kind: "dispatch";     event: string; data?: unknown }
  | { kind: "run-island";   island: string; args?: unknown }
```

"Islands" are bundles of code contributed by plugins that implement
richer behavior — a search box, a comments widget, a commerce cart, a
syntax-highlighted live editor. The Interactivity IR points at islands
by name; each referenced island becomes a dependency the compiler
bundles. Pages with no handlers and no bindings need no runtime.

Print/PDF backends generally drop the entire Interactivity IR, or
realise a small subset (hyperlinks, PDF form fields, tab order).
Terminal backends drop all of it. Web backends realise the full surface.
EPUB backends realise navigation and minimal scripting if the reader
supports it.

### 3.4 Extending the IRs through plugins

The three IRs are fixed at the kernel. Plugins extend them through
**declared extension types** — new node kinds, new style properties,
new interactivity triggers — registered at manifest-load time:

```typescript
// A plugin manifest declaring a new block kind:
{
  name: "@forme/embed-youtube",
  extends: {
    content: [{
      kind: "content-block",
      type: "custom_embed",           // new BlockNode.type
      discriminant: "youtube",        // distinguishes from other custom_embed
      schema: { videoId: "string", start: "number?" }
    }]
  },
  capabilities: ["content:extend", "network:youtube-oembed"],
  entry: "./index.ts"
}
```

At parse time, the extension is visible to any parser that knows about
it — a `parse-markdown` plugin with YouTube-embed support produces a
`CustomEmbedNode { kind: "youtube", ... }`. At render time, the renderer
consults the plugin host for a handler matching `{ type: "custom_embed",
discriminant: "youtube", backend: "html" }`. If no handler is registered
for the current backend, the node falls back to its `alt` representation
(a link, in this case). This is how an extension stays graceful across
backends it does not yet support.

---

## 4. The Pipeline Architecture

### 4.1 Typed DAG, not linear pipe

Unix pipes are linear byte streams. A blog build is not linear — it fans
out (N posts → N parses), transforms in parallel, fans in (build indexes,
feeds, search), and emits multiple artifacts. The right abstraction is a
**typed directed acyclic graph** where each node is a stage, each edge is
a typed stream of values, and the orchestrator walks the DAG executing
stages in topological order with parallelism where possible.

```
                 ┌───────────────┐
                 │  source-fs    │
                 │  ContentSource│
                 └───────┬───────┘
                         │
                 ┌───────▼───────┐
                 │ parse-markdown │
                 │  ContentNode   │
                 └───────┬───────┘
                         │
      ┌──────────────────┼────────────────────┐
      │                  │                    │
┌─────▼──────┐   ┌───────▼──────┐    ┌────────▼──────┐
│ transform- │   │ collect-chr- │    │ collect-by-tag│
│ syntax-hl  │   │ onological   │    │ Collection[]  │
│ ContentNode│   │ Collection   │    └────────┬──────┘
└─────┬──────┘   └──────┬───────┘             │
      │                 │                     │
      └──────────┬──────┘                     │
                 │                            │
       ┌─────────▼────────┐                   │
       │  render-static    │                  │
       │  RenderedPage[]   │                  │
       └─────────┬─────────┘                  │
                 │                            │
      ┌──────────┼────────────┐               │
      │          │            │               │
┌─────▼───┐ ┌────▼──────┐ ┌───▼──────┐ ┌──────▼──────┐
│ emit-fs │ │ feed-rss  │ │ search-  │ │ render-tag-  │
│         │ │           │ │ pagefind │ │ pages        │
└─────────┘ └───────────┘ └──────────┘ └──────────────┘
```

This DAG has exactly the shape every SSG already runs internally; Forme
just makes it explicit and rewireable.

### 4.2 The stage interface

The contract every package implements:

```typescript
interface Stage<In extends Kind, Out extends Kind> {
  // Static identification
  readonly name:          string
  readonly version:       SemVer
  readonly apiVersion:    number              // pipeline contract version

  // Type contract
  readonly consumes:      KindDescriptor<In>  // what this stage takes in
  readonly produces:      KindDescriptor<Out> // what it emits
  readonly capabilities:  Capability[]        // what it's allowed to do

  // Execution
  run(input: In, ctx: StageContext): AsyncIterable<Out> | Out | Promise<Out>

  // Optional lifecycle hooks
  init?(ctx: StageContext): Promise<void>
  dispose?(ctx: StageContext): Promise<void>
}
```

**Invariants that make composition safe:**

1. **No hidden state.** `run` takes its input and context; produces its
   output. No singletons, no globals, no process-wide caches. If a stage
   needs a cache, it's in `ctx` and is typed.
2. **No ambient I/O.** A stage that reads from disk declares the
   `storage:read` capability and receives a storage adapter through
   `ctx.storage`. It never reaches for `fs.readFile` directly. Same for
   network, environment, time (for reproducible builds).
3. **Iterable output.** `AsyncIterable` allows streaming — `parse-markdown`
   can emit each `ContentNode` as it finishes rather than batching all N.
4. **Structural types over nominal.** `KindDescriptor` is structural;
   stages compose by type compatibility, not by identity. A `parse-mdx`
   that produces `ContentNode` is interchangeable with `parse-markdown`
   even though they are different packages.

### 4.3 Orchestrator

The orchestrator is the runtime that wires stages together and executes
the DAG. Given a **pipeline configuration** (YAML, TOML, JSON, or
TypeScript code), it:

1. Resolves every referenced stage package (local, registry, or git).
2. Validates type compatibility across every edge.
3. Builds the DAG, topological-sorts it, identifies parallelisable nodes.
4. Loads each plugin under its declared capability set.
5. Executes stages, streaming outputs where applicable.
6. Collects artifacts, reports errors with per-stage context.

Example configuration:

```yaml
# forme.config.yaml — an Astro-like blog
pipeline:
  - source-fs: { root: ./content }
  - parse-markdown: { gfm: true }
  - transform-syntax-highlight: { theme: github-dark }
  - transform-autolink-headings
  - collect-chronological: { route: "/posts/:slug" }
  - render-islands: { theme: "@myorg/theme-minimal" }
  - asset-image-optimize: { formats: [avif, webp] }
  - feed-rss: { out: /feed.xml }
  - search-pagefind
  - emit-fs: { out: ./dist }
```

Swap three stages, get a different product:

```yaml
# forme.config.yaml — a WordPress-like dynamic site
pipeline:
  - source-sqlite: { path: ./content.db }
  - parse-blocks
  - transform-syntax-highlight
  - render-dynamic                  # instead of render-islands
  - emit-cf-worker: { out: ./worker.js }
```

### 4.4 Content kinds — the universal vocabulary

The type system of the pipeline. Every stage declares its inputs and
outputs in this vocabulary.

| Kind              | Meaning                                                        |
| ----------------- | -------------------------------------------------------------- |
| `ContentSource`   | raw bytes + path + metadata from storage                       |
| `ContentNode`     | parsed `DocumentNode` + frontmatter + identity (doc-ast + meta)|
| `StyleDocument`   | style IR (§3.2)                                                |
| `Interactivity`   | interactivity IR (§3.3)                                        |
| `Document`        | `(ContentNode, StyleDocument, Interactivity)` triple           |
| `Collection`      | ordered set of `Document`s + grouping key                      |
| `Asset`           | image / video / font / binary with metadata and references     |
| `RenderedPage`    | HTML/CSS/JS bundle + metadata for one output page              |
| `PrintForme`      | backend-neutral composed page ready for a print backend        |
| `RequestHandler`  | executable handler for dynamic per-request rendering           |
| `SearchIndex`     | serialised search index (format depends on indexer)            |
| `Feed`            | serialised feed (RSS, JSON Feed, Atom, sitemap, …)             |
| `DeployArtifact`  | final shippable thing (files, bundle, handler, manifest)       |

This vocabulary is extensible. A plugin that adds EPUB support declares
a new kind `EpubPackage`; packages that consume or produce it opt in.
But the core set is small, stable, and shared.

---

## 5. Stage Categories

Each category is a family of packages with a shared contract. v0 ships
one or two per category; the architecture admits unlimited growth.

### 5.1 Sources — where content lives

| Package          | Produces         | Capability          |
| ---------------- | ---------------- | ------------------- |
| `source-fs`      | `ContentSource`  | `storage:read`      |
| `source-drive`   | `ContentSource`  | `network:google`    |
| `source-github`  | `ContentSource`  | `network:github`    |
| `source-dropbox` | `ContentSource`  | `network:dropbox`   |
| `source-s3`      | `ContentSource`  | `network:s3`        |
| `source-notion`  | `ContentSource`  | `network:notion`    |
| `source-sqlite`  | `ContentSource`  | `storage:read-db`   |
| `source-wordpress-xml` | `ContentSource` | `storage:read` |
| `source-ipfs`    | `ContentSource`  | `network:ipfs`      |

### 5.2 Parsers — bytes to structure

| Package              | Produces       | Consumes sources of type |
| -------------------- | -------------- | ------------------------ |
| `parse-markdown`     | `ContentNode`  | `*.md`                   |
| `parse-mdx`          | `ContentNode`  | `*.mdx`                  |
| `parse-asciidoc`     | `ContentNode`  | `*.adoc`                 |
| `parse-rst`          | `ContentNode`  | `*.rst`                  |
| `parse-html`         | `ContentNode`  | `*.html`                 |
| `parse-docx`         | `ContentNode`  | `*.docx`                 |
| `parse-org`          | `ContentNode`  | `*.org`                  |
| `parse-blocks-json`  | `ContentNode`  | `*.json` (Gutenberg etc.)|
| `parse-frontmatter`  | `ContentNode`  | combines with any parser |

All parsers produce `ContentNode` built on the existing
`document-ast`. Cross-parser parity is the reason to use a single IR.

### 5.3 Transforms — structure to structure

| Package                          | Purpose                                |
| -------------------------------- | -------------------------------------- |
| `transform-syntax-highlight`     | annotate `code_block` with tokens      |
| `transform-autolink-headings`    | add id + self-link to headings         |
| `transform-toc`                  | extract TOC, inject anchor points      |
| `transform-footnotes`            | resolve footnote refs, emit backrefs   |
| `transform-embeds`               | resolve `<iframe>`/oEmbed → node kind  |
| `transform-internal-links`       | rewrite `/slug` to resolved URLs       |
| `transform-image-rewrite`        | rewrite image paths to CDN URLs        |
| `transform-typography`           | smart quotes, dashes, ligatures        |
| `transform-math`                 | KaTeX/MathJax pre-render or tag        |
| `transform-shortcodes`           | expand `{{ shortcode }}` macros        |
| `transform-xref`                 | resolve cross-references between docs  |
| `transform-i18n`                 | apply translations by locale           |

### 5.4 Collectors — many documents to a structured whole

| Package                     | Strategy                                    |
| --------------------------- | ------------------------------------------- |
| `collect-chronological`     | date-sorted; output is `posts/YYYY/MM/...`  |
| `collect-by-tag`            | one collection per tag                      |
| `collect-by-author`         | one collection per author                   |
| `collect-by-sidebar`        | hierarchy from `sidebar.yaml`               |
| `collect-flat-routes`       | route = file path (Next-like)               |
| `collect-versioned`         | docs versioning (Docusaurus-like)           |
| `collect-book-outline`      | chapters + parts for a book backend         |
| `collect-deck-order`        | slide sequence for a slide deck             |
| `collect-graph-links`       | wiki-style backlinks                        |

### 5.5 Renderers — the backend attach point

This is where backends fork. Every renderer consumes a `Document`
(content + style + interactivity) and a `Collection` for site-level
context; they differ in what they produce.

| Package             | Produces              | Target medium                |
| ------------------- | --------------------- | ---------------------------- |
| `render-static`     | `RenderedPage`        | HTML only, no JS             |
| `render-islands`    | `RenderedPage`        | HTML + per-island JS         |
| `render-spa`        | `RenderedPage`        | Single-page app              |
| `render-dynamic`    | `RequestHandler`      | per-request server/worker    |
| `render-latex`      | `PrintForme`          | `.tex` files                 |
| `render-pdf`        | `PrintForme`          | direct PDF drawing ops       |
| `render-epub`       | `EpubPackage`         | EPUB 3 bundle                |
| `render-email`      | `EmailMessage`        | MIME multipart HTML+text     |
| `render-terminal`   | `TerminalBuffer`      | ANSI-coloured text           |
| `render-card`       | `ImageBuffer`         | social card image            |
| `render-deck-html`  | `RenderedPage`        | reveal.js-style web deck     |
| `render-deck-pdf`   | `PrintForme`          | PDF slide deck               |

Renderers are the only stage category that changes when you change the
output medium. Every upstream stage is unchanged. That is the
composability claim made real.

### 5.6 Asset processors

| Package                      | Purpose                              |
| ---------------------------- | ------------------------------------ |
| `asset-image-optimize`       | resize, AVIF/WebP, responsive srcset |
| `asset-image-placeholder`    | LQIP / BlurHash                      |
| `asset-video-transcode`      | HLS / DASH ladders                   |
| `asset-font-subset`          | subset WOFF2 to used glyphs          |
| `asset-sprite`               | SVG icon sprite                      |
| `asset-strip-exif`           | privacy: strip image metadata        |

### 5.7 Search, feeds, sitemaps, meta

| Package              | Produces       |
| -------------------- | -------------- |
| `search-pagefind`    | `SearchIndex`  |
| `search-minisearch`  | `SearchIndex`  |
| `search-semantic`    | `SearchIndex` (+ client embedder) |
| `search-sqlite-fts`  | `SearchIndex` (SQLite WASM)       |
| `feed-rss`           | `Feed`         |
| `feed-jsonfeed`      | `Feed`         |
| `feed-atom`          | `Feed`         |
| `feed-sitemap`       | `Feed`         |
| `feed-newsletter`    | `Feed` (email campaign) |
| `meta-opengraph`     | per-page meta tags |
| `meta-schema-org`    | JSON-LD       |

### 5.8 Emitters — delivery

| Package                | Target                           | Capability                |
| ---------------------- | -------------------------------- | ------------------------- |
| `emit-fs`              | local `dist/` directory          | `storage:write`           |
| `emit-cf-pages`        | Cloudflare Pages (Direct Upload) | `network:cloudflare`      |
| `emit-cf-worker`       | Cloudflare Worker deploy         | `network:cloudflare`      |
| `emit-github-pages`    | GitHub Pages push                | `network:github`          |
| `emit-netlify`         | Netlify Deploy API               | `network:netlify`         |
| `emit-s3`              | S3 / R2 / compatible             | `network:s3`              |
| `emit-ipfs`            | IPFS pin                         | `network:ipfs`            |
| `emit-email-campaign`  | ESP (Mailgun, Resend, SES)       | `network:email`           |
| `emit-print-server`    | CUPS / physical printer          | `storage:write`           |

### 5.9 Notifiers — after-emit hooks

| Package            | Purpose                        |
| ------------------ | ------------------------------ |
| `notify-email`     | send newsletter after publish  |
| `notify-rss-ping`  | notify aggregators             |
| `notify-webhook`   | generic HTTP callback          |
| `notify-mastodon`  | post to fediverse              |
| `notify-atproto`   | post to Bluesky                |

---

## 6. Backends — What "Output" Can Mean

A backend in Forme is a **configured subset of renderers and emitters**
that targets a particular medium. A single content graph can be emitted
to multiple backends simultaneously (the same post to web, RSS, email,
and a weekly PDF digest).

### 6.1 Web

The flagship backend. Consumes content + style + interactivity fully.
Produces HTML + CSS + per-island JS, optimised per page.

```
Content + Style + Interactivity
  → render-islands
  → asset-image-optimize
  → compiler/decide    (static | islands | dynamic per page)
  → compiler/bundle    (per-island JS + shared runtime)
  → compiler/css       (used-CSS extraction per page)
  → emit-fs            OR emit-cf-pages
```

**Output shape per page:**

```
index.html              pre-rendered HTML or dynamic handler stub
page-<hash>.css         only CSS referenced by this page
island-<name>-<h>.js    one bundle per declared island, hashed
runtime-<h>.js          shared hydration runtime (cross-page dedup)
assets/img-<h>.avif     responsive image variants
manifest.json           page → asset mapping, consumed at runtime
```

### 6.2 LaTeX → PDF

Consumes content + style (interactivity dropped). Produces `.tex`
files that a TeX engine (xelatex, lualatex) compiles to PDF.

```
Content + Style
  → render-latex
  → emit-fs               (writes .tex files)
  → post-build: xelatex   (external compiler)
```

Why LaTeX? Typesetting quality. Decades of fine-grained control over
kerning, microtypography, mathematics, floats, bibliographies. For
anything print-like where typography matters — books, papers, reports,
resumes — LaTeX remains the highest-quality target.

### 6.3 Direct PDF

Consumes content + style (interactivity dropped except PDF forms/links).
Produces PDF directly without a LaTeX intermediate, using the layout and
paint layers already present in the monorepo (`layout-block`,
`layout-to-paint`, and a PDF paint-VM to be written).

Tradeoff: less typographic refinement than LaTeX, but no external
compiler dependency, faster builds, and easier reproducibility.

### 6.4 EPUB

Consumes content + limited interactivity (navigation + basic scripting
for supported readers). Produces an EPUB 3 package — a zip of XHTML
content documents + CSS + package metadata.

### 6.5 Email

Consumes content + style + limited interactivity. Produces MIME
multipart `text/html` + `text/plain`, with email-safe CSS (inline
styles, tables for layout, no JS). Paired with `emit-email-campaign` for
an ESP or a `notify-email` for a mailing list backend.

### 6.6 Terminal

Consumes content + minimal style (ANSI colors + dim/bold). Produces a
buffer of styled text for CLI help, man pages, terminal-based readers.

### 6.7 Slides

Two sub-backends from the same upstream:

- **Web**: reveal.js-style deck — HTML + CSS + navigation JS. Interactive.
- **PDF**: one page per slide, handout-style. No interactivity.

### 6.8 Social card

Emits an image per post for link previews (OpenGraph, Twitter Card).
Uses a subset of style + content to compose a 1200×630 image via the
same layout + paint layers.

### 6.9 Dynamic server

Not a static backend. `render-dynamic` produces a `RequestHandler`
(function of `Request → Response`) that runs per request — for
WordPress-shaped use cases where the content source is live (database,
CMS API) and per-request personalisation is required.

### 6.10 Backend contract

Each backend declares:

```typescript
interface Backend {
  name:            string
  consumes:        ("content" | "style" | "interactivity")[]
  style_properties: Set<StylePropertyKind>   // which properties it supports
  interactivity:   Set<TriggerKind | EffectKind> // which it realises
  custom_handlers: Map<CustomKindKey, Renderer>   // for plugin-declared kinds
  fallback:        (node: Node) => Node | null    // graceful degradation
}
```

A `custom_embed` kind with `discriminant: "youtube"` is looked up in
`custom_handlers`. If absent, `fallback` is called — typically producing
a `LinkNode` to the embed URL, which every backend can render. This is
how the system stays honest across backends that grow at different paces.

---

## 7. The AOT Compiler

The pipeline's output is not the page. The page is what the compiler
*decides to emit* based on what the page actually uses. This is the
"smallest possible artifact" half of the design.

### 7.1 Per-page dependency analysis

For every page, the compiler walks the `Document` and collects:

- **Content kinds used** (every block type, every inline type, every
  custom kind)
- **Style rules touched** (transitive closure of selectors that match
  any node in the page)
- **Interactivity handlers and bindings referenced**
- **Islands required** (transitive from handler `effect.run-island`)
- **Assets referenced** (images, fonts, videos)

Output: a per-page **used-set** — what code, CSS, and assets the page
needs. This is the input to the bundler.

### 7.2 The decide pass

For each page the compiler chooses one of four output modes:

```
if page.interactivity.is_empty && page.content.is_pure_static:
    mode = STATIC_HTML           # zero JS
elif page.interactivity.all_islands_are_lazy:
    mode = STATIC_WITH_ISLANDS   # HTML + lazy-loaded JS on interaction
elif page.requires_client_routing or page.requires_client_state:
    mode = SPA_BUNDLE            # React/Vue/Svelte-style app bundle
elif page.requires_per_request_data:
    mode = DYNAMIC_HANDLER       # server function
```

The default path for a blog post is `STATIC_HTML`. An article that
embeds a comment widget becomes `STATIC_WITH_ISLANDS` — the comment
island lazy-loads on scroll into view. An admin dashboard would go
`SPA_BUNDLE`. A login page would go `DYNAMIC_HANDLER`.

### 7.3 Per-island JS bundling

Each declared island gets its own bundle. A page that references
islands `search` and `comments` gets two bundles. Shared runtime
(hydration core, state primitives) is extracted into a single
cross-page bundle and dedup'd by content hash. All bundles are
code-split at the island boundary; a page doesn't pay for an island
it doesn't use.

Size targets per island, enforced by build:

| Island                          | Target (gzipped) |
| ------------------------------- | ---------------- |
| Comments                        | < 15 KB          |
| Search (client-only, MiniSearch)| < 20 KB + index  |
| Search (Pagefind)               | < 10 KB + shards |
| Newsletter signup               | < 5 KB           |
| Code copy button                | < 1 KB           |
| Syntax highlighter (lazy load)  | < 30 KB          |
| Shared hydration runtime        | < 4 KB           |

These are budgets, not hopes. A build fails if an island exceeds its
budget and the overage is flagged for triage.

### 7.4 Used-CSS extraction

Style rules that no node in the page matches are dropped. The remaining
rules are concatenated, deduplicated, and minified. A per-page stylesheet
ships as `page-<hash>.css` in ~1–5 KB for a typical post. Fonts are
subsetted to the glyphs the page actually contains when `asset-font-subset`
is enabled.

### 7.5 Static-by-default is enforced, not aspirational

`render-islands` is the default renderer, *not* `render-spa`. A page
that can be static ships as static. A page that needs interactivity
ships the minimum JS for that interactivity. A page that needs full
client state opts into `render-spa` per-page, not site-wide.

This is the architectural inverse of Gatsby/Next/SvelteKit, which
default to hydrating the whole app and treat "no JS" as a special case.
Forme defaults to zero JS and treats hydration as a declared need.

---

## 8. The Plugin Model

### 8.1 Axioms — never plugins

Five things are kernel, not plugin. Making them pluggable would
introduce circularity, version-skew hell, or incompatibility between
otherwise-composable ecosystems.

1. **The plugin system itself** — manifest format, loader, lifecycle.
2. **The capability/permission model** — what plugins are allowed to do.
3. **The three IR schemas** — content, style, interactivity.
4. **The pipeline orchestrator** — stage contract, DAG executor, error
   boundaries.
5. **Content addressing and identity** — stable IDs across renames.

Everything else is a plugin, including the default editor, the default
theme, every renderer, every parser.

### 8.2 Extension points

Named slots where plugins attach. Each has a typed contract.

| Extension point        | What plugins contribute                        |
| ---------------------- | ---------------------------------------------- |
| `pipeline.stage`       | a new stage, declaring `consumes`/`produces`   |
| `content.kind`         | a new block or inline node kind                |
| `style.property`       | a new style property                           |
| `interactivity.trigger`| a new trigger                                  |
| `interactivity.effect` | a new effect                                   |
| `backend`              | a new render target                            |
| `editor.block`         | an editing UI for a content kind               |
| `editor.toolbar`       | buttons/dropdowns in named toolbar slots       |
| `editor.sidebar`       | sidebar panels                                 |
| `editor.command`       | keyboard-bindable commands                     |
| `editor.publish-stage` | a step in the publish workflow                 |
| `storage.adapter`      | a new storage backend                          |
| `deploy.adapter`       | a new deploy backend                           |
| `auth.provider`        | a new identity provider                        |
| `theme`                | a complete theme (style + default renderer)    |
| `island`               | a runtime JS/CSS bundle for interactive widget |

### 8.3 Plugin manifest v1

Versioned from day one. Breaking changes require a major bump and
plugins declare the `apiVersion` they target.

```toml
# plugin.toml

[plugin]
name        = "@forme/embed-youtube"
version     = "0.1.0"
api-version = 1
entry       = "./dist/index.js"
description = "YouTube embed block"
license     = "MIT"
homepage    = "https://github.com/foo/forme-youtube"

[requires]
forme   = ">=0.1"

[capabilities]
declared = ["content:extend", "network:youtube-oembed"]

[extends]

[[extends.content-kind]]
type          = "custom_embed"
discriminant  = "youtube"
schema        = "./schema/youtube.json"

[[extends.editor-block]]
for-kind      = "custom_embed"
for-disc      = "youtube"
component     = "./dist/editor.js#YoutubeEditor"

[[extends.backend-handler]]
for-kind      = "custom_embed"
for-disc      = "youtube"
for-backend   = "web"
handler       = "./dist/render-web.js#renderYoutubeWeb"

[[extends.backend-handler]]
for-kind      = "custom_embed"
for-disc      = "youtube"
for-backend   = "latex"
handler       = "./dist/render-latex.js#renderYoutubeLatex"
```

### 8.4 Capability model

Every capability a plugin wants to exercise is declared in the manifest.
The plugin host enforces the declaration at every call site. A plugin
without `network:*` cannot make HTTP requests. A plugin without
`storage:write` cannot write files. These are not gentlemen's agreements
— the plugin host intercepts the relevant APIs and throws for undeclared
capabilities.

Capabilities are hierarchical and namespaced:

```
storage:read
storage:write
storage:read-db
network:*
network:cloudflare
network:github
network:custom-domain.<domain>
telemetry:emit
editor:inject-ui
content:extend
filesystem:user-home      (strongly gated; requires explicit user grant)
system:shell               (never granted to third-party plugins)
```

The user sees requested capabilities like an OS app permission screen
when installing a plugin. A plugin that requests `network:*` raises a
warning: "this plugin can contact any internet host. Proceed?" The
default denial is strict; the user must explicitly grant.

### 8.5 First-party plugins built on the same API

The default editor is a plugin using `editor.*` extension points. The
default theme is a plugin using `theme` and `style.*`. The default
markdown parser is a plugin using `pipeline.stage`. If a first-party
feature cannot be expressed as a plugin on the public API, the API is
wrong and gets fixed — not given a private back-channel. This is the
discipline that separates VS Code (which survives and grows its
ecosystem) from WordPress (whose plugin system is a layer of filters
bolted over an opaque core).

### 8.6 Plugin supply chain

Plugins run in the user's editor and can access user data according to
their declared capabilities. This is a meaningful trust surface. v1+
measures:

- **Signed plugins.** Publishers sign releases; the host verifies the
  signature and warns on unsigned.
- **Pinned versions.** `forme.config.yaml` records exact versions with
  integrity hashes. Supply-chain attacks that replace a version get
  caught at load time.
- **Declared capabilities shown at install.** The user approves the
  capability set, not just the name.
- **Auditable code.** Plugins are open source by default convention (not
  enforceable, but a marketplace norm).
- **Sandboxing where practical.** Islands run in a worker with no DOM
  access except to their island region. Editor plugins run with reduced
  Node privileges.

---

## 9. Data and Privacy Posture

### 9.1 The user owns their data

Content — the posts, drafts, images, media — lives where the user
chose to put it. Local filesystem by default. Google Drive, GitHub,
Dropbox, S3, or other storage adapters on opt-in. **Forme does not
operate any infrastructure that stores user content.**

Content NEVER enters Forme-operated servers. In transit. On disk. In
memory. Not through a proxy, not through a passthrough. The only thing
that traverses Forme's servers is:

- Product telemetry (structured events, no content)
- Authorization decisions (signed URLs, tokens — no payload)
- Plugin metadata (registry entries, updates)
- Account identifiers (email, plan) — no content

### 9.2 How the deploy works without touching content

The standard deploy flow (web backend, Cloudflare Pages target):

```
┌──────────────┐
│   User's     │
│   browser    │ ─── builds the site locally ───┐
│ or desktop   │                                │
│   shell      │                                │
└──────┬───────┘                                │
       │                                        │
       │ (1) POST /authz/deploy                 │
       │     { namespace, destination }         │
       │                                        │
       ▼                                        │
┌──────────────┐                                │
│    Forme     │ ─── (2) signs a scoped, ───────┤
│    server    │     short-lived upload URL     │
│              │     (never sees payload)       │
└──────┬───────┘                                │
       │                                        │
       │ (3) returns signed URL                 │
       │                                        │
       ▼                                        │
┌──────────────┐                                │
│   Browser /  │ ─── (4) uploads directly to ───┘
│   Shell      │     Cloudflare over TLS with
│              │     the signed URL
└──────────────┘
```

Forme's server role: *authorizer*, not *courier*. It signs permission
slips; it does not carry the package.

### 9.3 Bring Your Own Cloudflare as a graduation path

The default (free / hobby tier) uses a Forme-operated Cloudflare
account. Forme does not look at user sites at rest but technically
could. Honest framing: "we never see it in transit, we don't read it
at rest, but admin access is technically possible."

A "Bring Your Own Cloudflare" path, available for any user who wants
it, connects their own Cloudflare account via OAuth and issues deploy
tokens scoped to their account. Forme has no access. This is the
promise that survives audit without footnotes.

### 9.4 Telemetry — owned by Forme, scoped

Forme collects product telemetry: feature usage, crash categories,
build times, plugin adoption. It does not collect content.

Disciplines that keep the line clean:

- **Allowlist schema, not free text.** Every telemetry event's fields
  are declared at compile time; anything not in the schema is dropped
  client-side.
- **Client-side scrubbing.** Anything leaving the browser is assumed
  logged forever. Never rely on server-side redaction.
- **Error reporting categorises on the client.** Clients emit
  `{ category, code }`, not raw error messages or stack traces.
- **Hashed install IDs**, not user-identifying data.
- **Published schema.** The full telemetry schema is in the public
  docs, as `telemetry-schema.md`. Users can audit.
- **Opt-out for aggregate; opt-in for anything individual.**

The telemetry pipeline itself is self-hosted on Cloudflare (Workers
Analytics Engine + R2 archive). No third-party analytics vendor ever
sees any event. This keeps the compliance surface minimal.

### 9.5 Plugin supply chain is a data surface

A plugin with `network:*` can phone home. If a third-party plugin
exfiltrates user content, Forme's BYO-data promise fails in the
public narrative even if legally it is the plugin author's failure.
Mitigations:

- Capabilities gate network access. `network:*` requires explicit user
  grant at install and raises a warning.
- Islands run sandboxed with no ambient storage / filesystem access.
- A first-party telemetry API (`forme.telemetry.emit`) routes plugin
  events through the same scrubbing + allowlist infrastructure.
  Plugins that want analytics use this by default, not `fetch`.

### 9.6 Collaborative editing is a deliberate non-feature of v0/v1

Realtime multi-user editing is incompatible with "content never touches
our servers" unless the relay is fully end-to-end-encrypted (Yjs +
encrypted relay, for example). It is possible but a significant
undertaking. The v0/v1 stance: no collaboration, no relay. If
collaboration ships in v2+, it ships as E2EE or not at all — not via a
relay that can read content. This decision is locked in now rather
than discovered as a breach later.

---

## 10. The Authoring Surface

### 10.1 What the editor is

The editor is the application that a human uses to create documents.
It runs locally (desktop app by default; web app as an option). It:

- Loads content from a storage adapter
- Provides block-based editing against the Content IR
- Provides style editing against the Style IR
- Provides interactivity editing against the Interactivity IR
- Runs the pipeline locally for live preview
- Triggers publish via an emitter

The editor is built on the plugin API. Its core is a tiny shell. Every
block type, every toolbar button, every sidebar panel is contributed by
plugins — including first-party ones.

### 10.2 Editor architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Editor Shell (tiny)                      │
│  window, menus, tab management, auto-save, auto-update      │
└─────────┬───────────────────────────────────┬───────────────┘
          │                                   │
          ▼                                   ▼
┌─────────────────────┐              ┌─────────────────────┐
│   Editor Core       │              │    Plugin Host       │
│   block primitives, │◄────────────►│   loader, sandbox,   │
│   selection, undo,  │              │   capability enforce │
│   slot registry     │              │                      │
└──────┬──────────────┘              └──────────┬───────────┘
       │                                        │
       ▼                                        ▼
┌─────────────────────┐              ┌─────────────────────┐
│  Plugins contribute:│              │   Pipeline runs     │
│  - block types      │              │   locally for live  │
│  - toolbar items    │              │   preview           │
│  - side panels      │              │                     │
│  - commands         │              │                     │
│  - publish stages   │              │                     │
└─────────────────────┘              └─────────────────────┘
```

### 10.3 Block editor

The block editor operates on the Content IR. Each block kind
registers a `editor.block` extension contributing:

- A **view** component (how the block looks in the editor)
- A **controls** component (the block-specific menu / handles)
- A **schema hook** (creating a new instance, validating, cloning)
- A **keymap hook** (Enter, Backspace, Tab behaviour)

The base set (paragraph, heading, list, image, code_block, blockquote,
table, embed) is shipped as `@forme/editor-default-blocks` — a
first-party plugin on the same API every third party would use.

### 10.4 Style editor

The style editor lives alongside the block editor, not on top of it.
It provides:

- A **design-token explorer** for the active theme (colors, type
  scale, spacing)
- **Per-block style controls** that write to the Style IR
- **Theme preview** — see the effect of a token change across the whole
  site
- **Source mode** — edit the Style IR directly as YAML/TOML/TS

No raw CSS. Raw CSS is a backend concern. The editor always speaks
Style IR and compiles to CSS at preview/build time.

### 10.5 Interactivity editor

The interactivity editor is the riskiest surface — it's where
non-technical users brush against programming. Two modes:

- **Canned islands**: pick from a library of first-party and
  third-party islands (comments, search, newsletter signup, share
  buttons). No code written. Configuration UI per island.
- **Declarative bindings** (v1+): a simple rule builder — "when
  clicked, toggle this block's visibility." Writes to the Interactivity
  IR; never exposes handwritten JavaScript in the primary UX.
- **Script mode** (v2+): for power users, a JS editor that authors
  custom islands. Scoped with capability declarations.

### 10.6 Live preview is the pipeline in debug mode

Preview does not cheat. It runs the real pipeline on the draft content,
only with caching aggressive enough to feel instant. This means what
you see in preview is what you get at publish — no surprises. An
additional benefit: local preview is a forcing function for pipeline
performance. If a build is slow, preview is slow. Pipeline caching is
thus first-class.

### 10.7 Desktop shell

Default packaging: **Tauri**. Reasons:

- Native filesystem access without Chrome-specific APIs
- Small binary (~10 MB vs Electron's ~100 MB)
- Cross-platform (macOS, Windows, Linux)
- Auto-update, code signing are solved
- The editor UI can still be web (TypeScript) — Tauri wraps it

A web-only mode is possible (PWA + OPFS + File System Access API) but
the feature parity is lower (no real filesystem watch, no native menus,
no OS integration). Desktop is the default for v1+.

### 10.8 "Mom can use it" — what that actually means

The mom-can-use-it bar decomposes into concrete UX requirements:

1. Installable in under five minutes. One download, one drag to
   Applications. No CLI.
2. First post without reading docs. The first-run experience picks a
   theme, shows an editor, lets you type, hit Publish.
3. Publish to `<name>.github.io` or `<name>.pages.dev` in one click,
   without touching git, tokens, DNS, or YAML.
4. Themes that look good without customization. Real typography, real
   line-heights, real spacing. Not the "free template" look.
5. Images via drag-drop from Finder, Photos, Safari. No "upload image"
   dialog.
6. Undo that works for everything, including across sessions.
7. Autosave. Crashes and quits never lose work.
8. Built-in help that answers the actual questions non-developers
   have: how do I change the font, how do I add a subscribe box, how do
   I share a link, how do I change the domain.

These are v1 targets, not v0. v0 is for the author of this spec; v1 is
when mom gets handed the keys.

---

## 11. Named Products as Configurations

The ultimate validation of the architecture: every named product is a
pipeline configuration, not a separate codebase.

### 11.1 Jekyll / Hugo

```
source-fs → parse-markdown → transform-syntax-highlight
         → collect-chronological + collect-by-tag
         → render-static → emit-fs
```

### 11.2 Eleventy (any template)

```
source-fs → parse-markdown → parse-frontmatter
         → collect-chronological
         → render-static (theme with Nunjucks / Liquid / Handlebars)
         → emit-fs
```

### 11.3 Gatsby

```
source-fs → parse-mdx → data-layer-graphql
         → collect-flat-routes
         → render-spa → asset-image-optimize → emit-fs
```

(Note: `data-layer-graphql` is one possible collector that exposes the
collection as a typed GraphQL schema. Gatsby's novelty is the data
layer, not the rendering — Forme can reproduce it if someone wants.)

### 11.4 Next.js SSG

```
source-fs → parse-mdx → collect-flat-routes
         → render-spa → emit-fs
```

### 11.5 Astro

```
source-fs → parse-mdx → transform-syntax-highlight
         → collect-chronological
         → render-islands → asset-image-optimize → emit-fs
```

### 11.6 Docusaurus

```
source-fs → parse-mdx → transform-versioning → transform-xref
         → collect-by-sidebar
         → render-spa → search-algolia → emit-fs
```

### 11.7 WordPress

```
source-sqlite (or source-mysql)
         → parse-blocks → transform-syntax-highlight
         → render-dynamic → emit-cf-worker
         (+ runtime islands for comments, forms, search)
```

### 11.8 Substack

```
source-hosted → parse-blocks
         → collect-chronological
         → render-static (for web)
         → render-email (for newsletter — same content, different backend)
         → emit-cf-pages (web)
         → emit-email-campaign (email)
         → notify-email subscribers
         (+ paywall island for gated posts)
```

Note how the **same content graph** produces **two outputs** through
**two renderers and two emitters** — web and email — without
duplicating content sources. That is the multi-backend claim made real
on a recognisable product shape.

### 11.9 Ghost

```
source-sqlite → parse-blocks
         → collect-chronological
         → render-static + render-email
         → emit-fs + emit-email-campaign
         (+ members island, paywall island)
```

### 11.10 Medium

```
source-hosted → parse-blocks
         → collect-chronological + collect-graph-links
         → render-spa → emit-hosted
         (+ clap, follow, paywall islands)
```

### 11.11 Squarespace

```
source-hosted → parse-visual-builder
         → render-islands → emit-hosted
         (editor: visual drag-drop instead of block editor)
```

### 11.12 LaTeX academic paper

```
source-fs → parse-markdown → parse-frontmatter
         → transform-math → transform-xref → transform-citations
         → collect-book-outline
         → render-latex → emit-fs
         → post-build: xelatex main.tex
```

### 11.13 Book (PDF via LaTeX)

Same as 11.12 with `collect-book-outline` partitioned into chapters and
parts.

### 11.14 EPUB ebook

```
source-fs → parse-markdown → transform-footnotes
         → collect-book-outline
         → render-epub → emit-fs
```

### 11.15 Slide deck (web + PDF handout from the same source)

```
source-fs → parse-markdown → transform-syntax-highlight
         → collect-deck-order
         → render-deck-html + render-deck-pdf
         → emit-fs (for both)
```

### 11.16 Newsletter digest (PDF)

```
source-fs → parse-markdown → collect-chronological
         → filter-last-week
         → render-latex → emit-fs
         → post-build: xelatex → ship PDF
```

### 11.17 Personal wiki / digital garden (Obsidian-shaped)

```
source-fs → parse-markdown → transform-wiki-links
         → collect-graph-links
         → render-static → search-minisearch → emit-fs
         (+ backlinks island, graph view island)
```

### 11.18 Terminal man-page

```
source-fs → parse-markdown → render-terminal → emit-fs
```

Every one of these is a valid, buildable configuration. The diversity
you see across the "different" products is ninety percent in the
renderer and ten percent in the collector. Everything upstream is
shared.

---

## 12. Package Inventory

The full ~50-package inventory for the vision, grouped by layer. This is
the "definition of done" boundary — everything that needs to exist to
realize the full vision. v0 ships a small subset (see §15).

Column meaning:
- **Layer**: where it sits (kernel, IR, adapter, stage, runtime, editor,
  shell, default, compiler)
- **Plugin?**: whether it's loaded through the plugin host (most are)
- **v0?**: in the initial cut

### 12.1 Kernel — never plugins

| Package              | Purpose                                          | v0? |
| -------------------- | ------------------------------------------------ | --- |
| `forme-types`        | all shared TypeScript types                      | ✓   |
| `forme-stage`        | `Stage<In, Out>` interface + kind registry        | ✓   |
| `forme-plugin-host`  | manifest loader, capability enforcer, lifecycle  | ✓   |
| `forme-orchestrator` | DAG builder + executor + error boundaries        | ✓   |
| `forme-identity`     | content-hash IDs, stable across renames          | ✓   |
| `forme-capability`   | capability model + enforcement points            | ✓   |
| `forme-event-bus`    | typed pub/sub between stages and plugins         |     |

### 12.2 IR — schemas, not code

| Package                      | Purpose                           | v0? |
| ---------------------------- | --------------------------------- | --- |
| `document-ast` (existing)    | Content IR                        | ✓   |
| `forme-style-ir`             | Style IR types + validator        | ✓   |
| `forme-interactivity-ir`     | Interactivity IR types + validator|     |
| `forme-collection-ir`        | Collection / grouping types       | ✓   |
| `forme-asset-ir`             | Asset metadata + refs             | ✓   |
| `forme-pipeline-config-ir`   | Pipeline config schema             | ✓   |

### 12.3 Adapters — interfaces

| Package              | Purpose                                | v0? |
| -------------------- | -------------------------------------- | --- |
| `forme-storage`      | storage adapter interface              | ✓   |
| `forme-storage-fs`   | local filesystem impl                  | ✓   |
| `forme-storage-drive`| Google Drive impl                      |     |
| `forme-storage-github`| GitHub repo impl                      |     |
| `forme-storage-dropbox`| Dropbox impl                         |     |
| `forme-storage-s3`   | S3 / R2 impl                           |     |
| `forme-deploy`       | deploy adapter interface               | ✓   |
| `forme-deploy-fs`    | local `dist/` impl                     | ✓   |
| `forme-deploy-cf-pages`| Cloudflare Pages Direct Upload       |     |
| `forme-deploy-cf-worker`| Cloudflare Worker deploy            |     |
| `forme-deploy-github-pages`| GitHub Pages push                |     |
| `forme-auth`         | auth provider interface                 |     |
| `forme-auth-google`  | Google OAuth                            |     |
| `forme-auth-github`  | GitHub OAuth                            |     |

### 12.4 Stages

**Sources:**

| Package                   | v0? |
| ------------------------- | --- |
| `forme-source-fs`         | ✓   |
| `forme-source-drive`      |     |
| `forme-source-github`     |     |
| `forme-source-sqlite`     |     |
| `forme-source-wordpress-xml` |  |

**Parsers:**

| Package                                    | v0? |
| ------------------------------------------ | --- |
| `forme-parse-markdown`                     | ✓   |
|   (reuses existing `commonmark-parser` + `gfm-parser`) |  |
| `forme-parse-frontmatter`                  | ✓   |
| `forme-parse-mdx`                          |     |
| `forme-parse-asciidoc` (reuses `asciidoc-parser`) | |
| `forme-parse-html`                         |     |
| `forme-parse-docx`                         |     |
| `forme-parse-blocks-json`                  |     |

**Transforms:**

| Package                                | v0? |
| -------------------------------------- | --- |
| `forme-transform-syntax-highlight`     | ✓   |
| `forme-transform-autolink-headings`    | ✓   |
| `forme-transform-toc`                  |     |
| `forme-transform-footnotes`            |     |
| `forme-transform-embeds`               |     |
| `forme-transform-internal-links`       | ✓   |
| `forme-transform-image-rewrite`        | ✓   |
| `forme-transform-typography`           |     |
| `forme-transform-math`                 |     |
| `forme-transform-shortcodes`           |     |
| `forme-transform-xref`                 |     |
| `forme-transform-i18n`                 |     |

**Collectors:**

| Package                              | v0? |
| ------------------------------------ | --- |
| `forme-collect-chronological`        | ✓   |
| `forme-collect-by-tag`               |     |
| `forme-collect-by-author`            |     |
| `forme-collect-by-sidebar`           |     |
| `forme-collect-flat-routes`          |     |
| `forme-collect-versioned`            |     |
| `forme-collect-book-outline`         |     |
| `forme-collect-deck-order`           |     |
| `forme-collect-graph-links`          |     |

**Renderers:**

| Package                   | v0? |
| ------------------------- | --- |
| `forme-render-static`     | ✓   |
|   (reuses existing `document-ast-to-html`) | |
| `forme-render-islands`    |     |
| `forme-render-spa`        |     |
| `forme-render-dynamic`    |     |
| `forme-render-latex`      |     |
| `forme-render-pdf`        |     |
|   (reuses `document-ast-to-layout` → layout → paint → PDF) | |
| `forme-render-epub`       |     |
| `forme-render-email`      |     |
| `forme-render-terminal`   |     |
| `forme-render-card`       |     |
| `forme-render-deck-html`  |     |
| `forme-render-deck-pdf`   |     |

**Asset processors:**

| Package                        | v0? |
| ------------------------------ | --- |
| `forme-asset-image-optimize`   | ✓   |
| `forme-asset-image-placeholder`|     |
| `forme-asset-video-transcode`  |     |
| `forme-asset-font-subset`      |     |
| `forme-asset-strip-exif`       |     |

**Search / feeds / meta:**

| Package                    | v0? |
| -------------------------- | --- |
| `forme-search-pagefind`    |     |
| `forme-search-minisearch`  |     |
| `forme-search-semantic`    |     |
| `forme-feed-rss`           | ✓   |
| `forme-feed-jsonfeed`      |     |
| `forme-feed-atom`          |     |
| `forme-feed-sitemap`       | ✓   |
| `forme-meta-opengraph`     | ✓   |
| `forme-meta-schema-org`    |     |

**Emitters:**

| Package                    | v0? |
| -------------------------- | --- |
| `forme-emit-fs`            | ✓   |
| `forme-emit-cf-pages`      |     |
| `forme-emit-cf-worker`     |     |
| `forme-emit-github-pages`  |     |
| `forme-emit-netlify`       |     |
| `forme-emit-s3`            |     |
| `forme-emit-email-campaign`|     |

**Notifiers:**

| Package                  | v0? |
| ------------------------ | --- |
| `forme-notify-email`     |     |
| `forme-notify-rss-ping`  |     |
| `forme-notify-webhook`   |     |
| `forme-notify-mastodon`  |     |
| `forme-notify-atproto`   |     |

### 12.5 Client runtime

| Package                   | Purpose                        | v0? |
| ------------------------- | ------------------------------ | --- |
| `forme-runtime-islands`   | hydration core (< 4 KB gzipped)|     |
| `forme-runtime-router`    | optional client router         |     |
| `forme-runtime-state`     | state primitives               |     |
| `forme-runtime-loader`    | lazy-load additional islands   |     |

### 12.6 Editor

| Package                         | Purpose                    | v0? |
| ------------------------------- | -------------------------- | --- |
| `forme-editor-core`             | block editing primitives   |     |
| `forme-editor-slots`            | UI slot registry           |     |
| `forme-editor-preview`          | pipeline-backed live preview|    |
| `forme-editor-workflow`         | draft → review → publish   |     |
| `forme-editor-block-style`      | block-level style controls |     |
| `forme-editor-interactivity`    | interactivity rule builder |     |

### 12.7 Shell

| Package              | Purpose                          | v0? |
| -------------------- | -------------------------------- | --- |
| `forme-cli`          | headless build/deploy CLI        | ✓   |
| `forme-shell-desktop`| Tauri app wrapping editor        |     |
| `forme-dev-server`   | local HTTP preview + hot reload  | ✓   |

### 12.8 Defaults (first-party plugins)

| Package                          | Plugin? | v0? |
| -------------------------------- | ------- | --- |
| `forme-default-blocks`           | ✓       |     |
| `forme-default-theme`            | ✓       | ✓   |
| `forme-default-editor`           | ✓       |     |
| `forme-default-capabilities`     | ✓       | ✓   |

### 12.9 Compiler

| Package                        | Purpose                          | v0? |
| ------------------------------ | -------------------------------- | --- |
| `forme-compiler-analyze`       | per-page dep analysis            | ✓   |
| `forme-compiler-decide`        | choose output mode per page      | ✓   |
| `forme-compiler-bundle`        | per-island JS bundling           |     |
| `forme-compiler-css`           | used-CSS extraction              | ✓   |
| `forme-compiler-emit`          | per-page asset manifest          | ✓   |

---

## 13. Relationship to Existing Coding-Adventures Packages

Forme is deliberately additive. It builds on existing primitives where
they already do the job and introduces new packages only where there is
genuinely no existing equivalent.

### 13.1 Direct reuse (no new package needed)

| Existing                                   | Role in Forme                          |
| ------------------------------------------ | -------------------------------------- |
| `document-ast`                             | Content IR, unchanged                  |
| `document-ast-to-html`                     | core of `forme-render-static`          |
| `document-ast-to-layout`                   | core of `forme-render-pdf` pipeline    |
| `document-ast-sanitizer`                   | security layer for user-authored HTML  |
| `commonmark-parser`                        | inside `forme-parse-markdown`          |
| `gfm-parser`                               | inside `forme-parse-markdown`          |
| `asciidoc-parser`                          | inside `forme-parse-asciidoc`          |
| `directed-graph`                           | skeleton of `forme-orchestrator`       |
| `content_addressable_storage`              | identity / cache in `forme-identity`   |
| `compiler-ir` patterns                     | design reference for IRs               |
| `compiler-source-map`                      | preview → editor source-mapping        |
| `format-doc`, `format-doc-to-paint`        | contributes to print backend           |

### 13.2 New packages needed

Everything in §12 without an "existing" mapping above. The big-ticket
new packages are: the kernel six, the Style and Interactivity IRs, the
stage interface and orchestrator, the AOT compiler passes, the backend
renderers beyond `-static`, the editor stack, and the Tauri shell.

### 13.3 Relationship to BR01 Venture

Venture and Forme are complementary:

- Venture turns URLs into pixels (browser).
- Forme turns content into URLs-and-pixels-and-PDF-and-email (author).
- The web output of Forme is consumed by Venture.
- The pure-HTML backend (`render-static`) ensures Venture can render
  Forme output even at Mosaic-1.0 feature parity.

Both share the "thin orchestrator wiring small crates" architecture
and should share packages where the responsibilities overlap (`document-ast-to-html`,
the `document-ast` itself, the layout / paint layers for PDF rendering).

---

## 14. Security Model

### 14.1 Threat model

| Threat                                       | Mitigation                      |
| -------------------------------------------- | ------------------------------- |
| Malicious third-party plugin exfiltrates data| Capability model + sandbox      |
| Supply-chain attack (compromised package)     | Pinned versions + sigs + hashes |
| XSS via authored content                     | `document-ast-sanitizer`        |
| XSS via theme CSS / JS                       | CSP + theme sandboxing          |
| User-uploaded asset with EXIF privacy leak   | `asset-strip-exif`              |
| Credential exfiltration through error reports| Client-side scrubbing           |
| CSRF against deploy endpoints                | Signed short-lived URLs         |
| Session hijacking                            | Tokens in browser only + PKCE    |
| Plugin with arbitrary FS access              | `filesystem:*` requires grant   |

### 14.2 Capability enforcement points

A capability is meaningless unless the host actually intercepts the
operation. The capability host provides wrapped versions of:

- `fetch` (gated on `network:<host>` declarations)
- Storage reads/writes (gated on `storage:*`)
- File-system access (gated on `filesystem:*`)
- Environment / secrets (gated on `env:*`)
- Clock reads (gated on `system:time` in strict builds for reproducibility)
- Shell execution (not granted to third-party plugins, period)

Any plugin code that tries `globalThis.fetch` directly gets an error
with the message "plugin X did not declare `network:*` — if this plugin
needs network access, update its manifest and inform the user."

### 14.3 Content sanitization

Every piece of user-authored `raw_block` / `raw_inline` with
`format: "html"` passes through `document-ast-sanitizer` before
rendering. Themes that embed third-party scripts declare the script in
their manifest and a CSP directive is generated automatically. Inline
event handlers (`onclick`) are always stripped from user content; the
Interactivity IR is the supported way to attach behavior.

---

## 15. Performance Goals

These are not "eventually" hopes; they are budgets enforced by CI.

### 15.1 Build times

| Site size            | Cold build | Warm (incremental) |
| -------------------- | ---------- | ------------------ |
| 10 posts             | < 500 ms   | < 100 ms           |
| 100 posts            | < 2 s      | < 200 ms           |
| 1000 posts           | < 20 s     | < 500 ms           |
| 10k posts            | < 3 min    | < 2 s              |

Incremental builds use content-addressed caching — a stage re-runs only
when any input changes. The DAG orchestrator is the right scope for this
cache.

### 15.2 Page weights (web backend)

For a typical blog-post page, before the compiler's per-island work:

| Asset                  | Target           |
| ---------------------- | ---------------- |
| HTML                   | 5–20 KB          |
| Per-page CSS           | 1–5 KB           |
| Hydration runtime      | < 4 KB (shared)  |
| Per-island JS          | see §7.3         |
| Hero image (AVIF)      | < 100 KB         |
| Total (cold load)      | < 150 KB         |
| Total (warm, shared)   | < 30 KB          |

### 15.3 First contentful paint

Target: < 800 ms on a mid-range phone over 4G for cold load. This is a
direct consequence of static-by-default + per-page CSS + responsive
image optimization. Achievable at this budget; the default theme's job
is to not ruin it.

### 15.4 Editor responsiveness

| Action                           | Target               |
| -------------------------------- | -------------------- |
| Keystroke → character rendered   | < 16 ms (60 fps)     |
| Block insertion                  | < 50 ms              |
| Preview refresh                  | < 300 ms             |
| Publish button → deploy complete | < 15 s for 100-post site |

---

## 16. Roadmap

Phased to minimize the cost of being wrong about the architecture
before it's validated by real use.

### 16.1 v0 — Prove the pipeline (headless; dogfood own blog)

**Scope:** the kernel, the Content IR (reused), the Style IR (v1 schema
only, no theme customization UI), enough stages to produce a blog from
markdown, a CLI, a filesystem emitter, and a minimal default theme.
Produces a static site that powers the author's own blog.

**Out of scope for v0:** editor, desktop shell, interactivity IR
realization (stubs only), cloud storage or deploy adapters, multi-backend
rendering, search, feeds beyond RSS, plugin marketplace.

**Packages (v0 cut):** see §12, items marked ✓.

**Success criterion:** the author's current WordPress blog is replaced
with a Forme build, posted and live, with Lighthouse scores equal or
better across every axis.

### 16.2 v1 — Editor + plugin system (hand keys to mom)

**Scope:** Tauri shell, block editor, live preview, a themed default
editor experience, plugin host with the v1 capability model, first-party
plugins for the default block set / theme / editor, Cloudflare Pages
deploy adapter, minimal publish workflow.

**Success criterion:** mom installs the app, picks a theme, writes and
publishes a post to `<name>.pages.dev` in under thirty minutes with
zero help.

### 16.3 v2 — Multi-backend + authoring depth

**Scope:** `render-latex`, `render-pdf`, `render-epub`, `render-email`,
slide deck backends, book-outline collector, style editor with live
token editing, interactivity rule builder, more storage adapters (Drive,
GitHub), more deploy adapters (GitHub Pages, Netlify).

**Success criterion:** a book manuscript authored in Forme builds to
PDF and EPUB from one content source; a Substack-shaped
newsletter+web site runs as a single Forme config.

### 16.4 v3 — Ecosystem + BYO everything

**Scope:** plugin marketplace (discovery, signatures, reviews),
Bring-Your-Own-Cloudflare flow, third-party storage adapters at
parity with first-party, realtime collaborative editing via E2EE CRDT,
mobile companion app.

**Success criterion:** third-party plugins in the wild, powering sites
the core team did not build.

---

## 17. Open Questions

Explicit list of decisions deliberately deferred.

1. **Style IR property vocabulary.** §3.2 sketches the shape but not
   the exhaustive list. The right move is to define the vocabulary
   empirically from the first real themes and the first real LaTeX
   backend target, rather than by committee.
2. **Interactivity IR expression language.** §3.3 shows `Predicate`
   and `ValueExpr` abstractly. The actual expression language (JSON
   Logic? S-expressions? a tiny typed DSL?) is v1.
3. **Pipeline config format.** YAML in examples here, but TOML, JSON,
   and TypeScript are all candidates. Probably support all three with
   one canonical form (TOML?) — decide when writing the orchestrator.
4. **Plugin registry mechanism.** npm for v0 (plugins are just npm
   packages). A Forme-specific registry with reviews and signatures
   for v3. In between (v1/v2) probably still npm with a Forme-specific
   index layered over it.
5. **Editor technology choice.** Tiptap (ProseMirror) vs Lexical vs
   BlockNote vs custom. Decide when starting `forme-editor-core`.
   Preference: Tiptap for ecosystem, with the block-level structure
   mapped to the Content IR at the edges.
6. **Preview runtime.** In-editor preview — does it run the pipeline
   in a worker, or does it render via a local dev server? Probably
   the latter for fidelity; the former as an optimization.
7. **v0 blog style.** What's the default theme's aesthetic? Not a
   technical question but shapes first impressions. Personal
   preference of the author applies; this is a `hello-world` aesthetic
   decision more than an architecture one.
8. **Non-TypeScript implementation partners.** Some pieces (PDF
   renderer, maybe the orchestrator's hot path) would benefit from a
   Rust implementation compiled to WASM. Decide when those pieces are
   bottlenecks, not before.
9. **Plugin-defined content kinds crossing backend boundaries.** A
   `youtube-embed` block renders fine in HTML, but what about LaTeX?
   The extension mechanism in §3.4 handles it structurally; the
   editorial question is which defaults the framework ships vs. leaves
   to plugins.

---

## 18. Success Criteria

The vision is realized when each of these is true:

1. **The author's own blog runs on Forme**, dogfooded continuously.
2. **A non-developer** (the author's mom) installs Forme, writes a
   post, and publishes without help.
3. **Every named product in §11** can be expressed as a valid Forme
   pipeline configuration — and at least three of them (blog, book,
   newsletter) are actually used that way.
4. **Third-party plugins exist in the wild**, demonstrating the
   extension API is complete and stable enough to build on.
5. **No user content has ever resided on Forme-operated
   infrastructure.** The BYO-data promise remains intact as a matter
   of architecture, not discipline.
6. **Page weights for a typical post load under 150 KB cold**,
   consistently, without per-post optimization work.
7. **Build times scale**: a 1000-post site builds in under 20 seconds
   cold, under 500 ms incremental.
8. **Multi-backend proven**: one content graph produces, from the
   same `forme.config.yaml`, a web site, an email newsletter, an RSS
   feed, and a PDF digest.

---

## Appendix A — Prior Art (Acknowledged and Deliberate)

Forme takes from each of these; in most cases, it generalizes what
each did in its own silo.

- **[Unified / Remark / Rehype](https://unifiedjs.com/)** — the
  document pipeline at document scope. Forme lifts the same model
  to whole-site scope and adds Style and Interactivity parallel IRs.
- **[Pandoc](https://pandoc.org/)** — the N × M → N + M insight via
  a single document IR. `document-ast` already adopts this; Forme
  extends it across more backends.
- **[Astro](https://astro.build/)** — islands architecture, multi-framework
  rendering. Forme generalizes islands across backends and makes the
  pipeline user-configurable rather than framework-welded.
- **[Eleventy](https://www.11ty.dev/)** — Unix-like philosophy at
  document scope, template-language agnosticism. Forme inherits the
  philosophy at site scope with stronger types.
- **[Contentlayer](https://contentlayer.dev/)** — content-as-typed-data
  layer. Forme's Collection IR serves the same role with broader shape.
- **[Gatsby](https://www.gatsbyjs.com/)** — GraphQL data layer. Forme
  admits a GraphQL collector as one option; does not privilege it.
- **[Obsidian](https://obsidian.md/)** — local-first, files-as-truth,
  plugin-everything. Forme matches the data posture and extends the
  plugin discipline.
- **[VS Code](https://code.visualstudio.com/)** — plugin API same as
  internal API, versioned carefully. Forme adopts this discipline.
- **[Typst](https://typst.app/)** — modern typesetting. Inspiration for
  the Style IR semantics, not an implementation dependency.
- **[LaTeX](https://www.latex-project.org/)** — print-quality typesetting
  target. Forme's LaTeX backend rides on xelatex/lualatex.
- **[Blot.im](https://blot.im/)**, **Smallvictori.es** — the
  "storage-is-your-Dropbox" pattern. Forme admits this as a storage
  adapter configuration.
- **[Pagefind](https://pagefind.app/)** — sharded client-side search.
  First-class `forme-search-pagefind` plugin.

## Appendix B — Glossary

- **Backend** — in Forme, the configured output target (web, LaTeX,
  PDF, EPUB, email, …). *Not* a server-side runtime in the web-framework
  sense.
- **Content IR** — the `document-ast` intermediate representation for
  structured content.
- **DAG** — directed acyclic graph. The shape of a Forme pipeline.
- **Document** — the `(content, style, interactivity)` triple for a
  single authored unit.
- **Extension point** — a named slot where plugins attach. Typed.
- **Forme** — the system described in this document (working name).
- **Island** — a region of an output page that has interactivity, and
  the JS/CSS bundle that provides it.
- **Kernel** — the set of non-plugin primitives (see §8.1).
- **Orchestrator** — the runtime that executes the pipeline DAG.
- **Pipeline** — a configured sequence of stages forming a DAG.
- **Plugin** — a package contributed through the plugin host and
  extension points.
- **Stage** — a single pipeline step with typed inputs and outputs.
- **Style IR** — the intermediate representation for presentation.
- **Interactivity IR** — the intermediate representation for behavior.

## Appendix C — This Is a Living Document

This file will evolve as individual package specs (`FM01…FMnn`) are
written. Each of those will further refine, and possibly push back
on, parts of this vision. Where a per-package spec and this vision
disagree, the per-package spec — the one closer to running code —
wins, and this document is updated to match. The history of that
tension is part of the project's honest record.
