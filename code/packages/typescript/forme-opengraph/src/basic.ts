/**
 * basic.ts — `generateBasicTags(meta)`.
 *
 * Emits the three basic HTML head tags that drive search-engine
 * snippets (independent of OpenGraph / Twitter, which drive social
 * preview cards).
 *
 *   <title>...</title>
 *   <meta name="description" content="...">
 *   <link rel="canonical" href="...">
 *
 * Each field is optional — omit any and the corresponding tag is
 * skipped.  `canonical` MUST be an absolute http(s) URL when
 * supplied.
 *
 * @module basic
 */

import { assertAbsoluteUrl, escapeHtmlAttr, escapeHtmlText } from "./escape.js";
import type { BasicMeta } from "./types.js";

export function generateBasicTags(meta: BasicMeta): string {
  if (meta.canonical !== undefined) assertAbsoluteUrl("link rel=canonical", meta.canonical);

  const tags: string[] = [];
  if (meta.title       !== undefined) tags.push(`<title>${escapeHtmlText(meta.title)}</title>`);
  if (meta.description !== undefined) tags.push(`<meta name="description" content="${escapeHtmlAttr(meta.description)}">`);
  if (meta.canonical   !== undefined) tags.push(`<link rel="canonical" href="${escapeHtmlAttr(meta.canonical)}">`);
  return tags.join("\n");
}
