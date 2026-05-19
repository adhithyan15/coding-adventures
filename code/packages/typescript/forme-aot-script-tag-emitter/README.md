# @coding-adventures/forme-aot-script-tag-emitter

> FM00 v0 emitter — `ScriptTag` config → HTML `<script src="...">`
> tag string, with SRI integrity validation + crossorigin /
> referrerpolicy / async / defer / nomodule support.

Sixteenth FM00 v0 stage package. Pure transform — no I/O, no fs,
no network, no env, no shell.

## What it does

`generateScriptTags(input) → string` takes either a single
`ScriptTag` descriptor or an array of them and emits one
`<script>` tag per entry, joined by newlines (no trailing
newline). Empty array → empty string.

| Field            | Output behaviour                                          |
| ---------------- | --------------------------------------------------------- |
| `src`            | required. `http(s)://` or root-relative `/path`           |
| `type`           | `module` \| `importmap`                                   |
| `integrity`      | SRI `sha256-...` / `sha384-...` / `sha512-...` (space-separated for multiple) |
| `crossorigin`    | `anonymous` \| `use-credentials`                          |
| `async`          | boolean attr                                              |
| `defer`          | boolean attr                                              |
| `nomodule`       | boolean attr                                              |
| `referrerpolicy` | spec-defined enum                                         |

Attribute order in output is fixed:
`type → src → integrity → crossorigin → referrerpolicy → async → defer → nomodule`.

## Quick start

```ts
import { generateScriptTags } from "@coding-adventures/forme-aot-script-tag-emitter";

// Single module script with SRI:
generateScriptTags({
  src: "https://cdn.example.com/app.js",
  type: "module",
  integrity: "sha384-oqVuAfXRKap7fdgcCY5uykM6+R9GqQ8K/uxy9rx7HNQlGYl1kPzQho1wx4JwY8wC",
  crossorigin: "anonymous",
});
// <script type="module" src="https://cdn.example.com/app.js" integrity="sha384-..." crossorigin="anonymous"></script>

// App + analytics + legacy fallback:
generateScriptTags([
  { src: "/main.js",   type: "module", integrity: "sha384-...", crossorigin: "anonymous" },
  { src: "/legacy.js", nomodule: true, defer: true },
  { src: "https://analytics.example.com/a.js", async: true, referrerpolicy: "no-referrer" },
]);
```

## Validation

Two-pass fail-fast. The validator walks the entire input array
first; if anything throws, the caller gets a `TypeError` and
**no output is ever produced** — there's no risk of a half-formed
`<script>` reaching the page.

### URL accept-list (`src`)

`http://...` or `https://...` (scheme case-insensitive), or
root-relative `/path` (NOT `//host`, NOT `/\host`). Everything
else throws.

ASCII control bytes (`\x00-\x1F`, `\x7F`) are also rejected
up-front — otherwise `/\tevil` would pass the root-relative
check (`url[1]` is tab, not `/` or `\`) and become `/evil`
after HTML-attribute escape, silently redirecting to a
different file.

### SRI integrity

Format: one or more whitespace-separated `<algo>-<base64>`
tokens. Algo ∈ `{sha256, sha384, sha512}`. Base64 charset
restricted to `[A-Za-z0-9+/]` plus padding (NO URL-safe
`-` `_` variants — the SRI spec uses standard base64). Base64
length must match the algo's expected digest size (44 / 64 / 88
chars respectively) **with exactly the right number of `=`
padding characters** for that algo (1 / 0 / 2 respectively).
The per-algo padding check is essential — a sha256 string with
`==` instead of `=` decodes to 31 bytes (not 32) and browsers
silently disable SRI rather than throwing. We catch this at the
emitter.

Multiple hashes in one `integrity` are allowed and emitted
verbatim; the browser picks the strongest it supports.

### `type` allowlist

`module` | `importmap`. Classic scripts (no `type=`) come from
omitting the field. Legacy MIMEs (`text/javascript`,
`application/javascript`) are intentionally NOT in the allowlist
— they're equivalent to omission in modern browsers, and
accepting them widens the surface for typo-driven bugs.

### `crossorigin` allowlist

`anonymous` | `use-credentials`.

### `referrerpolicy` allowlist

The eight values defined by the Referrer Policy spec:
`no-referrer`, `no-referrer-when-downgrade`, `origin`,
`origin-when-cross-origin`, `same-origin`, `strict-origin`,
`strict-origin-when-cross-origin`, `unsafe-url`.

### `async` + `defer` conflict

The HTML spec says when both are set on a classic script, `defer`
is silently ignored. We treat setting both as a caller bug and
throw — emitting both bytes hides the bug downstream.

## Security posture

Eight concerns explicitly addressed (pre-push review found
three issues — all fixed in this initial release):

1. **URL scheme injection.** Every `src` runs through
   `validateScriptSrc`. `javascript:`, `data:`, `file:`,
   `vbscript:`, protocol-relative, backslash-variant, bare
   relative, empty, non-string all rejected with `TypeError`.
2. **HTML attribute injection.** Every interpolated value
   passes through `escapeHtmlAttr` (single-pass char-class
   replacement + ASCII control-byte strip). Attacker-controlled
   strings can't break out of the attribute quote.
3. **SRI integrity format validation.** Caller can't accidentally
   ship a malformed `integrity=` attribute (which would silently
   disable SRI in some browsers and allow MITM substitution).
   Wrong algo → throw; wrong base64 length → throw; non-base64
   chars → throw; **wrong per-algo padding** → throw (a sha256
   string with `==` is the right total length but decodes to
   31 bytes; browsers silently disable SRI rather than reject).
4. **Object-prototype walk defence.** Internal algo table is a
   `Map`, not a plain object — so `"__proto__"` / `"toString"`
   / `"hasOwnProperty"` as algo names don't return truthy
   values from `Object.prototype` and bypass the unknown-algo
   branch.
5. **Allowlists.** `type`, `crossorigin`, `referrerpolicy` all
   restrict to spec-defined sets. Defends against typo-driven
   bugs that browsers might treat permissively.
6. **ASCII control bytes in `src` rejected at the validator** —
   prevents `escapeHtmlAttr` from silently stripping them and
   redirecting to a different URL.
7. **`async` + `defer` conflict** rejected as caller bug.
8. **Fail-fast.** Validation completes for every entry BEFORE
   any tag is emitted.

## Behavioural notes

- **Empty array** → empty string.
- **Single object** is treated as a one-element array.
- **Boolean attributes** (`async`, `defer`, `nomodule`) emit as
  bare attribute names (`async`) — the HTML spec canonical form.
  `false` / `undefined` omits them entirely.
- **Reproducibility.** Same input → byte-identical output. No
  hidden randomness, no input mutation.

## v0 simplifications (documented)

- **No inline `<script>...code...</script>`.** External `src`
  only — inline has a fundamentally different trust boundary
  (caller-supplied code executes verbatim) and belongs in a
  separate emitter.
- **No `blocking="render"`** attribute (recently-shipped, low
  adoption; v1 may add).
- **No `fetchpriority`** attribute (deferred).
- **No SRI hash computation** — caller supplies the integrity
  string. This package only validates format.

## Tests

139 tests across two files. **100% line / 100% branch** coverage
on all source files with logic.

## Capabilities

`[]` — pure transform.

## How it fits in the stack

Sibling of `forme-aot-meta-link-tags` (head `<meta>`/`<link>` tags)
and `forme-aot-rss-discovery-link` (`<link rel="alternate">` feed
tags). Higher-level FM00 head builders compose these to produce
the final `<head>`.
