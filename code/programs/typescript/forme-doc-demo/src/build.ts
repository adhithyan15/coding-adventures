/**
 * build.ts — pure transform that wires the DOC00 v0 cluster
 * end-to-end.
 *
 * Input:  an in-memory list of `{ path, source }` markdown files.
 * Output: a `PageBundleConfig` ready for either `generatePageBundle`
 *         (for hashing) or direct disk-write iteration.
 *
 * Capability `[]`.  Lives in `src/` not `bin/` because we want to
 * unit-test the pipeline without touching the filesystem.  All
 * fs I/O lives in `main.ts`.
 *
 * # Pipeline (per page)
 *
 * ```
 *   raw markdown
 *       ▼ forme-doc-frontmatter.extractFrontmatter
 *   { body, frontmatter }
 *       ▼ commonmark-parser.parse
 *   DocumentNode
 *       ▼ forme-doc-heading-anchors.generateHeadingAnchors
 *   { document, anchors }                       (anchors not used yet)
 *       ▼ forme-doc-code-block-decorator.decorateCodeBlocks
 *   DocumentNode'
 *       ▼ forme-doc-syntax-highlighter.highlightCodeBlocks
 *   DocumentNode''
 *       ▼ forme-doc-toc-extractor.extractToc
 *   { document, toc, entries }
 *       ▼ document-ast-to-html.toHtml          (uses the final AST)
 *   bodyHtml                                   (the <main> contents)
 *       ▼ forme-doc-page-shell.renderPageShell  (+ sidebar)
 *   { head, body }
 *       ▼ forme-aot-html-doc-emitter.generateHtmlDocument
 *   full HTML document string
 * ```
 *
 * # Sidebar (built once, cross-page)
 *
 * Each page contributes `{ path, frontmatter }` to
 * `forme-doc-sidebar-builder.buildSidebar`, which produces the
 * tree we pass into every page-shell render call.
 *
 * # Search (built once, cross-page)
 *
 * Each page contributes `{ id, title, body }` (body = plain text
 * from the final AST, via `plainText`) to
 * `forme-doc-search-index-builder.buildSearchIndex`, which
 * produces a `{ manifest, shards }` pair.  The whole bundle is
 * then composed by `forme-doc-site-emitter.emitSite`.
 */

import { createHash } from "node:crypto";

import { extractFrontmatter } from "@coding-adventures/forme-doc-frontmatter";
import { parse as parseMarkdown } from "@coding-adventures/commonmark-parser";
import { generateHeadingAnchors } from "@coding-adventures/forme-doc-heading-anchors";
import { decorateCodeBlocks } from "@coding-adventures/forme-doc-code-block-decorator";
import { highlightCodeBlocks } from "@coding-adventures/forme-doc-syntax-highlighter";
import { extractToc } from "@coding-adventures/forme-doc-toc-extractor";
import { toHtml } from "@coding-adventures/document-ast-to-html";
import { renderPageShell } from "@coding-adventures/forme-doc-page-shell";
import { generateHtmlDocument } from "@coding-adventures/forme-aot-html-doc-emitter";
import { buildSidebar } from "@coding-adventures/forme-doc-sidebar-builder";
import { buildSearchIndex } from "@coding-adventures/forme-doc-search-index-builder";
import { emitSite } from "@coding-adventures/forme-doc-site-emitter";
import type { PageBundleConfig } from "@coding-adventures/forme-doc-site-emitter";

import { plainText } from "./plain-text.js";

/**
 * One markdown source file the caller has read from disk.
 *
 *   - `path`   — relative path under the corpus root, e.g.
 *                `"guide/installation.md"`.  Drives both the
 *                output route and the sidebar position.
 *   - `source` — raw markdown including frontmatter.
 */
export interface MarkdownFile {
  readonly path: string;
  readonly source: string;
}

export interface BuildOptions {
  readonly siteTitle: string;
  readonly githubUrl?: string;
  readonly copyright?: string;
  /** Embedded JS for the search client; emitted as /search/client.js if provided. */
  readonly searchClientJs?: string;
  /** Embedded CSS injected into every page's <head>.  Optional. */
  readonly themeCss?: string;
}

