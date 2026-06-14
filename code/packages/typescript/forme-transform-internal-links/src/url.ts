/**
 * url.ts — internal-link detection and resolved-URL validation.
 *
 * Two small but security-critical helpers:
 *
 *   - `isInternalSlug(url)` — true if the LinkNode destination
 *     is "internal" (an absolute path like `/about` or
 *     `/blog/post`) and so a candidate for rewriting.
 *   - `assertResolvedUrl(url)` — verifies the URL the resolver
 *     returned is safe to emit into an `href` attribute.
 *
 * The resolver is caller-supplied code we can't audit.  A buggy
 * (or hostile) resolver could return `javascript:alert(1)` for
 * an innocent-looking slug.  Validating the output before
 * splicing it back into the AST gives us a chokepoint that
 * downstream renderers can rely on.
 *
 * @module url
 */

/**
 * True if `url` is a root-relative internal slug.
 *
 * Accept set:
 *   - Starts with `/` (single slash — site-root).
 *   - Does NOT start with `//` (protocol-relative).
 *   - Does NOT start with `/\` (some browsers normalise `\` to
 *     `/`, turning `/\evil.com` into `//evil.com` — same
 *     ambiguous-scheme attack surface).
 *   - Is a non-empty string.
 *
 * Reject set:
 *   - `http://...`, `https://...` (already absolute — no rewrite).
 *   - `mailto:`, `tel:`, `javascript:`, `data:` (not internal).
 *   - `//example.com/x` (protocol-relative — ambiguous scheme).
 *   - `/\example.com/x` (backslash variant — same risk).
 *   - `relative/path`, `./about` (bare relative — author should
 *     normalise to `/about` first).
 *   - Empty string.
 *
 * Why no fragment-only links (`#section`)?  Those are intra-
 * document references handled by `autolinkHeadings`; this
 * package leaves them alone.
 */
export function isInternalSlug(url: string): boolean {
  if (typeof url !== "string" || url.length === 0) return false;
  if (url.length >= 2 && url[0] === "/" && (url[1] === "/" || url[1] === "\\")) {
    return false;
  }
  return url[0] === "/";
}

/**
 * Assert that a resolver-returned URL is safe to emit into an
 * `href` attribute.  Throws `TypeError` if not.
 *
 * Accept set (mirrors the rest of the Forme stage cluster):
 *   - `http://...` / `https://...`  (case-insensitive scheme)
 *   - Root-relative paths (`/about`) — the resolver chose not to
 *     fully resolve; keep as relative.
 *
 * Reject set:
 *   - `javascript:`, `data:`, `file:`, `vbscript:`, `mailto:`,
 *     `tel:`, etc.
 *   - Protocol-relative (`//host/path`).
 *   - Bare relative (`about`, `./about`).
 *   - Empty string.
 *   - Non-string.
 *
 * The error message includes the offending URL so resolver
 * authors can debug.  Truncates long URLs to 200 chars to keep
 * the error readable.
 */
export function assertResolvedUrl(url: unknown): asserts url is string {
  if (typeof url !== "string" || url.length === 0) {
    throw new TypeError(
      `forme-transform-internal-links: resolver returned ${
        url === null ? "null" : typeof url === "string" ? "empty string" : typeof url
      }; expected http(s):// or root-relative /path`,
    );
  }
  if (isHttpUrl(url) || isRootRelative(url)) return;
  const shown = url.length > 200 ? `${url.slice(0, 200)}…` : url;
  throw new TypeError(
    `forme-transform-internal-links: resolver returned unsafe URL ${
      JSON.stringify(shown)
    }; expected http(s):// or root-relative /path`,
  );
}

function isHttpUrl(url: string): boolean {
  // Case-insensitive `http://` / `https://` prefix check — match
  // by lowering only the first 8 chars to avoid a full-string
  // `toLowerCase()` allocation in the hot path.
  const head = url.slice(0, 8).toLowerCase();
  return head.startsWith("http://") || head.startsWith("https://");
}

function isRootRelative(url: string): boolean {
  // Must start with exactly one slash and NOT have a second
  // `/` or `\` (some browsers normalise `\` to `/`, so
  // `/\evil.com` round-trips to `//evil.com` — same
  // ambiguous-scheme attack as protocol-relative).
  if (url === "/") return true;
  if (url.length < 2 || url[0] !== "/") return false;
  return url[1] !== "/" && url[1] !== "\\";
}
