/**
 * page-emitter.ts — `emitPages(distDir, artefacts, options, io?)` (FM06 §5).
 *
 * Takes a `Map<pageId, CssArtifact>` (produced by
 * `forme-aot-css-slicer.slicePerPage` or
 * `forme-aot-incremental-cache.sliceWithCache`) and writes one CSS
 * file per page under `distDir`, plus an optional HTML wrapper
 * that loads the CSS.
 *
 * ## Route → file path mapping
 *
 *   pageId `"/"`              → `index.html`            + `index.css`
 *   pageId `"/about"`          → `about.html`            + `about.css`
 *   pageId `"/blog/post.html"` → `blog/post.html`        + `blog/post.css`
 *   pageId `"/blog/"`          → `blog/index.html`       + `blog/index.css`
 *
 * Rules:
 *   - Leading `/` stripped (the dist dir IS the root).
 *   - Trailing `/` → append `index.html`.
 *   - `.html` extension preserved if already present in the
 *     basename; otherwise added.
 *   - The matching CSS file sits next to the HTML with `.css`
 *     extension.
 *
 * ## Route validation (security-critical)
 *
 * pageIds are validated before any filesystem op.  Rejected:
 *   - empty string
 *   - any path segment equal to `..` (path traversal)
 *   - any path segment starting with `.` (hidden file)
 *   - any segment containing NUL, backslash (Windows ambiguity),
 *     or ASCII control chars
 *   - absolute paths (anything starting with `\\`, `//`, or
 *     `<letter>:` Windows drive)
 *
 * The default scheme is permissive enough for real-world routes
 * (`/blog/2026/05/post.html`, `/_next/static/x.html`) while
 * defending against attacker-controlled pageIds escaping `distDir`.
 *
 * ## HTML wrapper
 *
 * When `options.writeHtml: true`, each page emits:
 *
 *   <!doctype html>
 *   <html><head>
 *     <meta charset="utf-8">
 *     <link rel="stylesheet" href="<basename>.css">
 *   </head><body>
 *     <htmlBody-output>
 *   </body></html>
 *
 * The `htmlBody(pageId)` callback returns the BODY content (HTML
 * fragment).  Default: empty body.  Caller is responsible for the
 * body's own escaping — we don't sanitise it (it would prevent
 * legitimate HTML content from rendering).
 *
 * ## IO injection
 *
 * `emitPages(.., io?)` accepts an optional `EmitIO` so tests can
 * use in-memory writers.  When omitted, production wiring uses
 * `node:fs/promises`.
 *
 * @module page-emitter
 */

import { promises as fs } from "node:fs";
import * as path from "node:path";
import type { CssArtifact } from "@coding-adventures/forme-aot-css-slicer";

// ─── Public types ────────────────────────────────────────────────────────

/** Side-effect surface — injectable for testability. */
export interface EmitIO {
  mkdir(dir: string, opts: { recursive: true }): Promise<void>;
  writeFile(file: string, contents: string): Promise<void>;
}

export interface EmitOptions {
  /** Emit an HTML wrapper that loads the CSS.  Default `false`. */
  readonly writeHtml?: boolean;
  /**
   * Callback returning the BODY HTML fragment for a given pageId.
   * Only called when `writeHtml: true`.  Default: returns empty
   * string.
   *
   * The body is NOT sanitised — callers are responsible for their
   * own HTML escaping per the standard XSS-prevention guidance.
   * We don't sanitise here because legitimate use cases need
   * literal `<` / `&` in markup.
   */
  readonly htmlBody?: (pageId: string) => string;
}

/** What `emitPages` wrote for one page. */
export interface PageEmit {
  readonly cssPath: string;
  /** Only present when `options.writeHtml === true`. */
  readonly htmlPath?: string;
  readonly byteSize: number;
}

export interface EmitResult {
  readonly written: ReadonlyMap<string, PageEmit>;
  readonly totalBytes: number;
}

// ─── Public entry point ──────────────────────────────────────────────────

/**
 * Emit per-page artefacts under `distDir`.  Caller's responsibility
 * to ensure `distDir` exists and is writable.  Sub-directories ARE
 * created on demand.
 *
 * Returns a `EmitResult` with per-page paths + total bytes written.
 */
