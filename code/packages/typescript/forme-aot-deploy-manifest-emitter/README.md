# @coding-adventures/forme-aot-deploy-manifest-emitter

> FM00 v0 deploy-composition emitter — combines the page bundle,
> sitemap, robots, web app manifest, and any extra files into a
> single deterministic deploy-record JSON.

Twentieth FM00 v0 stage package. Pure transform — no I/O, no
fs, no network. Uses Node's built-in `node:crypto` for SHA-256.

## What it does

`generateDeployManifest(config) → string` reads the outputs of
the upstream FM00 v0 emitters (page bundle JSON, sitemap.xml,
robots.txt, Web App Manifest JSON, plus any caller-supplied
extra files) and produces a single byte-deterministic JSON
manifest. A downstream deploy runner reads the manifest and
applies every file to the deploy target (S3, Netlify, generic
fs, ...).

## Quick start

```ts
import { generateDeployManifest } from "@coding-adventures/forme-aot-deploy-manifest-emitter";
import { generatePageBundle } from "@coding-adventures/forme-aot-page-bundle-emitter";
import { generateSitemap }     from "@coding-adventures/forme-aot-sitemap-emitter";

const pageBundle = generatePageBundle({
  baseUrl: "https://example.com",
  pages: [
    { route: "/",      html: indexHtml },
    { route: "/about", html: aboutHtml },
  ],
});

const sitemapXml = generateSitemap([{ url: "/" }, { url: "/about" }], "https://example.com");

const manifest = generateDeployManifest({
  pageBundle,
  sitemapXml,
  robotsTxt: "User-agent: *\nAllow: /\n",
  manifestJson: '{"name":"Example"}',
  extraFiles: [
    { outputPath: "favicon.ico", content: faviconBase64, contentType: "image/x-icon" },
    { outputPath: ".well-known/security.txt", content: "Contact: mailto:a@example.com\n", contentType: "text/plain" },
  ],
});
```

Output:

```json
{
  "version": 1,
  "baseUrl": "https://example.com",
  "fileCount": 7,
  "totalSizeBytes": 12345,
  "files": {
    ".well-known/security.txt": { "outputPath": "...", "contentType": "...", "sizeBytes": 31, "sha256": "…=", "source": "extra" },
    "about/index.html":         { "outputPath": "...", ..., "route": "/about", "source": "page-bundle" },
    "favicon.ico":              { "outputPath": "...", ..., "source": "extra" },
    "index.html":               { "outputPath": "...", ..., "route": "/", "source": "page-bundle" },
    "manifest.webmanifest":     { "outputPath": "...", ..., "source": "web-app-manifest" },
    "robots.txt":               { "outputPath": "...", ..., "source": "robots" },
    "sitemap.xml":              { "outputPath": "...", ..., "source": "sitemap" }
  }
}
```

## Input sources

| Field          | Required? | Synthesised output path |
| -------------- | --------- | ----------------------- |
| `pageBundle`   | yes       | per-route (already in the bundle) |
| `sitemapXml`   | no        | `sitemap.xml`           |
| `robotsTxt`    | no        | `robots.txt`            |
| `manifestJson` | no        | `manifest.webmanifest`  |
| `extraFiles`   | no        | caller-specified, validated |

The page bundle is the **source of truth** for HTML pages — we
re-emit its route entries verbatim (`route`, `outputPath`,
`contentType`, `sizeBytes`, `sha256`, `lastmod`). The other
inputs add one synthesised entry each at fixed output paths
above. Extra files cover everything else (favicons,
.well-known/, font subsets, robots.txt overrides, etc.).

## Extra-file output-path validation

The extra-file output-path validator is the security-critical
piece — these paths land on the deploy target's filesystem.

**Accepted** (relative file paths):
- One or more segments joined by `/`.
- Each segment matches `[A-Za-z0-9._~!$&'()*+,;=@-]` (RFC 3986
  unreserved + sub-delims + `@`, **minus** `%` `?` `#` `:`).
- ≤ 2048 chars.

**Rejected** (with explicit error messages):
- Non-string / empty / over-cap.
- Leading `/` (must be relative).
- Leading `~` (home-dir expansion).
- Any `\` (Windows separator).
- `..` segment (path traversal).
- `.` sole segment.
- Empty mid-segment (`a//b`, trailing `/`).
- `:` anywhere — Windows drive separator / HFS+ historic.
  (`https:/evil`, `C:/Windows`, `a/b:c` all rejected.)
