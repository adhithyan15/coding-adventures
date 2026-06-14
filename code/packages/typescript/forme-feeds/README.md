# @coding-adventures/forme-feeds

**RSS 2.0** + **Atom 1.0** feed XML generators for the Forme
pipeline.  Pure transform: `ChannelMeta | FeedMeta + items[]` →
reproducible XML string.

First FM00 v0 stage package — sits alongside the FM04 / FM06
families to round out the static-site-generator vocabulary.

## Quick start

```ts
import {
  generateRssFeed, generateAtomFeed,
  type FeedItem,
} from "@coding-adventures/forme-feeds";

const posts: FeedItem[] = [
  {
    id: "https://example.com/posts/hello",
    title: "Hello, world",
    link: "https://example.com/posts/hello",
    pubDate: "2026-05-17T10:00:00Z",
    contentHtml: "<p>First post!</p>",
    author: { name: "Jane Doe", email: "jane@example.com" },
  },
];

// RSS 2.0
const rss = generateRssFeed(
  {
    title: "My Blog",
    link: "https://example.com/",
    description: "Notes from the road",
    language: "en-US",
  },
  posts,
);

// Atom 1.0
const atom = generateAtomFeed(
  {
    id: "https://example.com/atom.xml",
    title: "My Blog",
    updated: "2026-05-17T10:00:00Z",
    link: "https://example.com/atom.xml",
  },
  posts,
);
```

## API

### `generateRssFeed(channel, items): string`

Emits an RSS 2.0 feed per [the RSS Advisory Board specification](https://www.rssboard.org/rss-specification).

```ts
interface ChannelMeta {
  title: string;
  link: string;
  description: string;
  language?: string;
  lastBuildDate?: string;   // ISO-8601 OR already-RFC-822
}
```

### `generateAtomFeed(feed, items): string`

Emits an Atom 1.0 feed per [RFC 4287](https://datatracker.ietf.org/doc/html/rfc4287).

```ts
interface FeedMeta {
  id: string;              // mandatory, conventionally the feed URL
  title: string;
  updated: string;         // ISO-8601
  link?: string;
  author?: { name: string; email?: string };
  subtitle?: string;
}
```

### Shared `FeedItem` shape

```ts
interface FeedItem {
  id: string;              // mandatory; URL form → RSS isPermaLink="true"
  title: string;
  link: string;            // canonical URL
  content?: string;        // plain text → RSS <description>, Atom <content type="text">
  contentHtml?: string;    // HTML → wrapped in CDATA; wins over content
  summary?: string;        // Atom <summary>; RSS ignores
  pubDate?: string;        // ISO-8601; RSS converts to RFC 822
  author?: { name: string; email?: string };
}
```

## XML escaping

Every interpolated string routes through `escapeXml` — the five XML
predefined entities (`& < > " '`) are replaced; invalid XML 1.0
characters (`U+0000-U+0008`, `U+000B`, `U+000C`, `U+000E-U+001F`,
`U+FFFE`, `U+FFFF`) are stripped silently (preserves `\t`, `\n`, `\r`).

`contentHtml` is wrapped in CDATA so pre-rendered HTML ships
verbatim.  Any literal `]]>` in the HTML is split across CDATA
boundaries (`]]]]><![CDATA[>`) so the section can't terminate early.

## Reproducibility (FM03)

Same inputs → byte-identical output.  No `Date.now()`, no random IDs,
no UA-dependent formatting.  RSS `pubDate` conversion uses
`Date.toUTCString()` which is deterministic for any valid ISO-8601
input.

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell, no env.  Same
posture as the FM04 translator packages.

## Security posture

Four concerns explicitly addressed:

1. **XML injection.**  `escapeXml` handles all five predefined
   entities in a single-pass replacement (CodeQL-friendly form).
2. **Invalid XML 1.0 characters.**  `stripInvalidXml` removes
   forbidden C0 controls before any escaping happens — feed
   parsers won't crash on stray NUL or vertical-tab bytes.
3. **CDATA injection.**  `wrapCdata` splits `]]>` sequences so
   pre-rendered HTML containing the CDATA terminator stays
   safely inside the section.
4. **No XML parsing surface.**  We only EMIT XML; we never parse
   user-supplied XML, so XXE attacks are inapplicable.

## Tests

56 tests across 3 files:

- `escape.test.ts` (16) — every entity, all invalid-char ranges,
  preserved C0 allowlist, CDATA wrap + `]]>` defence
- `rss.test.ts` (20) — envelope, channel metadata, items, isPermaLink
  detection, content vs contentHtml precedence, RFC 822 conversion +
  passthrough, escaping in titles, empty feed, reproducibility
- `atom.test.ts` (20) — envelope, feed metadata, entries, content
  type negotiation, summary, per-entry author, CDATA termination
  defence, escaping, empty feed, reproducibility

Coverage: **100% line / 100% branch** across the four `.ts` files
that contain logic (`types.ts` is type-only declarations).

## Spec adherence

- RSS 2.0 per https://www.rssboard.org/rss-specification
- Atom 1.0 per RFC 4287

No spec divergences.

## v0 simplifications

- **No iTunes/podcast extensions.**  Add when a use case demands.
- **No Atom `<content src="...">` external content.**  Always
  inlined.
- **No multi-author lists.**  Single author per item.
- **No categories / tags.**  Add to `FeedItem` when needed.
