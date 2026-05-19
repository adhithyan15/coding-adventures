/**
 * generate.ts — main `generateSitemap` entry.
 *
 * Algorithm:
 *
 *   1. Normalise `baseUrl` (validates http(s)://, trims
 *      trailing slash).
 *   2. **VALIDATION PASS** (no XML emitted yet): for every
 *      entry, validate `url` (resolve relative to baseUrl,
 *      reject bad schemes), validate `changefreq` (allowlist),
 *      clamp `priority` (range).  Collect resolved entries
 *      into a fresh array.  If anything throws, the caller
 *      gets the error WITHOUT a partial sitemap.
 *   3. **EMIT PASS**: build the XML string from the resolved
 *      entries.  Every interpolated value passes through
 *      `escapeXml`.
 *
 * Output is deterministic — same input → byte-identical XML.
 * Entries are emitted in input order; the caller decides the
 * sort.
 *
 * @module generate
 */

import { escapeXml } from "./escape.js";
import { normaliseBaseUrl, resolveEntryUrl } from "./url.js";
import type { SitemapEntry } from "./types.js";
import { clampPriority, validateChangefreq } from "./validate.js";

interface ResolvedEntry {
  readonly loc: string;
  readonly lastmod: string | undefined;
  readonly changefreq: string | undefined;
  readonly priority: string | undefined;
}

/**
 * Generate the sitemap.xml string for an ordered list of
 * entries.  Returns the XML; caller writes it wherever.
 *
 * Throws `TypeError` on any validation failure (bad URL
 * scheme, bad changefreq, bad baseUrl).  Throws BEFORE any
 * XML is emitted so callers never see a partial document.
 *
 * Reproducibility: same `entries` + same `baseUrl` →
 * byte-identical output.
 *
 * Input arrays are never mutated.
 */
export function generateSitemap(
  entries: readonly SitemapEntry[],
  baseUrl: string,
): string {
  const base = normaliseBaseUrl(baseUrl);

  // Validation pass — fail fast, no partial output.
  const resolved: ResolvedEntry[] = new Array(entries.length);
  for (let i = 0; i < entries.length; i++) {
    const e = entries[i]!;
    resolved[i] = {
      loc: resolveEntryUrl(e.url, base),
      lastmod: e.lastmod,
      changefreq: e.changefreq === undefined ? undefined : validateChangefreq(e.changefreq),
      priority: e.priority === undefined ? undefined : clampPriority(e.priority),
    };
  }

  // Emit pass.
  const parts: string[] = [
    `<?xml version="1.0" encoding="UTF-8"?>`,
    `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">`,
  ];
  for (let i = 0; i < resolved.length; i++) {
    parts.push(renderUrl(resolved[i]!));
  }
  parts.push(`</urlset>`);
  return parts.join("\n");
}

/**
 * Render one `<url>` block.  Order of children matches the
 * sitemap protocol's documented presentation order: loc →
 * lastmod → changefreq → priority.
 */
function renderUrl(e: ResolvedEntry): string {
  const children: string[] = [`  <loc>${escapeXml(e.loc)}</loc>`];
  if (e.lastmod !== undefined) {
    children.push(`  <lastmod>${escapeXml(e.lastmod)}</lastmod>`);
  }
  if (e.changefreq !== undefined) {
    // changefreq is already allowlist-validated, so escape is
    // belt-and-braces (no `&` / `<` could survive validation).
    children.push(`  <changefreq>${escapeXml(e.changefreq)}</changefreq>`);
  }
  if (e.priority !== undefined) {
    // priority is `clampPriority`-formatted (literal "0.0"-"1.0"),
    // so escape is also belt-and-braces.
    children.push(`  <priority>${escapeXml(e.priority)}</priority>`);
  }
  return `<url>\n${children.join("\n")}\n</url>`;
}
