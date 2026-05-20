/**
 * types.ts — public signatures for the page bundle emitter.
 *
 * A "page bundle" is the deploy-time representation of a static
 * site: a route table mapping URL paths to file output paths,
 * with content metadata (size, hash, content-type, last-modified)
 * for each page.  The downstream deploy tool reads this manifest
 * to decide which files to write where.
 *
 * Pages are passed as in-memory strings (the FM00 pipeline holds
 * everything in memory until the deploy step); the manifest
 * doesn't include the page bodies themselves — just the
 * metadata.  This keeps the manifest small enough to inspect /
 * diff between builds.
 *
 * @module types
 */

/**
 * One page in the bundle.
 *
 *   - `route`        — required.  Root-relative URL path,
 *                      e.g. `"/"`, `"/about"`, `"/posts/x"`.
 *                      MUST start with `/`; MUST NOT contain
 *                      `..` segments, `//` (protocol-relative
 *                      hint), or `\` (Windows / backslash
 *                      variant).
 *   - `html`         — required.  The page's HTML string (the
 *                      output of `generateHtmlDocument(...)`
 *                      from `forme-aot-html-doc-emitter`).
 *                      Passthrough — the bundle emitter hashes
 *                      and measures it but never escapes it.
 *   - `lastmod`      — optional `lastModified` ISO 8601 string.
 *                      Pass-through into the manifest;
 *                      validated only as "must be a string"
 *                      (date-format validation is the caller's
 *                      job).
 *   - `contentType`  — optional MIME content-type.  Defaults
 *                      to `"text/html; charset=utf-8"`.  Pure
 *                      pass-through string (validated only as
 *                      "must be a string"; allowlist would lock
 *                      callers out of valid MIMEs).
 */
export interface PageEntry {
  readonly route: string;
  readonly html: string;
  readonly lastmod?: string;
  readonly contentType?: string;
}

/**
 * Top-level config consumed by `generatePageBundle`.
 *
 *   - `pages`    — required array of page entries.  Empty
 *                  array yields an empty manifest.
 *   - `baseUrl`  — optional canonical site URL.  When provided,
 *                  the manifest includes a top-level `baseUrl`
 *                  field; downstream tooling can use it to
 *                  prefix routes when generating sitemap / RSS
 *                  / etc.  Validated as http(s):// only.
 */
export interface PageBundleConfig {
  readonly pages: readonly PageEntry[];
  readonly baseUrl?: string;
}

/**
 * One entry in the emitted route table (manifest).
 *
 *   - `route`       — the input route (preserved verbatim).
 *   - `outputPath`  — relative file path (no leading `/`)
 *                     derived deterministically from route:
 *                       `/`         → `"index.html"`
 *                       `/about`    → `"about/index.html"`
 *                       `/p/x`      → `"p/x/index.html"`
 *                       `/p/x.html` → `"p/x.html"`  (extension
 *                                                    preserved
 *                                                    when the
 *                                                    last segment
 *                                                    already has
 *                                                    one)
 *   - `contentType` — defaulted if absent on input.
 *   - `sizeBytes`   — UTF-8 byte length of `html`.
 *   - `sha256`      — base64-encoded SHA-256 of `html`.
 *   - `lastmod`     — present only if the input had it.
 */
export interface RouteEntry {
  readonly route: string;
  readonly outputPath: string;
  readonly contentType: string;
  readonly sizeBytes: number;
  readonly sha256: string;
  readonly lastmod?: string;
}

/**
 * The decoded manifest shape — what `parsePageBundle` would
 * return (we don't export a parser here, but the shape is
 * documented for downstream consumers).
 */
export interface PageBundleManifest {
  readonly version: 1;
  readonly baseUrl?: string;
  readonly routes: Readonly<Record<string, RouteEntry>>;
}
