/**
 * validate.ts — URL accept-list, rel/as/crossorigin allowlists.
 *
 * The URL validator is the security-critical piece: every
 * tag attribute that browsers may auto-fetch (`href`, `src`)
 * must pass through it.  `javascript:` / `data:` / `file:` /
 * protocol-relative are rejected with `TypeError` and never
 * reach the output buffer.
 *
 * @module validate
 */

import type {
  CrossOrigin,
  IconRel,
  ResourceHintAs,
  ResourceHintRel,
} from "./types.js";

const ICON_REL_ALLOWLIST: ReadonlySet<string> = new Set([
  "icon",
  "shortcut icon",
  "apple-touch-icon",
  "mask-icon",
]);

const HINT_REL_ALLOWLIST: ReadonlySet<string> = new Set([
  "preload",
  "prefetch",
  "preconnect",
  "dns-prefetch",
  "modulepreload",
]);

const HINT_AS_ALLOWLIST: ReadonlySet<string> = new Set([
  "script",
  "style",
  "image",
  "font",
  "fetch",
  "document",
  "audio",
  "video",
  "track",
  "worker",
]);

const CROSSORIGIN_ALLOWLIST: ReadonlySet<string> = new Set([
  "anonymous",
  "use-credentials",
]);

/**
 * Validate a URL against http(s)://-or-root-relative.
 *
 * Same logic as `forme-aot-rss-discovery-link` / sitemap /
 * manifest.  Rejects `javascript:`, `data:`, `file:`,
 * `vbscript:`, protocol-relative `//host`, backslash-variant
 * `/\host`, bare relative `about`, empty string, non-string.
 *
 * `field` is the dotted field path (`"canonical"`,
 * `"icons[2].href"`, ...) used in the error message so callers
 * can pinpoint the offending input.
 */
export function validateUrl(url: unknown, field: string): string {
  if (typeof url !== "string" || url.length === 0) {
    throw new TypeError(
      `forme-aot-meta-link-tags: ${field} must be a non-empty string; got ${
        url === null ? "null" : typeof url
      }`,
    );
  }
  if (isHttpUrl(url)) return url;
  if (isRootRelative(url)) return url;
  const shown = url.length > 200 ? `${url.slice(0, 200)}…` : url;
  throw new TypeError(
    `forme-aot-meta-link-tags: ${field} must be http(s):// or root-relative /path; got ${JSON.stringify(shown)}`,
  );
}

/**
 * Allowlist check for `<link rel="icon">` rel variants.
 * Comparison is case-sensitive — HTML spec uses lowercase
 * canonical forms.
 */
export function validateIconRel(value: unknown, field: string): IconRel {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-meta-link-tags: ${field} must be a string; got ${typeof value}`,
    );
  }
  if (!ICON_REL_ALLOWLIST.has(value)) {
    throw new TypeError(
      `forme-aot-meta-link-tags: ${field} must be one of [${
        [...ICON_REL_ALLOWLIST].join(", ")
      }]; got ${JSON.stringify(value)}`,
    );
  }
  return value as IconRel;
}

/**
 * Allowlist check for resource-hint `rel`.
 */
export function validateHintRel(value: unknown, field: string): ResourceHintRel {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-meta-link-tags: ${field} must be a string; got ${typeof value}`,
    );
  }
  if (!HINT_REL_ALLOWLIST.has(value)) {
    throw new TypeError(
      `forme-aot-meta-link-tags: ${field} must be one of [${
        [...HINT_REL_ALLOWLIST].join(", ")
      }]; got ${JSON.stringify(value)}`,
    );
  }
  return value as ResourceHintRel;
}

/**
 * Allowlist check for `<link rel="preload" as="...">`.
 * Required when `rel === "preload"` / `"modulepreload"`,
 * ignored otherwise.
 */
export function validateHintAs(value: unknown, field: string): ResourceHintAs {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-meta-link-tags: ${field} must be a string; got ${typeof value}`,
    );
  }
  if (!HINT_AS_ALLOWLIST.has(value)) {
    throw new TypeError(
      `forme-aot-meta-link-tags: ${field} must be one of [${
        [...HINT_AS_ALLOWLIST].join(", ")
      }]; got ${JSON.stringify(value)}`,
    );
  }
  return value as ResourceHintAs;
}

/**
 * Allowlist check for `crossorigin` attribute value.
 */
export function validateCrossOrigin(value: unknown, field: string): CrossOrigin {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-meta-link-tags: ${field} must be a string; got ${typeof value}`,
    );
  }
  if (!CROSSORIGIN_ALLOWLIST.has(value)) {
    throw new TypeError(
      `forme-aot-meta-link-tags: ${field} must be one of [${
        [...CROSSORIGIN_ALLOWLIST].join(", ")
      }]; got ${JSON.stringify(value)}`,
    );
  }
  return value as CrossOrigin;
}

/**
 * Generic optional-string field check.  Returns the validated
 * string, or `undefined` if the input is `undefined`.  Throws
 * for any other non-string.
 */
export function validateOptionalString(value: unknown, field: string): string | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-meta-link-tags: ${field} must be a string; got ${typeof value}`,
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
