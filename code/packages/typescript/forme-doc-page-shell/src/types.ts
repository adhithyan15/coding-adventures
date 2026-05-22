/**
 * types.ts — public signatures for the page-shell renderer.
 *
 * The shapes for `SidebarEntry` and `TocEntry` are intentionally
 * structurally compatible with the equivalents in
 * `@coding-adventures/forme-doc-sidebar-builder` and
 * `@coding-adventures/forme-doc-toc-extractor`, but redefined
 * here to keep this package zero-dependency.  Callers using
 * those packages pass the outputs directly — TypeScript's
 * structural typing handles the rest.
 *
 * @module types
 */

// ─────────────────────────────────────────────────────────────────────
// Inputs
// ─────────────────────────────────────────────────────────────────────

/**
 * Site-wide configuration that's identical across every rendered
 * page (header brand, footer copyright, GitHub link, etc.).
 */
export interface SiteConfig {
  /** Brand name shown in the header. */
  readonly title: string;
  /** Optional URL the brand links to (default `"/"`). */
  readonly homeUrl?: string;
  /** Optional GitHub repo URL — surfaced as a header icon link. */
  readonly githubUrl?: string;
  /** Optional copyright text shown in the footer. */
  readonly copyright?: string;
  /** Optional version label shown in the footer. */
  readonly version?: string;
}

/**
 * Per-page data.
 */
export interface PageInfo {
  /** Page title — used for `<title>`, `<h1>`, breadcrumbs. */
  readonly title: string;
  /**
   * The page's body content.  TRUSTED: passed through to the
   * output verbatim, no escaping.  The expected source is the
   * HTML rendered by `commonmark-parser` (after the heading-anchor,
   * code-decorator, and syntax-highlighter passes) — those
   * packages enforce their own structural invariants.
   *
   * If the caller is constructing this from untrusted user input
   * directly (e.g. a CMS), they MUST escape it first.  That's
   * outside this package's contract.
   */
  readonly body: string;
  /** Optional meta description for `<head>`. */
  readonly description?: string;
  /** Optional breadcrumb trail (root → page). */
  readonly breadcrumbs?: readonly Breadcrumb[];
  /** Optional in-page table of contents (the toc-extractor's output). */
  readonly toc?: readonly TocEntry[];
  /** Optional "Edit this page" URL shown in the footer. */
  readonly editUrl?: string;
}

/** One breadcrumb in the breadcrumb trail. */
export interface Breadcrumb {
  readonly label: string;
  readonly href: string;
}

/**
 * One node of a hierarchical TOC tree.  Structurally compatible
 * with `@coding-adventures/forme-doc-toc-extractor`'s output.
 */
export interface TocEntry {
  readonly text: string;
  readonly id: string;
  readonly level: 1 | 2 | 3 | 4 | 5 | 6;
  readonly children: readonly TocEntry[];
}

/**
 * One entry in the sidebar nav tree.  Structurally compatible
 * with `@coding-adventures/forme-doc-sidebar-builder`'s output.
 */
export interface SidebarEntry {
  readonly kind: "page" | "group";
  readonly label: string;
  /** For "page" entries: the URL.  For "group" entries: `null` if
   *  the group has no index page; the index page's URL otherwise. */
  readonly path: string | null;
  /** Present for "group" entries (or `undefined` for "page"
   *  entries — kept optional so structural typing accepts both). */
  readonly children?: readonly SidebarEntry[];
}

/**
 * Optional rendering options.
 *
 * Note `lang` and `bodyClass` are NOT in v0 — this package emits
 * `<head>` and `<body>` *contents* only; the outer
 * `<html lang>` / `<body class>` tags are emitted by
 * `forme-aot-html-doc-emitter`.  Pass them there instead.
 */
export interface PageShellOptions {
  /**
   * Raw HTML to inject at the end of `<head>` (e.g. analytics
   * scripts, custom `<link rel="stylesheet">`).  **Caller is
   * responsible for escaping** — this is a documented escape
   * hatch, not a safe value.
   */
  readonly headExtra?: string;
  /**
   * URL of the active page in the sidebar.  Used to add an
   * `aria-current="page"` attribute to the matching sidebar
   * link, which most CSS themes style as the "highlighted"
   * entry.  Default: no highlight.
   */
  readonly currentPath?: string;
  /**
   * Search box placeholder text, if any.  When set, the header
   * gets a non-interactive search input (the search-client-js
   * package wires it up at runtime).  Default: no input.
   */
  readonly searchPlaceholder?: string;
}

/**
 * Top-level input shape for `renderPageShell`.
 */
export interface PageShellInput {
  readonly site: SiteConfig;
  readonly page: PageInfo;
  readonly sidebar: readonly SidebarEntry[];
  readonly options?: PageShellOptions;
}

// ─────────────────────────────────────────────────────────────────────
// Output
// ─────────────────────────────────────────────────────────────────────

/**
 * Output shape — two HTML chunks ready for
 * `forme-aot-html-doc-emitter` to wrap into a full document.
 */
export interface PageShellOutput {
  /**
   * Content of the `<head>` element: meta tags (charset,
   * viewport, description), `<title>`, optional `headExtra`.
   * Does NOT include the `<head>` tag itself.
   */
  readonly head: string;
  /**
   * Content of the `<body>` element: header, sidebar, main
   * (breadcrumbs + article + in-page TOC), footer.  Does NOT
   * include the `<body>` tag itself.
   */
  readonly body: string;
}
