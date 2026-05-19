/**
 * validate.ts — field validators.
 *
 * Three validators:
 *
 *   - `validateManifestUrl` — http(s):// or root-relative path
 *     accept-list; same `/\backslash-variant` defence used by
 *     `forme-transform-internal-links` and the sitemap emitter.
 *   - `validateDisplay` — allowlist of W3C-defined display
 *     modes.
 *   - `validateColor` — hex colour `#rgb`, `#rgba`, `#rrggbb`,
 *     `#rrggbbaa` only.  The CSS spec permits names + rgb() /
 *     hsl() syntax too but hex is the only form universally
 *     supported by all manifest consumers; we restrict for
 *     determinism and to avoid CSS-parsing complexity.
 *
 * @module validate
 */

import type { DisplayMode } from "./types.js";

const DISPLAY_ALLOWLIST: ReadonlySet<string> = new Set([
  "fullscreen",
  "standalone",
  "minimal-ui",
  "browser",
]);

/**
 * Validate a manifest URL field (`start_url`, `scope`,
 * `icons[].src`).  Accepts:
 *
 *   - http(s):// (case-insensitive scheme)
 *   - Root-relative `/path` (NOT `//host`, NOT `/\host`)
 *   - `/` exactly
 *
 * Rejects: empty, non-string, javascript:, data:, file:,
 * vbscript:, protocol-relative, backslash-variant, bare
 * relative.
 *
 * Throws `TypeError` with the offending value (truncated to
 * 200 chars) and the field name in the message.
 */
export function validateManifestUrl(url: unknown, field: string): string {
  if (typeof url !== "string" || url.length === 0) {
    throw new TypeError(
      `forme-aot-manifest-emitter: ${field} must be a non-empty string; got ${
        url === null ? "null" : typeof url
      }`,
    );
  }
  if (isHttpUrl(url)) return url;
  if (isRootRelative(url)) return url;
  const shown = url.length > 200 ? `${url.slice(0, 200)}…` : url;
  throw new TypeError(
    `forme-aot-manifest-emitter: ${field} must be http(s):// or root-relative /path; got ${JSON.stringify(shown)}`,
  );
}

/**
 * Validate `display` against the allowlist.  Case-insensitive
 * (the spec is technically case-sensitive but real-world
 * configs vary).  Returns the lowercased canonical value.
 */
export function validateDisplay(value: string): DisplayMode {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-manifest-emitter: display must be a string; got ${typeof value}`,
    );
  }
  const lower = value.toLowerCase();
  if (!DISPLAY_ALLOWLIST.has(lower)) {
    throw new TypeError(
      `forme-aot-manifest-emitter: display must be one of [${
        [...DISPLAY_ALLOWLIST].join(", ")
      }]; got ${JSON.stringify(value)}`,
    );
  }
  return lower as DisplayMode;
}

/**
 * Hex colour pattern: `#` followed by exactly 3, 4, 6, or 8
 * hex digits.  Case-insensitive.  We use a regex here because
 * (a) it's a simple character class with bounded quantifier
 * (no ReDoS surface), (b) the alternative `charCodeAt` loop
 * would be ~30 lines for a one-line regex with no clarity
 * win.
 */
const HEX_COLOR_RE = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/;

/**
 * Validate a hex colour value (`theme_color`, `background_color`).
 * Accepts `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`.
 *
 * Throws `TypeError` for anything else — CSS names
 * (`"red"`), rgb()/hsl() syntax, named-with-alpha
 * (`"rgba(...)"`).  Hex is the only universally-supported
 * form across all manifest consumers.
 */
export function validateColor(value: string, field: string): string {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-manifest-emitter: ${field} must be a string; got ${typeof value}`,
    );
  }
  if (!HEX_COLOR_RE.test(value)) {
    throw new TypeError(
      `forme-aot-manifest-emitter: ${field} must be a hex colour (#rgb, #rgba, #rrggbb, or #rrggbbaa); got ${JSON.stringify(value)}`,
    );
  }
  return value;
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
