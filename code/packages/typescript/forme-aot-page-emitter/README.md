# @coding-adventures/forme-aot-page-emitter

Per-page artefact emitter for the Forme **AOT compiler** (FM06 §5).
Takes a `Map<pageId, CssArtifact>` from
[`forme-aot-css-slicer`](../forme-aot-css-slicer) (or the
incremental-cache wrapper) and writes per-page CSS (and optional
HTML wrappers) to a `distDir`.

Fourth FM06 package — the missing piece that turns in-memory
artefacts into on-disk files a static-site host can serve.

## Quick start

```ts
import { slicePerPage } from "@coding-adventures/forme-aot-css-slicer";
import { emitPages } from "@coding-adventures/forme-aot-page-emitter";
import { mkdir } from "node:fs/promises";

await mkdir("./dist", { recursive: true });

const { artefacts } = slicePerPage(doc, [
  { id: "/index.html",    usedRuleIds: ["body", "headline"] },
  { id: "/blog/post.html", usedRuleIds: ["body", "headline", "code"] },
], { activeContexts: ["screen"] });

const { written, totalBytes } = await emitPages("./dist", artefacts, {
  writeHtml: true,
  htmlBody: (pageId) => renderBodyFor(pageId),
});

console.log(`wrote ${written.size} pages, ${totalBytes} bytes total`);
// dist/index.html
// dist/index.css
// dist/blog/post.html
// dist/blog/post.css
```

## Route → file path mapping

| pageId               | HTML path               | CSS path                |
|----------------------|-------------------------|-------------------------|
| `/`                  | `index.html`            | `index.css`             |
| `/about`             | `about.html`            | `about.css`             |
| `/about.html`        | `about.html`            | `about.css`             |
| `/blog/`             | `blog/index.html`       | `blog/index.css`        |
| `/blog/post.html`    | `blog/post.html`        | `blog/post.css`         |
| `/blog/post`         | `blog/post.html`        | `blog/post.css`         |
| `/_next/static/x`    | `_next/static/x.html`   | `_next/static/x.css`    |

Rules:
- Leading `/` stripped (the `distDir` IS the root).
- Trailing `/` → append `index.html`.
- `.html` extension preserved if already present in the basename;
  otherwise added.
- The matching CSS file sits next to the HTML with `.css` extension.

## Options

```ts
interface EmitOptions {
  writeHtml?: boolean;                            // default false
  htmlBody?: (pageId: string) => string;          // default () => ""
}
```

When `writeHtml: true`, each page emits a minimal wrapper:

```html
<!doctype html>
<html><head>
  <meta charset="utf-8">
  <link rel="stylesheet" href="<basename>.css">
</head><body>
  <htmlBody-output>
</body></html>
```

The `htmlBody(pageId)` callback returns the BODY content as an HTML
fragment.  **We don't sanitise it** — legitimate use cases need
literal `<` / `&` in markup.  Caller is responsible for their own
HTML escaping per the standard XSS-prevention guidance.

## Route validation (security-critical)

pageIds are validated **before any filesystem op**.  Rejected:

- empty string
- any path segment equal to `..` or `.` (path traversal / hidden file)
- segments containing NUL, ASCII control chars, or backslash
  (Windows ambiguity)
- absolute paths: `\\...`, `//...`, or `<letter>:...` (Windows drive)

Defence in depth: after `path.join`, the resolved path is verified
to still live under `distDir` (`path.resolve(out).startsWith(distAbs + sep)`).

## IO injection

```ts
interface EmitIO {
  mkdir(dir: string, opts: { recursive: true }): Promise<void>;
  writeFile(file: string, contents: string): Promise<void>;
}

await emitPages(distDir, artefacts, options, customIO);
```

Optional fourth parameter.  When omitted, production wiring uses
`node:fs/promises`.  Tests in this package use an in-memory
implementation so 32 tests run without touching disk.

## Capabilities — `["fs"]`

Writes per-page CSS and optional HTML wrappers under the
caller-supplied `distDir`.  Reads nothing.  No network, no shell.
The fs IO is also injectable via `EmitIO` so callers that want
zero filesystem dependency can supply their own writer.

## Security posture

Four concerns explicitly addressed:

1. **Path traversal via pageId.**  `assertValidPageId` rejects
   `..` / `.` segments, NUL bytes, control chars, backslashes, and
   absolute paths BEFORE any filesystem op.  Defence in depth:
   `path.resolve(cssFull).startsWith(distAbs + path.sep)` check
   after `path.join` catches anything that slipped through.
2. **HTML attribute injection.**  The CSS href in the `<link>` tag
   is HTML-attribute-escaped (`&` → `&amp;`, `<` → `&lt;`, etc.) —
   so a pageId like `r&d` produces a safe `href="r&amp;d.css"`.
3. **`htmlBody` is caller-trusted.**  Documented as NOT sanitised;
   the caller's own templating layer owns HTML escaping for body
   content.
4. **No `child_process`, no `eval`, no `require()` of user paths,
   no symlink resolution.**

## Tests

32 tests in `page-emitter.test.ts`:

- Basic CSS write (4 — per-page output, byteSize, totalBytes,
  empty map, recursive mkdir)
- Route → file path mapping (7 parameterised cases)
- HTML wrapper (5 — opt-in default, structure, htmlBody callback,
  empty body, totalBytes includes HTML)
- pageId validation (9 — empty, `..`, `.`, NUL, control char,
  backslash, Windows drive, double-slash absolute, leading
  backslash)
- Pre-existing files overwritten (1)
- IO injection (2)
- Page iteration order preserved (1)
- HTML attribute escaping (1)
- 50-page stress (1)
- pageId without leading slash (1)

Coverage: **98.07% line / 93.02% branch** — above the FM04 §14.4
≥95% line target.  Uncovered lines are the defensive
"path.resolve says we escaped distDir" throw (unreachable given
`assertValidPageId` already rejected the input) and the production
default-IO callbacks (tests use the in-memory IO).

## Spec adherence

Implements FM06 §5 (per-page artefact emission).  No spec
divergences.

## v0 simplifications

- **HTML wrapper is minimal** — just doctype, charset meta, CSS
  link, and the body callback's output.  Consumers wanting fuller
  HTML (title, OpenGraph, hreflang, etc.) layer their own emit on
  top via `htmlBody`.
- **Single CSS link per page.**  Multi-stylesheet pages would need
  a richer emit signature.
- **No hashing in filenames.**  Cache-busting hash suffixes
  (`index.<hash>.css`) would be a downstream concern that wraps
  this emitter.