export async function emitPages(
  distDir: string,
  artefacts: ReadonlyMap<string, CssArtifact>,
  options: EmitOptions = {},
  io: EmitIO = defaultIO,
): Promise<EmitResult> {
  const written = new Map<string, PageEmit>();
  let totalBytes = 0;
  const writeHtml = options.writeHtml === true;
  const htmlBody = options.htmlBody ?? (() => "");

  // Iterate in caller-supplied order (Map preserves insertion order).
  for (const [pageId, art] of artefacts) {
    assertValidPageId(pageId);
    const routePath = pageIdToRoutePath(pageId);

    // CSS file path: same dir + same basename + .css extension.
    const cssRel  = `${stripHtmlExt(routePath)}.css`;
    const htmlRel = routePath;

    const cssFull  = path.join(distDir, cssRel);
    const htmlFull = path.join(distDir, htmlRel);

    // Defence in depth: even after path.join, verify BOTH the CSS
    // and HTML resolved paths still live under distDir.  We use
    // `path.relative` rather than `startsWith` because the latter
    // is bypassable by sibling-prefix attacks (`distDir = "/tmp/d"`
    // → `/tmp/d-evil/...` also starts-with `/tmp/d`).
    // `path.relative` produces a `..`-prefixed path (or an
    // absolute path on different-drive Windows scenarios) when the
    // target is outside; we explicitly reject those forms.
    const distAbs = path.resolve(distDir);
    for (const candidate of [cssFull, htmlFull]) {
      const rel = path.relative(distAbs, path.resolve(candidate));
      if (rel.startsWith("..") || path.isAbsolute(rel)) {
        throw new Error(`forme-aot-page-emitter: pageId ${JSON.stringify(pageId)} resolves outside distDir`);
      }
    }

    await io.mkdir(path.dirname(cssFull), { recursive: true });
    await io.writeFile(cssFull, art.css);
    let pageBytes = Buffer.byteLength(art.css, "utf8");

    let htmlPath: string | undefined;
    if (writeHtml) {
      // Compute the relative `<link href>` so the HTML works from
      // any URL root (we never absolute-href; that's the consumer's
      // policy choice).
      const cssHref = path.basename(cssRel);
      const body = htmlBody(pageId);
      const html = renderHtmlWrapper(cssHref, body);
      await io.writeFile(htmlFull, html);
      htmlPath = htmlFull;
      pageBytes += Buffer.byteLength(html, "utf8");
    }

    written.set(pageId, htmlPath !== undefined
      ? { cssPath: cssFull, htmlPath, byteSize: pageBytes }
      : { cssPath: cssFull, byteSize: pageBytes });
    totalBytes += pageBytes;
  }

  return { written, totalBytes };
}

// ─── Default IO ──────────────────────────────────────────────────────────

const defaultIO: EmitIO = {
  mkdir: (dir, opts) => fs.mkdir(dir, opts).then(() => undefined),
  writeFile: (file, contents) => fs.writeFile(file, contents, "utf8"),
};

// ─── pageId validation ──────────────────────────────────────────────────

const SEGMENT_FORBIDDEN_RE = /[\x00-\x1F\x7F\\]/;
const DRIVE_LETTER_RE = /^[A-Za-z]:/;

function assertValidPageId(pageId: string): void {
  if (typeof pageId !== "string" || pageId.length === 0) {
    throw new Error(`forme-aot-page-emitter: pageId must be a non-empty string (got ${JSON.stringify(pageId)})`);
  }
  // Reject Windows-style absolute paths even on POSIX (defence in
  // depth — a consumer that round-trips routes through Windows
  // shouldn't accidentally smuggle a drive letter through us).
  if (DRIVE_LETTER_RE.test(pageId)) {
    throw new Error(`forme-aot-page-emitter: pageId looks like a Windows absolute path: ${JSON.stringify(pageId)}`);
  }
  // Reject double-leading-slash / backslash absolute paths.
  if (pageId.startsWith("//") || pageId.startsWith("\\")) {
    throw new Error(`forme-aot-page-emitter: pageId must not be an absolute path: ${JSON.stringify(pageId)}`);
  }
  // Split on `/` and validate each segment.  We split on `/` only
  // (not `path.sep`) because pageIds are URL-shaped, not OS-shaped.
  const stripped = pageId.startsWith("/") ? pageId.slice(1) : pageId;
  const segments = stripped.split("/").filter((s) => s.length > 0);
  for (const seg of segments) {
    if (seg === ".." || seg === ".") {
      throw new Error(`forme-aot-page-emitter: pageId contains ${JSON.stringify(seg)} segment: ${JSON.stringify(pageId)}`);
    }
    if (SEGMENT_FORBIDDEN_RE.test(seg)) {
      throw new Error(`forme-aot-page-emitter: pageId segment contains forbidden chars (NUL, control, or backslash): ${JSON.stringify(pageId)}`);
    }
  }
}

/**
 * Map a route-shaped pageId to a relative file path (HTML).
 *   "/"                → "index.html"
 *   "/about"           → "about.html"
 *   "/blog/"           → "blog/index.html"
 *   "/blog/post.html"  → "blog/post.html"
 *   "/_next/static/x"  → "_next/static/x.html"
 */
function pageIdToRoutePath(pageId: string): string {
  let p = pageId.startsWith("/") ? pageId.slice(1) : pageId;
  if (p.length === 0) return "index.html";
  if (p.endsWith("/")) return `${p}index.html`;
  // If the basename already ends in `.html`, keep it.  Otherwise append.
  const base = path.posix.basename(p);
  if (base.endsWith(".html")) return p;
  return `${p}.html`;
}

function stripHtmlExt(p: string): string {
  return p.endsWith(".html") ? p.slice(0, -".html".length) : p;
}

// ─── HTML wrapper ────────────────────────────────────────────────────────

function renderHtmlWrapper(cssHref: string, body: string): string {
  // cssHref is derived from pageId via stripHtmlExt + basename, so
  // it's always a sane filename.  Body is caller-controlled.
  return [
    `<!doctype html>`,
    `<html><head>`,
    `  <meta charset="utf-8">`,
    `  <link rel="stylesheet" href="${escapeHtmlAttr(cssHref)}">`,
    `</head><body>`,
    body,
    `</body></html>`,
    ``,
  ].join("\n");
}

const HTML_ATTR_ESCAPE_MAP: Readonly<Record<string, string>> = Object.freeze({
  "&": "&amp;",
  "<": "&lt;",
  ">": "&gt;",
  "\"": "&quot;",
  "'": "&#39;",
});
const HTML_ATTR_RE = /[&<>"']/g;
function escapeHtmlAttr(s: string): string {
  return s.replace(HTML_ATTR_RE, (ch) => HTML_ATTR_ESCAPE_MAP[ch]!);
}
