/**
 * @coding-adventures/forme-doc-page-shell
 *
 * RTD-minimal two-column HTML page-shell.  Wraps content in:
 *
 *   - Header bar: site title (brand link), optional search input,
 *     optional GitHub repo link.
 *   - Two-column layout: sidebar nav on the left, main content
 *     in the middle, optional in-page TOC on the right.
 *   - Main content: optional breadcrumbs, `<h1>` page title,
 *     trusted body HTML.
 *   - Footer: optional version, "edit this page" link, copyright.
 *
 * Output is `{ head, body }` HTML chunks the
 * `forme-aot-html-doc-emitter` wraps into a full document.
 *
 * Pure transform.  Capabilities: `[]`.  Zero runtime dependencies.
 *
 * ```ts
 * import { renderPageShell } from "@coding-adventures/forme-doc-page-shell";
 *
 * const { head, body } = renderPageShell({
 *   site: { title: "My Docs", githubUrl: "https://github.com/me/repo" },
 *   page: {
 *     title: "Getting Started",
 *     body: "<p>Welcome to the docs.</p>",
 *     toc: [{ text: "Install", id: "install", level: 2, children: [] }],
 *   },
 *   sidebar: [
 *     { kind: "page", label: "Intro", path: "/intro" },
 *     { kind: "group", label: "Guide", path: "/guide", children: [
 *       { kind: "page", label: "Setup", path: "/guide/setup" },
 *     ]},
 *   ],
 *   options: { currentPath: "/guide/setup" },
 * });
 * ```
 *
 * ## Security
 *
 * Every user-supplied string is HTML-escaped via `escapeHtml` /
 * `escapeAttr`.  Every URL goes through `safeHref`, which
 * allowlists `http:` / `https:` / `mailto:` / relative URLs /
 * fragments — `javascript:`, `data:`, `vbscript:`, etc. become
 * `"#"`.
 *
 * Two TRUSTED fields are passed through verbatim and clearly
 * documented:
 *   - `page.body` — HTML rendered by upstream markdown / heading
 *     / decorator / highlighter packages.
 *   - `options.headExtra` — explicit escape hatch for analytics
 *     scripts, custom stylesheets, etc.  Caller's responsibility
 *     to ensure safety.
 *
 * Seventh concrete DOC00 v0 package.
 *
 * @module index
 */

export { renderPageShell } from "./render-shell.js";
export { renderSidebar } from "./render-sidebar.js";
export { renderToc } from "./render-toc.js";
export { renderBreadcrumbs } from "./render-breadcrumbs.js";
export { escapeHtml, escapeAttr, safeHref } from "./escape.js";
export type {
  SiteConfig,
  PageInfo,
  Breadcrumb,
  TocEntry,
  SidebarEntry,
  PageShellOptions,
  PageShellInput,
  PageShellOutput,
} from "./types.js";
