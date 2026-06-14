/**
 * index-renderer.ts — `renderIndexPage(items, options)`.
 *
 * Emits an HTML `<ul>` (or sequence of `<h2>` + `<ul>` sections
 * when grouped) suitable for an archive / blog-index page.
 * Pairs with `forme-aot-page-emitter` — the emitter writes the
 * `.html` wrapper, this renderer fills the body.
 *
 * Output shape (groupBy: "none"):
 *
 *   <ul class="forme-index">
 *     <li><a href="...">Title</a></li>
 *     ...
 *   </ul>
 *
 * Output shape (grouped):
 *
 *   <section class="forme-index-group">
 *     <h2>Heading</h2>
 *     <ul class="forme-index">
 *       <li>...</li>
 *     </ul>
 *   </section>
 *   ...
 *
 * With `showDate` / `showSummary`:
 *
 *   <li>
 *     <a href="...">Title</a>
 *     <time datetime="iso">formatted</time>
 *     <p class="summary">Summary text</p>
 *   </li>
 *
 * @module index-renderer
 */

import { assertItemUrl, escapeHtmlAttr, escapeHtmlText } from "./escape.js";
import { sortItems } from "./sort.js";
import { groupItems } from "./group.js";
import type { IndexItem, IndexOptions } from "./types.js";

export function renderIndexPage(
  items: readonly IndexItem[],
  options: IndexOptions = {},
): string {
  // Validate every URL FIRST so we never emit partial output on bad input.
  for (const item of items) assertItemUrl(item.url);

  const sortBy = options.sortBy ?? "pubDate-desc";
  const groupBy = options.groupBy ?? "none";

  const sorted = sortItems(items, sortBy);
  const groups = groupItems(sorted, groupBy);

  // Empty input → empty <ul>.  Callers can then conditionally
  // render a "no posts yet" message at a higher layer.
  if (items.length === 0) {
    return `<ul class="forme-index"></ul>`;
  }

  // groupBy === "none" → single flat <ul>.
  if (groupBy === "none") {
    return renderList(groups[0]!.items, options);
  }

  // Grouped — wrap each group in <section><h2>...</h2><ul>...</ul></section>.
  return groups.map((g) => {
    const heading = `<h2>${escapeHtmlText(g.heading)}</h2>`;
    const list    = renderList(g.items, options);
    return `<section class="forme-index-group">\n${heading}\n${list}\n</section>`;
  }).join("\n");
}

function renderList(items: readonly IndexItem[], options: IndexOptions): string {
  const showDate    = options.showDate === true;
  const showSummary = options.showSummary === true;
  const dateFormat  = options.dateFormat ?? identity;

  const lis = items.map((item) => {
    const parts: string[] = [];
    parts.push(`<a href="${escapeHtmlAttr(item.url)}">${escapeHtmlText(item.title)}</a>`);
    if (showDate && item.pubDate !== undefined) {
      parts.push(`<time datetime="${escapeHtmlAttr(item.pubDate)}">${escapeHtmlText(dateFormat(item.pubDate))}</time>`);
    }
    if (showSummary && item.summary !== undefined) {
      parts.push(`<p class="summary">${escapeHtmlText(item.summary)}</p>`);
    }
    return `  <li>${parts.join(" ")}</li>`;
  });
  return `<ul class="forme-index">\n${lis.join("\n")}\n</ul>`;
}

function identity(s: string): string { return s; }

// Re-export the grouping helper at module level so the public
// barrel can pick it up.
export { groupItems };
