/**
 * atom.ts — `generateAtomFeed(feed, items)`.
 *
 * Emits an Atom 1.0 feed per RFC 4287.  Output shape:
 *
 *   <?xml version="1.0" encoding="utf-8"?>
 *   <feed xmlns="http://www.w3.org/2005/Atom">
 *     <id>...</id>
 *     <title>...</title>
 *     <updated>...</updated>
 *     <link rel="self" href="..."/>?
 *     <author><name>...</name><email>...</email>?</author>?
 *     <subtitle>...</subtitle>?
 *     <entry>
 *       <id>...</id>
 *       <title>...</title>
 *       <link href="..."/>
 *       <updated>...</updated>
 *       <summary>...</summary>?
 *       <content type="html|text">...</content>?
 *       <author>...</author>?
 *     </entry>
 *     ...
 *   </feed>
 *
 * Deterministic: same inputs → byte-identical output.
 *
 * @module atom
 */

import { escapeXml, wrapCdata } from "./escape.js";
import type { FeedItem, FeedMeta } from "./types.js";

export function generateAtomFeed(
  feed: FeedMeta,
  items: readonly FeedItem[],
): string {
  const parts: string[] = [];
  parts.push(`<?xml version="1.0" encoding="utf-8"?>`);
  parts.push(`<feed xmlns="http://www.w3.org/2005/Atom">`);
  parts.push(`  <id>${escapeXml(feed.id)}</id>`);
  parts.push(`  <title>${escapeXml(feed.title)}</title>`);
  parts.push(`  <updated>${escapeXml(feed.updated)}</updated>`);
  if (feed.link !== undefined) {
    parts.push(`  <link rel="self" href="${escapeXml(feed.link)}"/>`);
  }
  if (feed.author !== undefined) {
    parts.push(`  <author>`);
    parts.push(`    <name>${escapeXml(feed.author.name)}</name>`);
    if (feed.author.email !== undefined) {
      parts.push(`    <email>${escapeXml(feed.author.email)}</email>`);
    }
    parts.push(`  </author>`);
  }
  if (feed.subtitle !== undefined) {
    parts.push(`  <subtitle>${escapeXml(feed.subtitle)}</subtitle>`);
  }

  for (const item of items) {
    parts.push(`  <entry>`);
    parts.push(`    <id>${escapeXml(item.id)}</id>`);
    parts.push(`    <title>${escapeXml(item.title)}</title>`);
    parts.push(`    <link href="${escapeXml(item.link)}"/>`);
    // Atom <updated> is mandatory per RFC 4287 §4.2.15.  We use the
    // item's pubDate if supplied, else the feed-level updated.
    parts.push(`    <updated>${escapeXml(item.pubDate ?? feed.updated)}</updated>`);
    if (item.summary !== undefined) {
      parts.push(`    <summary>${escapeXml(item.summary)}</summary>`);
    }
    if (item.contentHtml !== undefined) {
      parts.push(`    <content type="html">${wrapCdata(item.contentHtml)}</content>`);
    } else if (item.content !== undefined) {
      parts.push(`    <content type="text">${escapeXml(item.content)}</content>`);
    }
    if (item.author !== undefined) {
      parts.push(`    <author>`);
      parts.push(`      <name>${escapeXml(item.author.name)}</name>`);
      if (item.author.email !== undefined) {
        parts.push(`      <email>${escapeXml(item.author.email)}</email>`);
      }
      parts.push(`    </author>`);
    }
    parts.push(`  </entry>`);
  }

  parts.push(`</feed>`);
  parts.push("");
  return parts.join("\n");
}