/**
 * Compose the entire site.  Pure function; deterministic; no I/O.
 */
export function build(files: readonly MarkdownFile[], options: BuildOptions): PageBundleConfig {
  // -- Step 1: parse every file (frontmatter + AST) -------------
  const parsed = files.map(parseOne);

  // -- Step 2: build the cross-page sidebar ---------------------
  //
  // The sidebar-builder receives file paths ("guide/setup.md") and
  // emits entries whose `path` field is the same file path.  Page-
  // shell, however, compares `currentPath` against each entry's
  // `path` to render the `aria-current="page"` highlight.  We want
  // that comparison to work against URL routes ("/guide/setup"),
  // not file paths — so we normalise every entry's path through
  // `routeFor` before handing the tree to page-shell.
  const rawSidebar = buildSidebar(parsed.map((p) => ({
    path: p.file.path,
    frontmatter: p.frontmatter,
  })));
  const sidebar = normaliseSidebarPaths(rawSidebar);

  // -- Step 3: render every page's HTML -------------------------
  //
  // headExtra carries:
  //   - the inlined theme stylesheet, and
  //   - (if `options.searchClientJs` is set) a <script> tag
  //     that loads /search/client.js with `defer` so DOM is
  //     parsed before the bootstrap runs.
  //
  // Loading the search script from a separate URL (rather
  // than inlining the entire bundle into every page) means it
  // is cached once per visit; the manifest fetch fires
  // exactly once across the whole session via SearchClient's
  // built-in caching.
  //
  // CACHE-BUST: the script src includes a `?v=<buildId>` query
  // string derived from a SHA-256 of the bundle bytes (or the
  // process start timestamp if no bundle is supplied — irrelevant
  // there since there's nothing to load).  Safari in particular
  // is aggressive about caching same-URL JS / JSON; without a
  // unique query each build, a returning visitor sees the
  // PREVIOUS build's search client even after we redeploy.
  // We pass the same `?v=` to the manifest + shard fetches via
  // an inlined `window.__formeDocSearchBuildId` constant the
  // bootstrap reads.
  const themeCss = options.themeCss ?? DEFAULT_THEME_CSS;
  const hasSearch =
    options.searchClientJs !== undefined && options.searchClientJs.length > 0;
  const buildId = hasSearch
    ? buildIdFor(options.searchClientJs!)
    : "";
  const searchScriptTag = hasSearch
    ? `<script>window.__formeDocSearchBuildId=${JSON.stringify(buildId)};</script>` +
      `<script src="/search/client.js?v=${encodeURIComponent(buildId)}" defer></script>`
    : "";
  const headExtra = `<style>${themeCss}</style>${searchScriptTag}`;
  const pages = parsed.map((p) => renderPage(p, sidebar, options, headExtra));

  // -- Step 4: build the search index ---------------------------
  const searchInputs = parsed.map((p) => ({
    id: routeFor(p.file.path),
    title: titleOf(p.frontmatter, p.file.path),
    body: plainText(p.decoratedAst),
  }));
  const { manifest, shards } = buildSearchIndex(searchInputs);

  // -- Step 5: compose the PageBundleConfig --------------------
  return emitSite({
    pages: pages.map((page) => ({ route: page.route, html: page.html })),
    sidebar,
    search: {
      manifest,
      shards,
      clientJs: options.searchClientJs,
    },
  });
}

// =====================================================================
// Per-page pipeline
// =====================================================================

interface ParsedPage {
  readonly file: MarkdownFile;
  readonly frontmatter: Record<string, unknown>;
  readonly decoratedAst: unknown;  // DocumentNode after the full pipeline
  readonly anchors: ReturnType<typeof generateHeadingAnchors>["anchors"];
  readonly toc: ReturnType<typeof extractToc>;
}