- `?`, `#`, whitespace, NUL, unicode, percent-encoded.

## Page-bundle parse safety

`parsePageBundle` uses `JSON.parse` then walks fields by name.
No `for...in` loops, no prototype-chain traversal. Even a
crafted `__proto__` key in the input survives as an own property
on the parsed object — never reaching `Object.prototype`.
Verified by an end-to-end test (`Object.prototype` is checked
for pollution after parsing a sneaky bundle).

## JSON output format

Byte-deterministic:

- 2-space indent, trailing newline.
- Top-level keys: `version → baseUrl (if present) → fileCount → totalSizeBytes → files`.
- `files` sorted by `outputPath` lexicographically.
- Each entry's keys: `outputPath → contentType → sizeBytes → sha256 → source → route (if page-bundle) → lastmod (if present)`.

Same input → byte-identical manifest. Two consecutive builds
diff cleanly: only files that genuinely changed show up.

## Duplicate detection

Throws on any output-path collision:
- Within the page bundle.
- Sitemap path collides with a page-bundle route at `sitemap.xml`.
- Same for `robots.txt`, `manifest.webmanifest`.
- Extra-file path collides with any earlier entry.

The deploy runner must have an unambiguous "what gets written where" — no last-write-wins surprises.

## Security posture

Ten concerns explicitly addressed (pre-push review found three hardening gaps — all fixed before push):

1. **Path traversal.** Extra-file paths validated against strict shape regex + `..` / `.` / empty-segment checks before being written.
2. **Cross-platform path safety.** No `\`, no `:`, no leading `/` or `~`.
3. **Percent-encoding traversal.** `%` not in segment charset; `%2e%2e` etc. rejected wholesale rather than decoded.
4. **Prototype pollution.** `JSON.parse` + `Object.keys` (own enumerable only). No prototype walk.
5. **JSON-syntax injection.** `JSON.stringify` escapes; route/path strings can't break the output structure.
6. **Hashing.** SHA-256 via Node's built-in `node:crypto`. No external dep, no algorithm choice.
7. **Determinism.** Same input → byte-identical output. No nondeterminism source.
8. **Page-bundle outputPath revalidation.** The API accepts any JSON string as `pageBundle` — `parsePageBundle` re-runs the full `validateOutputPath` check on every route's `outputPath` so a malicious / buggy upstream can't smuggle `../etc/passwd` through to the deploy target. Defense-in-depth.
9. **Windows / cross-platform filename safety.** Per-segment checks reject Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1-9`, `LPT1-9` with/without extension, case-insensitive), trailing dot/space (Win32 silently strips), and per-segment > 255 bytes (ext4/APFS/NTFS filename limit).
10. **Prototype-pollution sink hardening.** Output paths literally `__proto__`, `constructor`, `prototype` rejected. `filesObj` accumulator uses `Object.create(null)` so there's no prototype-chain mutation path even if validation ever loosened.

## Behavioural notes

- **Page-bundle hashes** are trusted verbatim (we don't re-hash; the page bundle already did that work).
- **Synthesised entries** (sitemap / robots / manifest) get fresh hashes from the input string.
- **Extra-file `content`** is hashed as a UTF-8 string. Binary callers should base64-encode before passing.
- **No input mutation.**

## v0 simplifications (documented)

- **No deploy-target adapters** (S3, Netlify, Cloudflare Pages, etc.) — that's the next stage (a deploy-runner program).
- **No CDN purge directives** in the manifest.
- **No per-file caching headers / TTLs.** The contentType is enough for v0.
- **No incremental diff vs. previous deploy.** Caller can diff two manifest JSONs externally.

## Tests

112 tests across three files. **100% line / 98.47% branch**
coverage on all source files with logic. The missing branches
are unreachable in practice (sort comparator equality cases
since duplicate paths throw at validation time).

## Capabilities

`[]` — pure transform. Uses Node's built-in `node:crypto`.

## How it fits in the stack

The **final** compose stage of the FM00 v0 pipeline:

```
forme-aot-html-doc-emitter      →  per-page HTML strings
forme-aot-page-bundle-emitter   →  page bundle JSON (routes → hashes)
forme-aot-sitemap-emitter       →  sitemap.xml
forme-aot-robots-emitter        →  robots.txt
forme-aot-manifest-emitter      →  manifest.webmanifest

         ↓ all of the above ↓

forme-aot-deploy-manifest-emitter  ←  YOU ARE HERE
         ↓
deploy runner (next package)  →  S3 / Netlify / fs target
```
