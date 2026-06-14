/**
 * opengraph.ts — `generateOpenGraphTags(meta)`.
 *
 * Emits a newline-separated sequence of OpenGraph `<meta>` tags
 * suitable for dropping into `<head>`.  Tag order follows the
 * conventional OpenGraph documentation order (title, type, image,
 * url, then optional fields) — deterministic for reproducible
 * builds.
 *
 * @module opengraph
 */

import { assertAbsoluteUrl, escapeHtmlAttr } from "./escape.js";
import type { OpenGraphMeta } from "./types.js";

export function generateOpenGraphTags(meta: OpenGraphMeta): string {
  // URL validation FIRST — throw before emitting anything if a
  // URL is malformed.  We don't want partial output that silently
  // drops a malformed image.
  assertAbsoluteUrl("og:image", meta.image);
  assertAbsoluteUrl("og:url",   meta.url);
  if (meta.video !== undefined) assertAbsoluteUrl("og:video", meta.video);

  const tags: string[] = [];
  tags.push(metaTag("og:title", meta.title));
  tags.push(metaTag("og:type",  meta.type));
  tags.push(metaTag("og:image", meta.image));
  tags.push(metaTag("og:url",   meta.url));
  if (meta.description !== undefined) tags.push(metaTag("og:description", meta.description));
  if (meta.siteName    !== undefined) tags.push(metaTag("og:site_name",   meta.siteName));
  if (meta.locale      !== undefined) tags.push(metaTag("og:locale",      meta.locale));
  if (meta.video       !== undefined) tags.push(metaTag("og:video",       meta.video));
  return tags.join("\n");
}

function metaTag(property: string, content: string): string {
  return `<meta property="${escapeHtmlAttr(property)}" content="${escapeHtmlAttr(content)}">`;
}
