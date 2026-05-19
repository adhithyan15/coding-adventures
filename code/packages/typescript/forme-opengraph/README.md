# @coding-adventures/forme-opengraph

**OpenGraph** + **Twitter Card** + **basic `<meta>` tag** generators for the Forme pipeline (FM00 v0 SEO stage).

Pure transform: meta records → reproducible HTML `<meta>` / `<link>` / `<title>` tag sequence.  Companion to [`forme-feeds`](../forme-feeds) in the FM00 v0 stage family.

## Quick start

```ts
import { generateMetaTags } from "@coding-adventures/forme-opengraph";

const head = generateMetaTags({
  basic: {
    title: "Hello World",
    description: "A first post on my blog",
    canonical: "https://example.com/hello",
  },
  og: {
    title: "Hello World",
    type: "article",
    image: "https://example.com/og.png",
    url: "https://example.com/hello",
    description: "A first post on my blog",
    siteName: "My Blog",
  },
  twitter: {
    card: "summary_large_image",
    site: "@myblog",
    creator: "@me",
  },
});

// → <title>Hello World</title>
//   <meta name="description" content="A first post on my blog">
//   <link rel="canonical" href="https://example.com/hello">
//   <meta property="og:title" content="Hello World">
//   <meta property="og:type" content="article">
//   ...
```

## API

### `generateOpenGraphTags(meta): string`

Emits OpenGraph `<meta>` tags per https://ogp.me/.  Conventional tag order: title → type → image → url → description → siteName → locale → video.

```ts
interface OpenGraphMeta {
  title: string;
  type: string;          // "website", "article", "video.movie", ...
  image: string;         // MUST be absolute http(s)
  url: string;           // MUST be absolute http(s)
  description?: string;
  siteName?: string;     // emitted as og:site_name
  locale?: string;       // e.g. "en_US"
  video?: string;        // MUST be absolute http(s) if supplied
}
```

### `generateTwitterCardTags(meta): string`

Emits Twitter Card `<meta>` tags per https://developer.twitter.com/en/docs/twitter-for-websites/cards/overview/markup.

```ts
interface TwitterCardMeta {
  card: "summary" | "summary_large_image" | "player" | "app";
  title?: string;
  description?: string;
  image?: string;        // MUST be absolute http(s) if supplied
  site?: string;         // @site Twitter handle for the publishing site
  creator?: string;      // @creator Twitter handle for the content author
}
```

### `generateBasicTags(meta): string`

Emits `<title>`, `<meta name="description">`, `<link rel="canonical">` — drives search-engine snippets independently of social previews.

```ts
interface BasicMeta {
  title?: string;
  description?: string;
  canonical?: string;    // MUST be absolute http(s) if supplied
}
```

### `generateMetaTags({ basic?, og?, twitter? }): string`

Convenience wrapper — combines all three blocks in `basic → og → twitter` order.

## URL validation (security-critical)

The fields that resolve to a fetch target — `og:image`, `og:url`, `og:video`, `twitter:image`, canonical — are validated against `/^https?:\/\//i`.  Anything else **throws `TypeError`** synchronously before any output is emitted:

- Relative paths (`/path`, `path`, `./x`)
- `javascript:alert(1)` (XSS in scrapers that render previews)
- `data:text/html,…` (tracker / payload injection)
- `file:///etc/passwd`
- Protocol-relative `//host/path`

The error message includes the field name so callers can diagnose which input failed.

## HTML attribute escaping

All meta-tag content values land inside `content="..."` attributes.  All five HTML entities (`& < > " '`) are escaped in a single-pass replacement (CodeQL-friendly form).  ASCII control bytes (`U+0000-U+001F`, `U+007F`) are stripped before escaping — they have no legitimate place in a meta-tag value and they confuse downstream HTML parsers.

Unicode `> U+007F` passes through unchanged (HTML5 is UTF-8 by default).

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell.

## Reproducibility (FM03)

Same inputs → byte-identical output.  Tag order is fixed by spec convention (deterministic across runs).

## Security posture

Three concerns explicitly addressed:

1. **HTML attribute injection.**  `escapeHtmlAttr` handles all five entities in single-pass replacement.  Every value landing in a `content=` / `href=` attribute routes through it.  Tests pin escape coverage and explicitly verify control-byte stripping.
2. **URL scheme validation.**  `assertAbsoluteUrl` rejects everything except `http(s)://...` — `javascript:` / `data:` / `file:` / protocol-relative all throw `TypeError` before output is emitted.  Tests pin every rejected form.
3. **Control-character stripping.**  ASCII control bytes removed from every string field; preserves Unicode and standard whitespace.

## Tests

68 tests across 5 files:

- `escape.test.ts` (19) — every HTML entity, control-byte stripping, URL scheme acceptance and rejection (10+ cases), field-name in error messages, non-string rejection
- `opengraph.test.ts` (16) — required fields in order, all optional fields, URL validation rejects each forbidden form, HTML escaping, reproducibility
- `twitter.test.ts` (18) — all four card types, unknown card rejection, optional fields with order check, URL validation, escaping, reproducibility
- `basic.test.ts` (9) — each tag individually, all-three order, empty input, canonical URL validation, escaping
- `combined.test.ts` (6) — block ordering, skip-when-empty, URL-error propagation, no spurious blank lines, reproducibility

Coverage: **100% line / 100% branch** across all source files with logic (`types.ts` is type-only declarations).

## Spec adherence

- OpenGraph per https://ogp.me/
- Twitter Cards per https://developer.twitter.com/en/docs/twitter-for-websites/cards/overview/markup
- HTML5 attribute syntax per WHATWG HTML

No spec divergences.

## v0 simplifications

- No `og:image:width` / `og:image:height` / `og:image:alt` — add when a use case demands.
- No `article:author` / `article:published_time` Open Graph Article object extensions.
- No `twitter:player` extra metadata (height, width, stream).
- No multi-image arrays.