function parseOne(file: MarkdownFile): ParsedPage {
  const { body, frontmatter } = extractFrontmatter(file.source);
  const ast0 = parseMarkdown(body);

  // The heading-anchors walker computes slug IDs for every
  // heading; we'll need these later to inject into the rendered
  // HTML (since `document-ast-to-html`'s `renderHeading` ignores
  // the `id` field on `AnchoredHeadingNode`).
  const { document: ast1, anchors } = generateHeadingAnchors(ast0);
  const ast2 = decorateCodeBlocks(ast1);
  const ast3 = highlightCodeBlocks(ast2);
  // toc-extractor walks the AST and returns a tree (it also runs
  // heading-anchors internally, which is idempotent on already-
  // anchored headings).
  const toc = extractToc(ast3);

  return {
    file,
    frontmatter: frontmatter ?? {},
    decoratedAst: ast3,
    anchors,
    toc,
  };
}

interface RenderedPage {
  readonly route: string;
  readonly html: string;
}

function renderPage(
  parsed: ParsedPage,
  sidebar: ReturnType<typeof buildSidebar>,
  options: BuildOptions,
  headExtra: string,
): RenderedPage {
  const route = routeFor(parsed.file.path);
  const title = titleOf(parsed.frontmatter, parsed.file.path);

  // The <main> body — the markdown rendered to HTML, with anchor
  // IDs injected into each `<h*>` tag in document order.
  const rawBody = toHtml(parsed.decoratedAst as Parameters<typeof toHtml>[0]);
  const bodyHtml = injectHeadingIds(rawBody, parsed.anchors);

  const shell = renderPageShell({
    site: {
      title: options.siteTitle,
      homeUrl: "/",
      githubUrl: options.githubUrl,
      copyright: options.copyright,
    },
    page: {
      title,
      body: bodyHtml,
      toc: parsed.toc.entries,
    },
    sidebar,
    options: {
      currentPath: route,
      headExtra,
      searchPlaceholder: "Search docs…",
    },
  });

  const html = generateHtmlDocument({
    head: shell.head,
    body: shell.body,
    lang: "en",
  });

  return { route, html };
}

// =====================================================================
// Helpers
// =====================================================================

/**
 * Turn a corpus file path into a clean URL route:
 *
 *   "index.md"                     → "/"
 *   "getting-started.md"           → "/getting-started"
 *   "guide/installation.md"        → "/guide/installation"
 *   "guide/index.md"               → "/guide"
 *
 * Normalisation rules:
 *   - Strip leading `./` and leading `/`.
 *   - Strip `.md` / `.mdx` suffix.
 *   - Collapse trailing `/index` to `""` (so `guide/index.md`
 *     becomes `/guide`, not `/guide/index`).
 *   - Empty result becomes `/`.
 *
 * SECURITY NOTE: this is a *normalisation* helper, NOT a
 * sanitiser.  It does not strip `..` segments — input like
 * `"../etc/passwd.md"` produces `"/../etc/passwd"`.  We rely
 * on TWO facts to keep that benign:
 *   1. Corpus paths come from `readCorpus`, which walks a known
 *      root using `fs.readdir` — `readdir` entry names can never
 *      contain `/` or `..`, so a malicious `..` path cannot
 *      arise from any filesystem source.
 *   2. The downstream `forme-doc-site-emitter.emitSite`
 *      validates every route for `..`/`\`/`//` and throws
 *      before any disk write; `safeJoin` in `main.ts` adds a
 *      containment check on top.
 * Callers who hand `routeFor` arbitrary user-controlled strings
 * outside the read-from-disk path MUST validate the result.
 */
export function routeFor(path: string): string {
  let p = path;
  if (p.startsWith("./")) p = p.slice(2);
  if (p.startsWith("/")) p = p.slice(1);
  // Strip extension (only .md / .mdx — anything else preserved).
  if (p.endsWith(".mdx")) p = p.slice(0, -".mdx".length);
  else if (p.endsWith(".md")) p = p.slice(0, -".md".length);
  // Collapse trailing /index → "" so guide/index.md → guide.
  if (p === "index") p = "";
  else if (p.endsWith("/index")) p = p.slice(0, -"/index".length);
  return p === "" ? "/" : `/${p}`;
}

