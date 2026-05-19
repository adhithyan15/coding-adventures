# @coding-adventures/forme-aot-meta-link-tags

> FM00 v0 emitter — structured-config → HTML `<meta>` + `<link>`
> `<head>` tags, with URL accept-list + rel/as allowlists.

Fifteenth FM00 v0 stage package. Pure transform — no I/O, no fs,
no network, no env, no shell.

## What it does

`generateMetaLinkTags(config) → string` takes a structured
`MetaLinkConfig` and emits a deterministic string of one tag per
line. Returns the empty string for an empty config.

Five categories of tag are supported:

| Field          | Output                                                     |
| -------------- | ---------------------------------------------------------- |
| `meta`         | `<meta name|http-equiv="..." content="...">`               |
| `canonical`    | `<link rel="canonical" href="...">`                        |
| `prev`, `next` | `<link rel="prev|next" href="...">` pagination             |
| `icons`        | `<link rel="icon|apple-touch-icon|..." href="...">` icons  |
| `preload`      | `<link rel="preload|prefetch|preconnect|..." href="...">`  |

Output order is fixed: `meta → canonical → prev → next → icons →
hints`. Inside each array, the caller's order is preserved.

## Quick start

```ts
import { generateMetaLinkTags } from "@coding-adventures/forme-aot-meta-link-tags";

const head = generateMetaLinkTags({
  canonical: "https://example.com/blog/post-1",
  prev: "/blog/page/0",
  next: "/blog/page/2",
  meta: [
    { name: "viewport", content: "width=device-width, initial-scale=1" },
    { name: "description", content: "A blog post about feeds." },
    { httpEquiv: "x-ua-compatible", content: "IE=edge" },
  ],
  icons: [
    { href: "/favicon.svg", type: "image/svg+xml" },
    { href: "/apple-touch-icon.png", rel: "apple-touch-icon", sizes: "180x180" },
  ],
  preload: [
    { href: "/main.js", rel: "preload", as: "script" },
    { href: "/inter.woff2", rel: "preload", as: "font", type: "font/woff2", crossorigin: "anonymous" },
    { href: "https://fonts.example.com", rel: "preconnect", crossorigin: "anonymous" },
  ],
});
```

Produces (verbatim):

```html
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="description" content="A blog post about feeds.">
<meta http-equiv="x-ua-compatible" content="IE=edge">
<link rel="canonical" href="https://example.com/blog/post-1">
<link rel="prev" href="/blog/page/0">
<link rel="next" href="/blog/page/2">
<link rel="icon" type="image/svg+xml" href="/favicon.svg">
<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png">
<link rel="preload" as="script" href="/main.js">
<link rel="preload" as="font" type="font/woff2" crossorigin="anonymous" href="/inter.woff2">
<link rel="preconnect" crossorigin="anonymous" href="https://fonts.example.com">
```

## Validation

Two-pass fail-fast. The validator walks the entire config first;
if anything throws, the caller gets a `TypeError` with the field
path (`icons[2].href`, `preload[0].crossorigin`, …) and **no
output is ever produced** — there's no risk of a half-formed
`<head>` reaching disk.

### URL accept-list

Every `href` field — `canonical`, `prev`, `next`, `icons[].href`,
`preload[].href` — must be either:

- `http://...` or `https://...` (scheme case-insensitive), **or**
- root-relative `/path` (NOT `//host`, NOT `/\host`).

Everything else throws: `javascript:`, `data:`, `file:`,
`vbscript:`, protocol-relative `//`, backslash-variant `/\`, bare
relative (`about`), empty string, non-string.

### `rel` allowlist (icons)

`icon` | `shortcut icon` | `apple-touch-icon` | `mask-icon`.
Case-sensitive.

### `rel` allowlist (resource hints)

`preload` | `prefetch` | `preconnect` | `dns-prefetch` |
`modulepreload`. Case-sensitive.

### `as` allowlist (resource hints)

`script` | `style` | `image` | `font` | `fetch` | `document` |
`audio` | `video` | `track` | `worker`.

- Required when `rel="preload"` or `rel="modulepreload"`.
- Validated but dropped on `prefetch` / `preconnect` /
  `dns-prefetch` — including `as` there is invalid HTML.

### `crossorigin` allowlist

`anonymous` | `use-credentials`.

### `<meta>` shape

Exactly one of `name` / `httpEquiv` must be provided; `content`
is required. Empty `name` / `httpEquiv` throws.

## Security posture

Five concerns explicitly addressed:

1. **HTML attribute injection.** Every interpolated value passes
   through `escapeHtmlAttr` — single-pass character-class
   replacement covering all five HTML entities (`& < > " '`) plus
   ASCII control-byte stripping. Attacker-controlled `content`
   like `<script>alert(1)</script>` becomes inert text.
2. **URL scheme injection.** `javascript:`, `data:`, `file:`,
   `vbscript:`, protocol-relative, backslash-variant, bare
   relative all rejected with `TypeError`. Crawlers and link
   prefetchers follow these URLs aggressively — the emitter is
   the last line of defence.
3. **rel allowlist.** `rel` controls how browsers treat the
   linked resource (preload runs it immediately, icon is
   downloaded eagerly, preconnect opens a socket). Caller-
   supplied `rel` values are constrained to the spec-defined set
   per category.
4. **as allowlist.** Defends against `as="iframe"`-style
   confusion attacks where a preload might be mis-classified as a
   different fetch destination.
5. **Fail-fast.** Validation completes for every field before
   any tag is emitted. An exception means the caller has nothing
   to write — no partial `<head>` can reach the output buffer.

## Behavioural notes

- **Empty config** → empty string.
- **Output order** is fixed: `meta → canonical → prev → next →
  icons → hints`. The caller controls only the array order
  inside each category.
- **Attribute order** is fixed per tag (see source).
- **Reproducibility.** Same input → byte-identical output. No
  hidden randomness, no environment reads, no input mutation.
- **`charset` is intentionally NOT supported here.** Emit
  `<meta charset="utf-8">` via a separate head builder; this
  emitter focuses on `name` / `http-equiv` pairs.

## v0 simplifications (documented)

- **No `media`** attribute on `<link>` (cold-load optimisation;
  deferred).
- **No `integrity`** / SRI on hints (out of scope — that lives
  in `forme-aot-script-tag-emitter`).
- **No `referrerpolicy`** attribute (rarely set on head tags;
  v1 may add).
- **No HTTP Link header emission.** This package emits HTML
  only; server-side push hints handled elsewhere.

## Tests

132 tests across two files: `validate.test.ts` (URL accept /
reject matrix; rel / as / crossorigin allowlists; optional-string
helper; HTML escape + control-byte strip), `generate.test.ts`
(empty / null / wrong-type configs; canonical / prev / next
order; multiple meta tags with name + http-equiv; meta validation
edge cases; icons with defaults + multiple variants; resource
hints across all five rel values; preload-requires-as enforcement;
as-dropped-on-preconnect; output order across all categories;
fail-fast; HTML escaping; purity / determinism; full real-world
example with verbatim line check).

Coverage: **100% line / 100% branch** across all source files
with logic.

## Capabilities

`[]` — pure transform.

## How it fits in the stack

This is the generic `<head>` tag emitter, complementary to:

- `forme-aot-rss-discovery-link` — feed `<link rel="alternate">` tags.
- `forme-aot-sitemap-emitter` — sitemap.xml.
- `forme-aot-manifest-emitter` — Web App Manifest JSON.

Higher-level FM00 head builders compose all of the above to
produce the final `<head>` for each page.
