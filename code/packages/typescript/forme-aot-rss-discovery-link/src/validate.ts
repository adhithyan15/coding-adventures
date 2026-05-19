/**
 * validate.ts — href URL + type allowlist validators.
 *
 * @module validate
 */

import type { FeedType } from "./types.js";

const FEED_TYPE_ALLOWLIST: ReadonlySet<string> = new Set([
  "application/rss+xml",
  "application/atom+xml",
  "application/json",
]);

/**
 * Validate the feed `href` against http(s)://-or-root-relative
 * accept-list.  Same pattern as `forme-aot-sitemap-emitter` and
 * `forme-aot-manifest-emitter`.
 *
 * Throws `TypeError` with the offending URL (truncated to 200
 * chars) for `javascript:`, `data:`, `file:`, `vbscript:`,
 * protocol-relative, backslash-variant, bare relative, empty,
 * non-string.
 */
export function validateFeedHref(url: unknown): string {
  if (typeof url !== "string" || url.length === 0) {
    throw new TypeError(
      `forme-aot-rss-discovery-link: href must be a non-empty string; got ${
        url === null ? "null" : typeof url
      }`,
    );
  }
  if (isHttpUrl(url)) return url;
  if (isRootRelative(url)) return url;
  const shown = url.length > 200 ? `${url.slice(0, 200)}…` : url;
  throw new TypeError(
    `forme-aot-rss-discovery-link: href must be http(s):// or root-relative /path; got ${JSON.stringify(shown)}`,
  );
}

/**
 * Validate the `type` against the feed MIME allowlist.
 * Returns the canonical (input) value on success; throws
 * `TypeError` otherwise.
 *
 * Comparison is case-sensitive — MIME types are by convention
 * lowercase, and accepting case variants here would obscure
 * data-quality issues in caller manifests.
 */
export function validateFeedType(value: string): FeedType {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-rss-discovery-link: type must be a string; got ${typeof value}`,
    );
  }
  if (!FEED_TYPE_ALLOWLIST.has(value)) {
    throw new TypeError(
      `forme-aot-rss-discovery-link: type must be one of [${
        [...FEED_TYPE_ALLOWLIST].join(", ")
      }]; got ${JSON.stringify(value)}`,
    );
  }
  return value as FeedType;
}

function isHttpUrl(url: string): boolean {
  const head = url.slice(0, 8).toLowerCase();
  return head.startsWith("http://") || head.startsWith("https://");
}

function isRootRelative(url: string): boolean {
  if (url === "/") return true;
  if (url.length < 2 || url[0] !== "/") return false;
  return url[1] !== "/" && url[1] !== "\\";
}
