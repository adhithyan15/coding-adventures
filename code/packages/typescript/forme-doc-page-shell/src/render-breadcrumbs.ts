/**
 * render-breadcrumbs.ts — breadcrumb trail → HTML string.
 *
 * Emits an `<ol class="breadcrumbs">` (ordered list because the
 * sequence is semantically meaningful — root first, page last).
 * The final item is a `<span>` (no link, since it's the current
 * page); all prior items are `<a>` links.
 *
 * @module render-breadcrumbs
 */

import { escapeHtml, safeHref } from "./escape.js";
import type { Breadcrumb } from "./types.js";

/**
 * Render the breadcrumb trail.  Empty input → empty string
 * (the page-shell renderer omits the `<ol>` entirely).
 */
export function renderBreadcrumbs(items: readonly Breadcrumb[]): string {
  if (items.length === 0) return "";
  const lastIdx = items.length - 1;
  const lis = items
    .map((item, i) => {
      const labelHtml = escapeHtml(item.label);
      if (i === lastIdx) {
        // Current page — no link.
        return `<li aria-current="page"><span>${labelHtml}</span></li>`;
      }
      return `<li><a href="${safeHref(item.href)}">${labelHtml}</a></li>`;
    })
    .join("");
  return `<ol class="breadcrumbs" aria-label="Breadcrumb">${lis}</ol>`;
}
