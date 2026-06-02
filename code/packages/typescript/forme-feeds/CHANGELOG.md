# Changelog — @coding-adventures/forme-feeds

## Unreleased

### Fixed
- `INVALID_XML_RE` now uses explicit `\uXXXX` escape sequences instead of
  literal control bytes in the source.  Same code-point set, but readable
  and no longer flagged by codeql `js/overly-large-range`.

## 0.1.0 — 2026-05-17

Initial release.  First FM00 v0 stage package — pure RSS 2.0 +
Atom 1.0 feed XML generators.

### Added

- `generateRssFeed(channel, items): string` — emits RSS 2.0 XML
  per https://www.rssboard.org/rss-specification.
  Auto-converts ISO-8601 `lastBuildDate` / `pubDate` to RFC 822
  (passes through if already RFC-822-shaped).  Detects URL-form
  `id`s and marks them `isPermaLink="true"`.
- `generateAtomFeed(feed, items): string` — emits Atom 1.0 XML per
  RFC 4287.  Per-entry `<updated>` falls back to feed-level
  `<updated>` when `item.pubDate` is absent.
- `ChannelMeta` (RSS) and `FeedMeta` (Atom) — format-specific
  envelope metadata types.
- `FeedItem` — shared per-item type with `id`, `title`, `link`,
  optional `content` (plain) / `contentHtml` (CDATA-wrapped) /
  `summary` (Atom only) / `pubDate` / `author`.
- `escapeXml`, `stripInvalidXml`, `wrapCdata` — re-exported for
  callers wiring custom XML generators.

### Spec adherence

- RSS 2.0 per https://www.rssboard.org/rss-specification
- Atom 1.0 per RFC 4287
- XML 1.0 character validity per W3C XML §2.2

No spec divergences.

### Behavioural notes

- **`contentHtml` wins over `content`** when both are supplied —
  HTML is wrapped in CDATA, plain text is escaped.
- **RSS `<author>` uses `email (Name)` form** when email is present
  (per RSS 2.0 spec), bare name when absent.
- **Atom per-entry `<updated>` is mandatory** per RFC 4287
  §4.2.15 — we fall back to feed-level `updated` when item.pubDate
  is absent.
- **`<lastBuildDate>` / `pubDate`** accept either ISO-8601 or
  RFC-822-shaped strings; ISO inputs are converted to RFC 822
  (via `Date.toUTCString`), RFC-822 inputs pass through
  unchanged.  Unparseable inputs pass through verbatim.

### Security posture

Four concerns explicitly addressed:

- **XML injection.**  `escapeXml` handles `& < > " '` in a
  single-pass replacement (CodeQL incomplete-string-escape rule
  accepts this form).  Applied uniformly to ALL interpolated
  strings (titles, links, descriptions, author names, attribute
  values).
- **Invalid XML 1.0 character stripping.**  `stripInvalidXml`
  removes `U+0000-U+0008`, `U+000B`, `U+000C`, `U+000E-U+001F`,
  `U+FFFE`, `U+FFFF` (preserves `\t`, `\n`, `\r`) BEFORE any
  escaping happens, so downstream parsers don't crash on stray
  NUL or vertical-tab bytes from database exports.
- **CDATA termination defence.**  `wrapCdata` splits any literal
  `]]>` sequence into `]]]]><![CDATA[>` so pre-rendered HTML
  containing the CDATA terminator stays safely inside the section.
  Tested with `<script>x = a]]>b;</script>` input.
- **No XML parsing surface.**  We only emit XML; never parse
  user-supplied XML — XXE attacks are inapplicable.

### Capabilities

`[]` — pure transform.  No I/O, no network, no shell, no env.

### Tests

56 tests across 3 files:

- `escape.test.ts` (16) — every entity, all invalid-char ranges,
  preserved C0 allowlist (`\t`, `\n`, `\r`), CDATA wrap and `]]>`
  defence
- `rss.test.ts` (20) — envelope, channel metadata (language,
  lastBuildDate), items, isPermaLink detection, content vs
  contentHtml precedence, RFC 822 conversion + passthrough,
  XML escaping in all string fields, invalid-char stripping,
  empty feed, RFC 822 unparseable passthrough, reproducibility
- `atom.test.ts` (20) — envelope (namespace, declaration),
  feed metadata, per-entry id/title/link/updated (with feed-level
  fallback), content type negotiation, summary, per-entry author
  with/without email, CDATA termination defence, escaping, empty
  feed, reproducibility

Coverage: **100% line / 100% branch** across all source files
that contain logic (`types.ts` is type-only declarations).

### v0 simplifications (documented)

- No iTunes / podcast extensions.
- No Atom `<content src="...">` external references — always
  inlined.
- No multi-author lists per item.
- No categories / tags.
