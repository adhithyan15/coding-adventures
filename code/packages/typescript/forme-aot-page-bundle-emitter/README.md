# @coding-adventures/forme-aot-page-bundle-emitter

> FM00 v0 deploy-stage emitter — wraps multiple HTML pages
> into a deterministic JSON manifest describing where each
> page should be written and its SHA-256 content hash.

Nineteenth FM00 v0 stage package. Pure transform — no I/O, no
fs, no network, no env, no shell. Uses Node's built-in
`node:crypto` for hashing.

## What it does

`generatePageBundle(config) → string` takes an array of pages
and produces a JSON manifest. The downstream deploy tool reads
the manifest, then writes the actual files.

**Manifest shape:**

```json
{
  "version": 1,
  "baseUrl": "https://example.com",
  "routes": {
    "/":           { "route": "/",           "outputPath": "index.html",       "contentType": "text/html; charset=utf-8", "sizeBytes": 15, "sha256": "…=" },
    "/about":      { "route": "/about",      "outputPath": "about/index.html", "contentType": "text/html; charset=utf-8", "sizeBytes": 32, "sha256": "…=" },
    "/feed.xml":   { "route": "/feed.xml",   "outputPath": "feed.xml",         "contentType": "application/rss+xml",      "sizeBytes": 28, "sha256": "…=" },
    "/posts/x":    { "route": "/posts/x",    "outputPath": "posts/x/index.html", … }
  }
}
```

## Quick start

```ts
import { generatePageBundle } from "@coding-adventures/forme-aot-page-bundle-emitter";

const manifest = generatePageBundle({
  baseUrl: "https://example.com",
  pages: [
    { route: "/",          html: "<!doctype html><title>Home</title>" },
    { route: "/about",     html: "<!doctype html><title>About</title>" },
    { route: "/posts/x",   html: "<!doctype html><title>Post</title>", lastmod: "2026-05-19" },
    { route: "/feed.xml",  html: "<?xml?><rss/>", contentType: "application/rss+xml" },
  ],
});

// `manifest` is a UTF-8 JSON string, 2-space indented,
// trailing newline.  Hand it to your deploy tool.
```

## Route → output path

Deterministic derivation. The rule: if the last route segment
has a `.ext`, write to that filename; otherwise treat the route
as a directory and write `index.html` inside it.

| Route          | Output path             |
| -------------- | ----------------------- |
| `/`            | `index.html`            |
| `/about`       | `about/index.html`      |
| `/posts/x`     | `posts/x/index.html`    |
| `/feed.xml`    | `feed.xml`              |
| `/page.html`   | `page.html`             |
| `/file.tar.gz` | `file.tar.gz`           |
| `/.hidden`     | `.hidden/index.html`    |

The "has extension" check looks at the **last segment** for a
`.` at index > 0 (so dotfiles like `.hidden` don't count as
extensions).

## Route validation (path-traversal defence)

The route validator is the security-critical piece. Routes
become file paths via `routeToOutputPath` — any `..` segment or
absolute prefix would let an attacker write outside the deploy
root.

**Accepted:**
- Must start with `/`.
- Bare `/` is OK.
- Segments contain only `[A-Za-z0-9._~!$&'()*+,;=:@-]` (RFC 3986
  unreserved + sub-delims + `:` `@`).
- ≤ 2048 chars total.

**Rejected:**
- Empty / non-string / null.
- Doesn't start with `/`.
- Starts with `//` (protocol-relative).
- Starts with `/\` (backslash variant).
- Contains `\` anywhere (Windows separator).
- Contains `..` as a segment (path traversal).
- Contains `.` as a sole segment.
- Contains `//` mid-path (empty segment).
- Trailing `/` (empty trailing segment).
- Contains `?`, `#`, whitespace, NUL, or any char outside the
  segment charset.

Duplicate routes (after validation) throw.

## `baseUrl` validation

Optional. When provided: `http://...` or `https://...` only,
scheme case-insensitive, ≤ 2048 chars. Everything else throws.

## Hashing

SHA-256 via Node's built-in `node:crypto`, base64-encoded
(standard alphabet, including `+` `/` `=` padding). 44-char
output. No external dependency, no network.

`sizeBytes` is the UTF-8 byte length of the HTML string (via
`TextEncoder`), not the character count.

## JSON output format

Byte-deterministic:

- 2-space indent, trailing newline.
- Top-level key order: `version → baseUrl (if present) → routes`.
- `routes` keys sorted lexicographically by route string.
- Each route entry's keys in fixed order: `route → outputPath →
  contentType → sizeBytes → sha256 → lastmod (if present)`.

Same input → byte-identical output, byte-identical hash.
Reordering input pages doesn't change output.

## Security posture

Six concerns explicitly addressed:

1. **Path traversal.** Routes are validated against a strict
   shape regex + segment-by-segment `..` / `.` / empty checks
   before being converted to file paths. No way to escape the
   deploy root.
2. **Protocol confusion.** Routes starting with `//` (treated as
   URL by some servers) or `/\` (backslash variant) rejected.
3. **Windows path separators.** Any `\` in the route rejected
   — prevents `..\..\etc` on Windows-target deploys.
4. **`baseUrl` scheme injection.** `javascript:` / `data:` /
   `file:` / `ftp:` / scheme-less / non-http(s) all rejected.
5. **Hashing.** SHA-256 via Node's built-in `node:crypto`. No
   network, no third-party dep, no algorithm choice that could
   collide.
6. **Determinism.** Same input → byte-identical output. Diff
   between two builds shows exactly what changed; no spurious
   churn from non-deterministic ordering.

## Behavioural notes

- **HTML body is passthrough** — the bundle hashes / measures
  it but never escapes or validates it. It's already trusted
  upstream FM00 output (from `forme-aot-html-doc-emitter`).
- **Empty `pages: []`** → minimal manifest (`version + routes:
  {}`).
- **Duplicate routes** throw, even if HTML differs (caller bug).
- **No input mutation.** Pages array is read-only.

## v0 simplifications (documented)

- **No gzip / brotli precompression metadata** in the manifest.
  Caller compresses at deploy time.
- **No per-page redirect / rewrite rules.** Routes are 1:1
  with output files.
- **No multi-locale (`/en/`, `/de/`) routing logic** — caller
  pre-flattens.
- **No content-addressed output paths** (e.g.
  `page.abc123.html`). v1 may add as an option.

## Capabilities

`[]` — pure transform.

## How it fits in the stack

The **deploy-stage** companion to `forme-aot-html-doc-emitter`
(which produces individual HTML documents). Upstream:

- `forme-aot-html-doc-emitter` — produces the per-page HTML strings.

Downstream (planned):

- `forme-aot-deploy-manifest-emitter` — combines this page
  bundle with sitemap + robots + RSS into a single deploy
  manifest.

## Tests

98 tests across four files (`validate`, `path`, `hash`,
`generate`). **100% line coverage**, **98.82% branch coverage**
(the missing branch is a sort-comparator equality case that's
unreachable because duplicate routes throw).
