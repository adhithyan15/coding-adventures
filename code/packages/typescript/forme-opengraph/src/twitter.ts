/**
 * twitter.ts — `generateTwitterCardTags(meta)`.
 *
 * Emits a newline-separated sequence of Twitter Card `<meta>` tags
 * suitable for dropping into `<head>` per
 * https://developer.twitter.com/en/docs/twitter-for-websites/cards/overview/markup.
 *
 * Note: Twitter Cards use `name="twitter:..."`, not `property=`
 * (which is what OpenGraph uses).  This is per the Twitter spec —
 * `name=` is the historical HTML4 form; `property=` is RDFa.
 *
 * @module twitter
 */

import { assertAbsoluteUrl, escapeHtmlAttr } from "./escape.js";
import type { TwitterCardMeta } from "./types.js";

const ALLOWED_CARDS: ReadonlySet<TwitterCardMeta["card"]> = new Set([
  "summary",
  "summary_large_image",
  "player",
  "app",
]);

export function generateTwitterCardTags(meta: TwitterCardMeta): string {
  if (!ALLOWED_CARDS.has(meta.card)) {
    throw new TypeError(
      `forme-opengraph: twitter:card must be one of ${[...ALLOWED_CARDS].join(", ")} (got ${JSON.stringify(meta.card)})`,
    );
  }
  if (meta.image !== undefined) assertAbsoluteUrl("twitter:image", meta.image);

  const tags: string[] = [];
  tags.push(metaTag("twitter:card", meta.card));
  if (meta.title       !== undefined) tags.push(metaTag("twitter:title",       meta.title));
  if (meta.description !== undefined) tags.push(metaTag("twitter:description", meta.description));
  if (meta.image       !== undefined) tags.push(metaTag("twitter:image",       meta.image));
  if (meta.site        !== undefined) tags.push(metaTag("twitter:site",        meta.site));
  if (meta.creator     !== undefined) tags.push(metaTag("twitter:creator",     meta.creator));
  return tags.join("\n");
}

function metaTag(name: string, content: string): string {
  return `<meta name="${escapeHtmlAttr(name)}" content="${escapeHtmlAttr(content)}">`;
}
