/**
 * types.ts — public signatures for the deploy manifest emitter.
 *
 * The deploy manifest is the single hand-off file between
 * "build complete" and "deploy starts".  Every file that
 * should land on the target (static host, CDN, S3 bucket, etc.)
 * appears in the manifest with its output path, content type,
 * size, and SHA-256 hash.  A deploy runner reads the manifest,
 * resolves the file contents (either from the in-memory bundle
 * or from a separate content store), and writes them.
 *
 * Inputs are the already-validated outputs of the sibling FM00
 * v0 emitters:
 *   - `pageBundle`  — JSON string from `forme-aot-page-bundle-emitter`
 *                     (the per-page route table with hashes).
 *   - `sitemapXml`  — optional sitemap.xml string.
 *   - `robotsTxt`   — optional robots.txt string.
 *   - `manifestJson` — optional Web App Manifest JSON string.
 *   - `extraFiles`  — optional caller-supplied extra files
 *                     (favicon binaries, .well-known/, etc.).
 *
 * The pageBundle is treated as the **source of truth** for HTML
 * pages — we re-emit its route entries verbatim (route,
 * outputPath, contentType, sizeBytes, sha256, lastmod).  The
 * other inputs add one synthesised entry each at fixed output
 * paths (`sitemap.xml`, `robots.txt`, `manifest.webmanifest`).
 *
 * @module types
 */

/**
 * Extra file entry (caller-supplied).  Used for binary assets
 * the FM00 pipeline doesn't generate — favicons, .well-known/
 * verification files, font subsets, etc.
 *
 *   - `outputPath`  — relative path (no leading `/`), no `..`,
 *                     no `\`, no absolute prefix.
 *   - `content`     — the file contents as a string (binary
 *                     payloads should be base64-encoded by the
 *                     caller; the manifest is content-agnostic
 *                     for size/hash purposes).
 *   - `contentType` — required MIME type.
 *   - `lastmod`     — optional ISO 8601 string (passthrough).
 */
export interface ExtraFile {
  readonly outputPath: string;
  readonly content: string;
  readonly contentType: string;
  readonly lastmod?: string;
}

/**
 * Top-level config consumed by `generateDeployManifest`.
 */
export interface DeployManifestConfig {
  readonly pageBundle: string;
  readonly sitemapXml?: string;
  readonly robotsTxt?: string;
  readonly manifestJson?: string;
  readonly extraFiles?: readonly ExtraFile[];
}

/**
 * One file in the emitted deploy record.
 */
export interface DeployFileEntry {
  readonly outputPath: string;
  readonly contentType: string;
  readonly sizeBytes: number;
  readonly sha256: string;
  /** Original route (only present for HTML pages from the page bundle). */
  readonly route?: string;
  /** Source emitter (page-bundle, sitemap, robots, manifest, extra). */
  readonly source:
    | "page-bundle"
    | "sitemap"
    | "robots"
    | "web-app-manifest"
    | "extra";
  readonly lastmod?: string;
}

/**
 * The decoded deploy manifest shape.  Documented for downstream
 * consumers (we don't export a parser).
 */
export interface DeployManifest {
  readonly version: 1;
  readonly baseUrl?: string;
  readonly fileCount: number;
  readonly totalSizeBytes: number;
  readonly files: Readonly<Record<string, DeployFileEntry>>;
}
