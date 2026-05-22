/**
 * render-sidebar.ts — sidebar tree → HTML string.
 *
 * Emits a `<nav class="sidebar">` containing a nested
 * `<ul>` / `<li>` tree.  Each leaf page becomes `<li><a>…</a></li>`;
 * each group becomes `<li><span>…</span><ul>…</ul></li>` (or
 * `<li><a>…</a><ul>…</ul></li>` if the group has an index page,
 * making the group label clickable).
 *
 * Active-page highlighting: when `currentPath` matches a leaf's
 * `path`, the `<a>` gets `aria-current="page"`.  CSS themes hook
 * on this attribute for the "you are here" highlight.
 *
 * @module render-sidebar
 */

import { escapeHtml, safeHref } from "./escape.js";
import type { SidebarEntry } from "./types.js";

/**
 * Render the full sidebar `<nav>`.  Empty input → empty nav
 * (still emitted — themes may want to style the empty state).
 */
export function renderSidebar(
  entries: readonly SidebarEntry[],
  currentPath?: string,
): string {
  return `<nav class="sidebar" aria-label="Site navigation">${renderList(entries, currentPath)}</nav>`;
}

/**
 * Render a `<ul>` of entries.  Returns an empty string when
 * `entries` is empty so callers (e.g. groups with no children)
 * can omit the wrapper altogether.
 */
function renderList(
  entries: readonly SidebarEntry[],
  currentPath: string | undefined,
): string {
  if (entries.length === 0) return "";
  const items = entries.map((e) => renderEntry(e, currentPath)).join("");
  return `<ul>${items}</ul>`;
}

/**
 * Render one `<li>` for either a page or a group.
 */
function renderEntry(entry: SidebarEntry, currentPath: string | undefined): string {
  if (entry.kind === "group") {
    return renderGroup(entry, currentPath);
  }
  return renderPage(entry, currentPath);
}

function renderPage(entry: SidebarEntry, currentPath: string | undefined): string {
  // Page entries should always have a non-null path; we defend
  // against null defensively (treat as a non-clickable label).
  if (entry.path === null) {
    return `<li><span>${escapeHtml(entry.label)}</span></li>`;
  }
  const aria = currentPath === entry.path ? ` aria-current="page"` : "";
  return (
    `<li><a href="${safeHref(entry.path)}"${aria}>` +
    `${escapeHtml(entry.label)}</a></li>`
  );
}

function renderGroup(entry: SidebarEntry, currentPath: string | undefined): string {
  const children = entry.children ?? [];
  const childList = renderList(children, currentPath);
  // Group with an index page: the label is a link.  Without an
  // index: the label is a plain span.
  const labelHtml =
    entry.path !== null
      ? `<a href="${safeHref(entry.path)}"${currentPath === entry.path ? ` aria-current="page"` : ""}>` +
        `${escapeHtml(entry.label)}</a>`
      : `<span>${escapeHtml(entry.label)}</span>`;
  return `<li class="group">${labelHtml}${childList}</li>`;
}
