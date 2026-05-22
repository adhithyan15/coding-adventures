/**
 * @coding-adventures/forme-doc-sidebar-builder
 *
 * Take a directory layout (file paths) + each file's frontmatter
 * (sidebar position, title overrides, draft flag) and produce a
 * hierarchical, JSON-able sidebar navigation tree the page-shell
 * can render to HTML.
 *
 *   - Groups by directory.
 *   - Orders by frontmatter `sidebar_position` (alphabetical fallback).
 *   - Honours `title` / `sidebar_label` overrides.
 *   - Skips drafts (`draft: true`) — groups that empty out are
 *     also skipped.
 *   - Surfaces `index.md` pages as the group's own destination
 *     (the group's `path` field points to the index page).
 *   - Humanises directory names (`getting-started` →
 *     `"Getting Started"`, `api` → `"API"` via an acronym table).
 *
 * Pure transform.  Capabilities: `[]`.  No `eval`, no `new Function`,
 * no `JSON.parse` reviver, no fs / network / env / shell.  Zero
 * runtime dependencies.
 *
 * ```ts
 * import { buildSidebar } from "@coding-adventures/forme-doc-sidebar-builder";
 *
 * const sidebar = buildSidebar([
 *   { path: "intro.md",          frontmatter: { sidebar_position: 1 } },
 *   { path: "guide/index.md",    frontmatter: { title: "Guide", sidebar_position: 2 } },
 *   { path: "guide/setup.md",    frontmatter: { sidebar_position: 1 } },
 *   { path: "guide/api.md",      frontmatter: { sidebar_position: 2 } },
 *   { path: "advanced.md",       frontmatter: { draft: true } },  // skipped
 * ]);
 * // sidebar = [
 * //   { kind: "page",  label: "Intro", path: "intro.md", position: 1 },
 * //   { kind: "group", label: "Guide", path: "guide/index.md", position: 2, children: [
 * //     { kind: "page", label: "Setup", path: "guide/setup.md", position: 1 },
 * //     { kind: "page", label: "API",   path: "guide/api.md",   position: 2 },
 * //   ]},
 * // ]
 * ```
 *
 * Sixth concrete DOC00 v0 package (after `forme-doc-frontmatter`,
 * `forme-doc-heading-anchors`, `forme-doc-toc-extractor`,
 * `forme-doc-code-block-decorator`, `forme-doc-syntax-highlighter`).
 *
 * @module index
 */

export { buildSidebar } from "./builder.js";
export { humanise } from "./labels.js";
export { normalisePath, stripRoot } from "./path-utils.js";
export type {
  PageInput,
  BuildSidebarOptions,
  SidebarEntry,
  SidebarPageEntry,
  SidebarGroupEntry,
} from "./types.js";
