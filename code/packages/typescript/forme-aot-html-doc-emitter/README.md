# @coding-adventures/forme-aot-html-doc-emitter

> FM00 v0 final-assembly emitter — wraps `<head>` + `<body>`
> string chunks into a complete `<!doctype html>...</html>`
> document, with `lang` / `dir` / extra-attr support.

Eighteenth FM00 v0 stage package. Pure transform — no I/O, no
fs, no network, no env, no shell.

## What it does

`generateHtmlDocument(config) → string` takes a `HtmlDocConfig`
and emits a complete HTML document as a newline-joined string.

| Field       | Required? | Validation                              |
| ----------- | --------- | --------------------------------------- |
| `head`      | yes       | string (passthrough — NOT escaped)      |
| `body`      | yes       | string (passthrough — NOT escaped)      |
| `lang`      | no        | conservative BCP-47-shaped regex        |
| `dir`       | no        | `ltr` \| `rtl` \| `auto`                |
| `htmlAttrs` | no        | extra `<html>` attrs, validated         |
| `bodyAttrs` | no        | extra `<body>` attrs, validated         |

`head` and `body` are passthrough because they're already
trusted output from upstream FM00 emitters (`forme-aot-style-tag-emitter`,
`forme-aot-meta-link-tags`, `forme-aot-script-tag-emitter`,
`forme-aot-rss-discovery-link`) that did their own validation +
escaping. The attribute maps **do** get full validation.

## Quick start

```ts
import { generateHtmlDocument } from "@coding-adventures/forme-aot-html-doc-emitter";

generateHtmlDocument({
  lang: "en-US",
  dir: "ltr",
  head: [
    `<meta charset="utf-8">`,
    `<title>Hello</title>`,
    `<link rel="stylesheet" href="/main.css">`,
  ].join("\n"),
  body: [
    `<header><h1>Hello</h1></header>`,
    `<main><p>World</p></main>`,
  ].join("\n"),
  htmlAttrs: { "data-theme": "dark" },
  bodyAttrs: { class: "page", id: "home" },
});
```

Produces (verbatim, newline-separated):

```html
<!doctype html>
<html lang="en-US" dir="ltr" data-theme="dark">
<head>
<meta charset="utf-8">
<title>Hello</title>
<link rel="stylesheet" href="/main.css">
</head>
<body class="page" id="home">
<header><h1>Hello</h1></header>
<main><p>World</p></main>
</body>
</html>
```

## Validation

Two-pass fail-fast. The validator walks the entire config
before emitting any string — an exception means the caller has
nothing to write.

### `lang`

Conservative BCP-47 subset: one ASCII-alpha primary subtag
(1–8 chars), optionally followed by dash-separated alphanumeric
subsequent subtags. Covers `en`, `en-US`, `zh-Hant-HK`, `pt-BR`,
`de-CH-1996`, etc. Doesn't cover extensions / private-use
(rare; v1 may add).

### `dir`

Allowlist: `ltr` | `rtl` | `auto`. Case-sensitive.

### `htmlAttrs` / `bodyAttrs` keys

The attribute-key validator is the security-critical piece —
keys go straight into the rendered tag (`<html ${key}="...">`)
so they MUST be constrained.

Allowed:
- Lowercase ASCII letters / digits / dashes / colons only.
- Must start with a letter.
- Length 1–64.

Rejected:
- **Reserved** keys (`lang`, `dir`, `xmlns`) — use the dedicated
  config fields instead.
- **Any `on*` key** — event-handler namespace is an
  attacker-controlled JS execution sink (`onload`, `onclick`,
  `onerror`, ...). The entire `on*` prefix is rejected, not just
  the known names — defends against future-spec event handlers
  and obscure variants.

### `htmlAttrs` / `bodyAttrs` values

- Must be strings.
- ASCII control bytes rejected (otherwise `escapeHtmlAttr` would
  silently strip them).
- Every value passes through `escapeHtmlAttr` at render time.

### `head` / `body`

Required string. **Not escaped** — they're trusted upstream
output. If you're feeding caller-controlled raw HTML through
this field, escape it yourself first (or, better, run it
through a sanitiser).

## Security posture

Six concerns explicitly addressed:

1. **HTML attribute injection.** Every interpolated attribute
   value passes through `escapeHtmlAttr`. Hard-coded literals
   (`<html`, `<head>`, `<body`, `<!doctype html>`) are
   caller-uninfluenced. Attribute names go through the
   identifier-shape gate before reaching the tag.
2. **Event-handler injection.** The entire `on*` namespace is
   rejected outright. No way for a caller to ship
   `<body onload="...">` through `bodyAttrs`.
3. **Reserved-key shadowing.** `lang` / `dir` / `xmlns` cannot
   be supplied via the attribute map — they'd shadow the
   validator-checked dedicated fields. Caller has to use the
   typed field.
4. **Attribute-key injection.** Key shape regex `^[a-z][a-z0-9\-:]{0,63}$`
   prevents `x" onclick="alert(1)`-style escapes. No space, no
   quote, no `>`, no `=` — all rejected.
5. **Prototype pollution.** `Object.keys()` (own enumerable
   only, no prototype walk). Resolved attrs land in a `Map`,
   not a plain object.
6. **Fail-fast.** Full validation pass completes before any
   string concatenation — no half-formed `<!doctype html>` can
   reach the output buffer.

## Behavioural notes

- **Output layout** is fixed: 9 lines, newline-separated
  (`<!doctype html>`, `<html …>`, `<head>`, head, `</head>`,
  `<body …>`, body, `</body>`, `</html>`).
- **Attribute order on `<html>`**: `lang → dir → extras`
  (extras in `Object.keys` insertion order).
- **Attribute order on `<body>`**: extras in insertion order.
- **`head` / `body` are emitted verbatim** — caller controls
  any internal escaping.
- **Same input → byte-identical output.** No input mutation.

## v0 simplifications (documented)

- **No XHTML / XML self-closing tags.** HTML Living Standard
  syntax only.
- **No `<head>` / `<body>` content validation** — passthrough
  trusted upstream output.
- **No `<noscript>` fallback shell** — caller can add via
  `body`.
- **No BCP-47 extensions / private-use subtags** in `lang`
  validation (conservative subset).

## Tests

116 tests across two files. **100% line / 100% branch** coverage
on all source files with logic.

## Capabilities

`[]` — pure transform.

## How it fits in the stack

This is the **final** assembly stage in the FM00 v0 page
pipeline. Upstream:

- `forme-aot-meta-link-tags` — generic `<meta>` / `<link>` head tags.
- `forme-aot-style-tag-emitter` — `<link rel="stylesheet">` + `<style>`.
- `forme-aot-script-tag-emitter` — `<script>` tags.
- `forme-aot-rss-discovery-link` — feed `<link rel="alternate">` tags.

Caller composes head + body from these, passes both to this
emitter to get the final HTML document.
