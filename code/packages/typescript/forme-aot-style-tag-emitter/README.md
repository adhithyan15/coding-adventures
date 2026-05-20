# @coding-adventures/forme-aot-style-tag-emitter

> FM00 v0 emitter — `StyleConfig` → HTML `<link rel="stylesheet">`
> + inline `<style>` tag strings, with SRI integrity + media-query
> + `</style>`-injection defence.

Seventeenth FM00 v0 stage package. Pure transform — no I/O, no
fs, no network, no env, no shell.

## What it does

`generateStyleTags(config) → string` takes a `StyleConfig` and
emits one tag per line, joined by newlines (no trailing newline).
Empty config → empty string.

| Section          | Output                                       |
| ---------------- | -------------------------------------------- |
| `stylesheets[]`  | `<link rel="stylesheet" href="...">` entries |
| `inline[]`       | `<style>...css...</style>` blocks            |

External `<link>` first, then `<style>` — external sheets start
loading earlier and the cascade resolves predictably regardless
of where in `<head>` this block lands.

## Quick start

```ts
import { generateStyleTags } from "@coding-adventures/forme-aot-style-tag-emitter";

generateStyleTags({
  stylesheets: [
    { href: "/reset.css" },
    { href: "/main.css",
      integrity: "sha384-oqVuAfXRKap7fdgcCY5uykM6+R9GqQ8K/uxy9rx7HNQlGYl1kPzQho1wx4JwY8wC",
      crossorigin: "anonymous" },
    { href: "/print.css", media: "print" },
  ],
  inline: [
    { css: ":root { --c: #0a0a0a; }" },
    { media: "(prefers-color-scheme: dark)", css: ":root { --c: #fafafa; }" },
  ],
});
```

Produces:

```html
<link rel="stylesheet" href="/reset.css">
<link rel="stylesheet" href="/main.css" integrity="sha384-..." crossorigin="anonymous">
<link rel="stylesheet" href="/print.css" media="print">
<style>:root { --c: #0a0a0a; }</style>
<style media="(prefers-color-scheme: dark)">:root { --c: #fafafa; }</style>
```

## Validation

Two-pass fail-fast. The validator walks every entry before
emitting any tag — an exception means the caller has nothing to
write.

### URL accept-list (`href`)

`http://...` or `https://...` (scheme case-insensitive) or
root-relative `/path` (NOT `//host`, NOT `/\host`). Also rejects
ASCII control bytes (`\x00-\x1F`, `\x7F`) — otherwise
`escapeHtmlAttr` would silently strip them and change the URL.

### SRI integrity

Format: `<algo>-<base64>` with algo ∈ `{sha256, sha384, sha512}`
(or multiple space-separated). Standard base64 only (no URL-safe
`- _`). Base64 length must match the algo's expected digest
size **AND** the per-algo padding count must be exact (1 / 0 / 2
`=` chars respectively).

The per-algo padding check is the subtle one — a sha256 string
with `==` instead of `=` is the right total length (44 chars) but
decodes to 31 bytes, and browsers silently disable SRI rather
than throw. We catch this at the emitter.

The internal algo table is a `Map` (not a plain object) so
attacker-supplied algo names like `"__proto__"`, `"toString"`,
`"hasOwnProperty"` can't walk `Object.prototype` and bypass the
unknown-algo branch.

### `crossorigin` allowlist

`anonymous` | `use-credentials`.

### `media` query

Pure pass-through (HTML-attribute-escaped). We don't gatekeep
CSS media-query syntax — there's no usable pure-JS parser, and
locking callers out of valid queries would do more harm than the
attack surface (caller-controlled `media` attributes can't
escape attribute context once HTML-attr-escaped).

### Inline CSS body

Emitted verbatim between `<style>` and `</style>` (CSS is
parsed as raw text by browsers; escaping `<`/`>`/`&` would
corrupt selectors). The validator rejects any literal
`</style` sequence followed by whitespace, `>`, or `/`
(case-insensitive) — that's the only sequence the HTML parser
treats as a style-block close, and allowing it would let
attacker-controlled CSS smuggle arbitrary HTML and JS into the
page. If you genuinely need `</style>` in a CSS string literal,
use the CSS escape `\3C/style>` which preserves meaning without
matching this pattern.

### `disabled` boolean

When `true`, emits the bare `disabled` attribute. Browsers
download the stylesheet but don't apply it until JS toggles the
`disabled` property.

## Security posture

Seven concerns explicitly addressed:

1. **URL scheme injection.** `validateStyleHref` rejects every
   non-`http(s)://` / non-root-relative URL.
2. **HTML attribute injection.** Every interpolated value
   (including `media`, validated SRI, validated crossorigin,
   validated `href`) passes through `escapeHtmlAttr`.
3. **SRI integrity format validation** with **per-algo padding
   count** — silent-disable defence.
4. **Object-prototype walk defence.** Algo table is a `Map`.
5. **`</style>` rejection** in inline CSS — the canonical XSS
   sink for caller-supplied style blocks. Case-insensitive
   match against `</style` followed by `[\s>/]` mirrors the
   HTML parser's actual close-tag recognition.
6. **Control bytes in `href` rejected** at the validator (not
   silently stripped by escape).
7. **Fail-fast.** Validation completes for every entry BEFORE
   any tag is emitted.

## Behavioural notes

- **Empty config** → empty string.
- **Output order**: stylesheets first, then inline.
- **Attribute order** per `<link>`:
  `rel → href → media → integrity → crossorigin → disabled`.
- **Reproducibility.** Same input → byte-identical output.
- **Inline CSS body is NOT escaped.** Browsers parse `<style>`
  contents as raw text; escaping would corrupt CSS.

## v0 simplifications (documented)

- **No CSS minification / autoprefixing.** Caller pre-processes
  CSS before passing it in.
- **No CSS-syntax validation.** We only reject the
  HTML-injection vector (`</style>`); valid CSS is the caller's
  responsibility.
- **No `<link rel="preload" as="style">`.** That's a resource
  hint and lives in `forme-aot-meta-link-tags`.
- **No `title` attribute** on `<link>` (rarely used; deferred).
- **No `blocking="render"`** attribute (low adoption).

## Tests

122 tests across two files. **100% line / 100% branch** coverage
on all source files with logic.

## Capabilities

`[]` — pure transform.

## How it fits in the stack

Sibling of `forme-aot-script-tag-emitter` (script tags),
`forme-aot-meta-link-tags` (meta / link tags), and
`forme-aot-rss-discovery-link` (feed discovery). Higher-level
FM00 head builders compose all of these to produce the final
`<head>`.
