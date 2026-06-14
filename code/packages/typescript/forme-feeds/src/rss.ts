/**
 * rss.ts — `generateRssFeed(channel, items)`.
 *
 * Emits an RSS 2.0 feed per https://www.rssboard.org/rss-specification.
 *
 * Output shape:
 *
 *   <?xml version="1.0" encoding="utf-8"?>
 *   <rss version="2.0">
 *     <channel>
 *       <title>...</title>
 *       <link>...</link>
 *       <description>...</description>
 *       <language>...</language>?
 *       <lastBuildDate>...</lastBuildDate>?
 *       <item>
 *         <title>...</title>
 *         <link>...</link>
 *         <guid isPermaLink="...">...</guid>
 *         <description>...</description>?
 *         <pubDate>...</pubDate>?
 *         <author>...</author>?
 *       </item>
 *       ...
 *     </channel>
 *   </rss>
 *
 * Deterministic: same inputs → byte-identical output (no current
 * timestamp injection, no random ids, no UA-dependent dates).
 *
 * @module rss
 */

import { escapeXml, wrapCdata } from "./escape.js";
import type { ChannelMeta, FeedItem } from "./types.js";

export function generateRssFeed(
  channel: ChannelMeta,
  items: readonly FeedItem[],
): string {
  const parts: string[] = [];
  parts.push(`<?xml version="1.0" encoding="utf-8"?>`);
  parts.push(`<rss version="2.0">`);
  parts.push(`  <channel>`);
  parts.push(`    <title>${escapeXml(channel.title)}</title>`);
  parts.push(`    <link>${escapeXml(channel.link)}</link>`);
  parts.push(`    <description>${escapeXml(channel.description)}</description>`);
  if (channel.language !== undefined) {
    parts.push(`    <language>${escapeXml(channel.language)}</language>`);
  }
  if (channel.lastBuildDate !== undefined) {
    parts.push(`    <lastBuildDate>${escapeXml(toRfc822(channel.lastBuildDate))}</lastBuildDate>`);
  }

  for (const item of items) {
    parts.push(`    <item>`);
    parts.push(`      <title>${escapeXml(item.title)}</title>`);
    parts.push(`      <link>${escapeXml(item.link)}</link>`);
    parts.push(`      <guid isPermaLink="${isPermalink(item.id) ? "true" : "false"}">${escapeXml(item.id)}</guid>`);
    // description: HTML wins over plain content; both wrapped/escaped.
    if (item.contentHtml !== undefined) {
      parts.push(`      <description>${wrapCdata(item.contentHtml)}</description>`);
    } else if (item.content !== undefined) {
      parts.push(`      <description>${escapeXml(item.content)}</description>`);
    }
    if (item.pubDate !== undefined) {
      parts.push(`      <pubDate>${escapeXml(toRfc822(item.pubDate))}</pubDate>`);
    }
    if (item.author !== undefined) {
      // RSS 2.0 spec: <author> is "email (Name)" — but many readers
      // accept just the name when email is absent.  Prefer the
      // strictly-compliant form when email is present.
      const a = item.author;
      const txt = a.email !== undefined
        ? `${a.email} (${a.name})`
        : a.name;
      parts.push(`      <author>${escapeXml(txt)}</author>`);
    }
    parts.push(`    </item>`);
  }

  parts.push(`  </channel>`);
  parts.push(`</rss>`);
  parts.push("");
  return parts.join("\n");
}

// ─── Helpers ─────────────────────────────────────────────────────────────

/**
 * Convert an ISO-8601 datetime to RFC 822 (RSS's mandated format).
 * If the input is already RFC 822-shaped (contains `,` and `GMT`/
 * `±HHMM`), pass through unchanged so callers can opt out of the
 * conversion.
 */
function toRfc822(input: string): string {
  // Crude detection: RFC 822 always has a day-name prefix like "Sun, ".
  if (/^[A-Z][a-z]{2}, /.test(input)) return input;
  const d = new Date(input);
  if (Number.isNaN(d.getTime())) return input;     // pass through unrecognised
  return d.toUTCString().replace("GMT", "+0000");
}

/** True iff `id` looks like a permalink URL (`http://` or `https://`). */
function isPermalink(id: string): boolean {
  return id.startsWith("http://") || id.startsWith("https://");
}
