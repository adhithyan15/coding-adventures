# @coding-adventures/forme-aot-rss-discovery-link

Emit HTML `<link rel="alternate" type="application/rss+xml">`
tags for feed auto-discovery.  The de-facto convention browsers
and feed readers have used since the early 2000s to surface
"Subscribe to this site" affordances.

Pure transform — returns the tag HTML; caller drops it into
their `<head>`.  Validation runs BEFORE emission.

Fourteenth FM00 v0 stage package — joins the FM00 v0 cluster.

## Quick start

```ts
import { generateFeedDiscoveryLinks } from "@coding-adventures/forme-aot-rss-discovery-link";

// Single feed:
const tag = generateFeedDiscoveryLinks({
  href: "/feed.xml",
  title: "My Blog",
});
// <link rel="alternate" type="application/rss+xml" title="My Blog" href="/feed.xml">

// Multiple feeds (RSS + Atom + JSON Feed):
const tags = generateFeedDiscoveryLinks([
  { href: "/feed.xml",  type: "application/rss+xml",  title: "RSS" },
  { href: "/atom.xml",  type: "application/atom+xml", title: "Atom" },
  { href: "/feed.json", type: "application/json",     title: "JSON Feed" },
]);
// Three <link> tags, one per line.
```

Drop into your HTML `<head>`:

```html
<head>
  <meta charset="utf-8">
  <link rel="alternate" type="application/rss+xml" title="RSS"  href="/feed.xml">
  <link rel="alternate" type="application/atom+xml" title="Atom" href="/atom.xml">
</head>
```

## API

### `generateFeedDiscoveryLinks(input): string`

Main entry.  Accepts a single `FeedDiscoveryLink` or an array
of them.  Returns the concatenated HTML string (one tag per
line, no trailing newline).

```ts
interface FeedDiscoveryLink {
  readonly href: string;             // http(s):// OR /root-relative
  readonly type?: FeedType;          // default "application/rss+xml"
  readonly title?: string;           // HTML-attribute-escaped on emit
}

type FeedType =
  | "application/rss+xml"
  | "application/atom+xml"
  | "application/json";
```

Throws `TypeError` synchronously BEFORE any output if:
- `href` isn't `http(s)://` or root-relative.
- `type` isn't in the allowlist.
- `title` isn't a string.

### Sub-helpers (exposed)

- `validateFeedHref(url)` — URL accept-list.
- `validateFeedType(value)` — feed MIME allowlist.
- `escapeHtmlAttr(s)` / `stripAsciiControl(s)` — single-pass
  HTML attribute escape + control byte strip (same pattern as
  `forme-feeds` / `forme-opengraph` / `forme-index-renderer`).

## Validation rules

| Field | Validator                                              |
|-------|--------------------------------------------------------|
| `href`| http(s):// (case-insensitive) OR root-relative `/path` |
| `type`| `application/rss+xml`, `application/atom+xml`, `application/json` |
| `title`| string (no validation beyond type; HTML-attr-escaped) |

URL reject set (throws):
- `javascript:`, `data:`, `file:`, `vbscript:`
- protocol-relative `//host`
- `/\backslash-variant`
- bare relative
- empty, non-string

Type allowlist is **case-sensitive** — MIME types are by
convention lowercase; accepting case variants would obscure
data-quality issues in caller manifests.  `application/rdf+xml`
(RSS 1.0) is intentionally excluded (effectively deprecated).

## Attribute order

`rel` → `type` → `title` → `href` — matches the convention used
in WordPress, Hugo, Jekyll, and most static-site generators.
Deterministic across calls.

## HTML escaping

Every interpolated value (`type`, `title`, `href`) passes
through `escapeHtmlAttr`:
- Five HTML entities (`& < > " '`) replaced
- ASCII control bytes (`\x00-\x1F`, `\x7F`) stripped first

Defends against attacker-controlled `title` values trying to
break out of the attribute (`title="...">` followed by injected
markup).

## Reproducibility (FM03)

Same input → byte-identical output.

## Security posture

Three concerns explicitly addressed (pre-push review):

- **HTML attribute injection.**  Every value escaped via
  single-pass entity replacement — `<script>alert("xss")</script>`
  in a title becomes inert text.
- **URL scheme injection.**  `javascript:`, `data:`, `file:`,
  `vbscript:`, protocol-relative, backslash-variant all
  rejected with `TypeError`.  Browsers follow `<link
  rel="alternate" href>` URLs when feed readers auto-discover;
  bad schemes are an attack surface.
- **Type allowlist.**  Restricts to the three mainstream feed
  formats; prevents callers from injecting arbitrary MIME
  strings that downstream feed-reader implementations might
  parse permissively.

## Capabilities — `[]`

Pure transform.  No I/O, no network, no shell, no env, no fs.

## Tests

64 tests across 2 files, **100% line / 100% branch** coverage:

- `validate.test.ts` (33) — `validateFeedHref` accept (https,
  http, case-insensitive, root-relative, bare /) + reject
  (javascript:, data:, file:, protocol-relative, backslash-
  variant, bare relative, empty, non-string, null, long-URL
  truncation); `validateFeedType` allowlist (all three values)
  + reject (rdf+xml, text/xml, case-sensitive uppercase, empty,
  non-string); `escapeHtmlAttr` all five entities + composite
  + control-byte stripping (NUL, DEL, ESC) + defensive
  coercion.
- `generate.test.ts` (31) — single link minimal / full / each
  type / absolute href; array (multiple feeds, order preserved,
  single-elem same as object, empty); attribute order
  (`rel→type→title→href`, no title); HTML escaping (ampersand
  in href, quotes in title, XSS attempt in title, control
  bytes); URL validation (each forbidden scheme); type
  validation (rdf+xml, text/xml, case-sensitive); input shape
  validation (null link, non-string title, error identifies
  bad index); fail-fast (mid-array bad type); purity /
  determinism / no input mutation; full real-world example.

## Spec adherence

Implements the de-facto feed auto-discovery convention.  No
formal spec divergences.

## v0 simplifications

- **No `media="..."`** attribute support.  Rarely used for
  feed discovery; deferred.
- **No JSON Feed `application/feed+json` mime alias.**  v1
  may add the alternate spelling.
- **No comment-attribute support.**  Tags are emitted as
  single lines; if you want a `<!-- comment -->` next to them,
  prepend yourself.
