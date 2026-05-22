/**
 * render-toc.ts — in-page table-of-contents tree → HTML string.
 *
 * Emits an `<aside class="toc">` containing a nested `<ul>`
 * tree.  Each entry is a link to the matching heading anchor
 * (`#${id}`); nested headings become nested `<ul>`s.
 *
 * Heading IDs come from `forme-doc-heading-anchors` (or any
 * compatible source) — they're already URL-safe slugs.  We
 * still pass them through `safeHref`/`escapeAttr` for
 * defence-in-depth.
 *
 * @module render-toc
 */

import { escapeHtml, escapeAttr } from "./escape.js";
import type { TocEntry } from "./types.js";

/**
 * Render the full TOC `<aside>`.  Empty input → empty string
 * (NOT an empty `<aside>` — the page-shell renderer omits the
 * aside entirely when there's no TOC).
 */
export function renderToc(entries: readonly TocEntry[]): string {
  if (entries.length === 0) return "";
  return (
    `<aside class="toc" aria-label="On this page">` +
    `<p class="toc-title">On this page</p>` +
    renderList(entries) +
    `</aside>`
  );
}

function renderList(entries: readonly TocEntry[]): string {
  if (entries.length === 0) return "";
  const items = entries.map((e) => renderEntry(e)).join("");
  return `<ul>${items}</ul>`;
}

function renderEntry(entry: TocEntry): string {
  const childList = renderList(entry.children);
  // Anchor IDs are already URL-safe slugs, but pass through
  // escapeAttr defensively — any quote/ampersand in the id
  // (shouldn't happen but) would corrupt the href otherwise.
  return (
    `<li><a href="#${escapeAttr(entry.id)}">` +
    `${escapeHtml(entry.text)}</a>` +
    `${childList}</li>`
  );
}