/**
 * Resolve a page's display title:
 *   1. `frontmatter.title` if it's a non-empty string.
 *   2. Otherwise humanise the basename of the file path.
 */
export function titleOf(frontmatter: Record<string, unknown>, path: string): string {
  const t = frontmatter["title"];
  if (typeof t === "string" && t.length > 0) return t;
  // Fallback: humanise the basename.
  const slash = path.lastIndexOf("/");
  const base = slash === -1 ? path : path.slice(slash + 1);
  const noExt = base.replace(/\.(md|mdx)$/u, "");
  // dashed-or-underscored → spaced + Title Case (simple impl;
  // sidebar-builder's `humanise` would also work but pulling it
  // in for a fallback only is more wiring than it's worth).
  return noExt
    .split(/[-_]/u)
    .filter((w) => w.length > 0)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

/**
 * Inject `id="..."` attributes into rendered heading tags using
 * the in-document-order list of anchors from
 * `forme-doc-heading-anchors`.
 *
 * Why this exists: `document-ast-to-html` doesn't read the
 * `AnchoredHeadingNode.id` field (it only knows about plain
 * `HeadingNode`).  Rather than patch the upstream renderer (out
 * of scope), we walk the rendered HTML and inject the IDs here.
 *
 * The match is positional — N-th `<hL>` in HTML order maps to
 * the N-th anchor in document order.  This is safe because the
 * AST-to-HTML renderer emits headings in the same order as the
 * AST visits them, and `<h*>` tags only appear from heading
 * nodes (code-blocks render as `<pre><code>`, so no false
 * matches there).
 *
 * The regex is `/<h([1-6])>/g` (no `+` quantifier, fixed
 * 6-char character class) — trivially ReDoS-free.
 */
export function injectHeadingIds(
  html: string,
  anchors: ReadonlyArray<{ readonly id: string }>,
): string {
  let i = 0;
  return html.replace(/<h([1-6])>/g, (_match, level: string) => {
    if (i >= anchors.length) return _match;
    const id = anchors[i]!.id;
    i++;
    return `<h${level} id="${escapeAttr(id)}">`;
  });
}

/**
 * Escape a slug for safe insertion into an HTML attribute value.
 * Slugs from heading-anchors are already URL-safe (alphanumerics
 * + dashes), but defence-in-depth: still escape the five
 * HTML-special chars to make the helper safe for any string.
 */
function escapeAttr(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/**
 * Recursively walk the sidebar tree and replace every entry's
 * `path` field (a relative file path like `"guide/setup.md"`)
 * with its URL route equivalent (`"/guide/setup"`) so that
 * page-shell's `currentPath` comparison works.
 *
 * The input tree is treated as immutable; we return new entries.
 */
export function normaliseSidebarPaths<
  T extends { readonly path: string | null; readonly children?: readonly T[] }
>(tree: readonly T[]): T[] {
  return tree.map((entry) => normaliseOne(entry));
}

function normaliseOne<
  T extends { readonly path: string | null; readonly children?: readonly T[] }
>(entry: T): T {
  const out: { path: string | null; children?: T[] } & Record<string, unknown> = { ...entry };
  if (typeof entry.path === "string" && entry.path.length > 0) {
    out.path = routeFor(entry.path);
  }
  if (Array.isArray(entry.children)) {
    out.children = entry.children.map((c) => normaliseOne(c));
  }
  return out as T;
}

/**
 * Compute a short, content-addressed build ID from the bundled
 * search-client source.  Used as a cache-bust query string on
 * the script tag + fetch URLs so a returning visitor never
 * sees a stale bundle after we rebuild.
 *
 * Content-addressed (first 12 hex chars of SHA-256) means:
 *   - Same bundle content → same ID → browser reuses its cache
 *     (good — we WANT cache hits when nothing changed).
 *   - Different bundle content → different ID → forced re-fetch
 *     (good — we want fresh content when we ship).
 *
 * 12 hex chars = 48 bits of entropy — collisions essentially
 * impossible across realistic build counts.  Short enough to
 * keep URLs readable.
 */
export function buildIdFor(bundleSource: string): string {
  return createHash("sha256").update(bundleSource).digest("hex").slice(0, 12);
}

/**
 * A minimal stylesheet — just enough to make the demo look
 * presentable.  Two-column layout, lightly-styled sidebar,
 * monospace for code.  Inlined into every page's <head>.
 *
 * Intentionally NOT loaded as an external <link> — keeps the
 * demo single-file-portable (any page is fully self-contained
 * apart from its search shard fetches).
 */
const DEFAULT_THEME_CSS = `
:root { --fg:#222; --bg:#fff; --muted:#666; --link:#0366d6; --accent:#0366d6;
        --sidebar-bg:#fafafa; --border:#e5e5e5; --code-bg:#f6f8fa; }
* { box-sizing: border-box; }
body { font: 15px/1.55 system-ui, -apple-system, sans-serif; color: var(--fg);
       margin: 0; background: var(--bg); }
a { color: var(--link); text-decoration: none; }
a:hover { text-decoration: underline; }
header { display: flex; align-items: center; gap: 1rem; padding: .8rem 1.5rem;
         border-bottom: 1px solid var(--border); position: sticky; top: 0;
         background: var(--bg); z-index: 10; }
header .site-title { font-weight: 600; font-size: 1.05rem; color: var(--fg); }
header input.search { padding: .35rem .6rem; border: 1px solid var(--border);
                      border-radius: 4px; min-width: 280px; font: inherit;
                      background: var(--bg); color: var(--fg);
                      outline: none; transition: border-color .15s; }
header input.search:focus { border-color: var(--accent); }
header .github-link { margin-left: auto; }
.layout { display: grid; grid-template-columns: 240px minmax(0,1fr) 200px;
          gap: 0; min-height: calc(100vh - 56px); }
nav.sidebar { background: var(--sidebar-bg); border-right: 1px solid var(--border);
              padding: 1rem .8rem; overflow-y: auto; }
nav.sidebar ul { list-style: none; padding: 0 0 0 .8rem; margin: 0; }
nav.sidebar > ul { padding-left: 0; }
nav.sidebar li { margin: .25rem 0; }
nav.sidebar a { color: var(--fg); display: block; padding: .15rem .4rem;
                border-radius: 3px; }
nav.sidebar a[aria-current=page] { background: var(--accent); color: white; }
nav.sidebar .sidebar-group > .sidebar-group-label { font-weight: 600;
              color: var(--muted); font-size: .85rem;
              text-transform: uppercase; letter-spacing: .03em;
              padding: .5rem .4rem .15rem; }
main { padding: 2rem 2.5rem; max-width: 820px; min-width: 0; }
main h1 { margin-top: 0; font-size: 2rem; }
main h2 { margin-top: 2rem; padding-bottom: .3rem; border-bottom: 1px solid var(--border); }
main p, main ul, main ol, main pre { margin: .8rem 0; }
main code { background: var(--code-bg); padding: .1em .35em; border-radius: 3px;
            font-size: .92em; }
main pre { background: var(--code-bg); padding: 1rem; border-radius: 6px;
           overflow-x: auto; }
main pre code { background: none; padding: 0; font-size: .9rem; }
aside.toc { padding: 1rem; border-left: 1px solid var(--border);
            font-size: .9rem; color: var(--muted); }
aside.toc ul { list-style: none; padding: 0; }
aside.toc li { margin: .25rem 0; }
aside.toc a { color: var(--muted); }
aside.toc a:hover { color: var(--fg); }
footer { padding: 1rem 1.5rem; border-top: 1px solid var(--border);
         color: var(--muted); font-size: .85rem; text-align: center; }
@media (max-width: 900px) {
  .layout { grid-template-columns: 1fr; }
  nav.sidebar, aside.toc { display: none; }
}
`;
