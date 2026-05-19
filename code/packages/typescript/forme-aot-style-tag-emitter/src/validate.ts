/**
 * validate.ts — URL, SRI integrity, crossorigin, inline-CSS
 * validators.
 *
 * The SRI validator is structurally identical to the one in
 * `forme-aot-script-tag-emitter` — same Map-backed algo
 * lookup (defends against Object.prototype walks like
 * `__proto__`), same per-algo padding check (defends against
 * silent SRI disable in browsers).
 *
 * The inline-CSS validator rejects `</style>` because allowing
 * it would let arbitrary HTML follow inside the `<style>`
 * block — the canonical XSS sink for caller-supplied CSS.
 *
 * @module validate
 */

import type { CrossOrigin } from "./types.js";

const CROSSORIGIN_ALLOWLIST: ReadonlySet<string> = new Set([
  "anonymous",
  "use-credentials",
]);

// Map-backed so attacker-supplied algo names like `"__proto__"`,
// `"toString"`, `"hasOwnProperty"` can't walk Object.prototype
// and return truthy values from inherited properties.
const SRI_ALGOS: ReadonlyMap<string, number> = new Map([
  ["sha256", 32],
  ["sha384", 48],
  ["sha512", 64],
]);

// Standard base64 only — SRI does NOT permit URL-safe `- _` variants.
const BASE64_RE = /^[A-Za-z0-9+/]+={0,2}$/;

// Reject ASCII control bytes anywhere in a URL — otherwise
// `escapeHtmlAttr` would silently strip them and change the
// URL's meaning (e.g. `/path\x00.css` → `/path.css`).
// eslint-disable-next-line no-control-regex
const URL_CONTROL_RE = /[\x00-\x1F\x7F]/;

// Reject literal `</style>` (case-insensitive) in inline CSS.
// `<\/style[\s>]` matches the HTML parser's actual close-tag
// recognition: `</style` followed by whitespace or `>`.  This
// is the only sequence that can end the style block; matching
// it exactly avoids false positives on benign CSS like
// `content: "</style>"` only when that genuinely-attacker-
// controlled string is naked in source.  We err on the strict
// side — if you need a literal `</style>` in CSS, use
// `\3C/style>` (CSS escape) which preserves meaning without
// matching this pattern.
const STYLE_CLOSE_RE = /<\/style[\s>\/]/i;

/**
 * URL accept-list (same logic as the sibling emitters).
 * `field` is threaded through for clear error messages
 * (`stylesheets[2].href`).
 */
export function validateStyleHref(url: unknown, field: string): string {
  if (typeof url !== "string" || url.length === 0) {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} must be a non-empty string; got ${
        url === null ? "null" : typeof url
      }`,
    );
  }
  if (URL_CONTROL_RE.test(url)) {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} must not contain ASCII control bytes (\\x00-\\x1F, \\x7F)`,
    );
  }
  if (isHttpUrl(url)) return url;
  if (isRootRelative(url)) return url;
  const shown = url.length > 200 ? `${url.slice(0, 200)}…` : url;
  throw new TypeError(
    `forme-aot-style-tag-emitter: ${field} must be http(s):// or root-relative /path; got ${JSON.stringify(shown)}`,
  );
}

/**
 * Validate an SRI `integrity` string.  Same format and checks
 * as `forme-aot-script-tag-emitter`.
 */
export function validateIntegrity(value: unknown, field: string): string {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} must be a string; got ${typeof value}`,
    );
  }
  const trimmed = value.trim();
  if (trimmed.length === 0) {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} must be non-empty`,
    );
  }
  const tokens = trimmed.split(/\s+/);
  for (const tok of tokens) {
    validateSriToken(tok, field);
  }
  return tokens.join(" ");
}

function validateSriToken(token: string, field: string): void {
  const dashIdx = token.indexOf("-");
  if (dashIdx <= 0 || dashIdx >= token.length - 1) {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} token must be "<algo>-<base64>"; got ${JSON.stringify(token)}`,
    );
  }
  const algo = token.slice(0, dashIdx);
  const b64 = token.slice(dashIdx + 1);
  const expectedBytes = SRI_ALGOS.get(algo);
  if (expectedBytes === undefined) {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} algo must be one of [${
        [...SRI_ALGOS.keys()].join(", ")
      }]; got ${JSON.stringify(algo)}`,
    );
  }
  if (!BASE64_RE.test(b64)) {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} base64 has invalid characters; got ${JSON.stringify(b64)}`,
    );
  }
  const expectedB64Len = Math.ceil(expectedBytes / 3) * 4;
  if (b64.length !== expectedB64Len) {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} ${algo} expects ${expectedB64Len}-char base64; got ${b64.length}`,
    );
  }
  // Per-algo padding count check — sha256 needs exactly 1 `=`,
  // sha384 exactly 0, sha512 exactly 2.  Without this check a
  // wrong-padded string passes length but decodes to a different
  // byte length and browsers silently disable SRI.
  const expectedPad = (3 - (expectedBytes % 3)) % 3;
  const actualPad = b64.length - b64.replace(/=+$/, "").length;
  if (actualPad !== expectedPad) {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} ${algo} requires ${expectedPad} '=' padding char${expectedPad === 1 ? "" : "s"}; got ${actualPad}`,
    );
  }
}

/**
 * Validate `crossorigin` value.
 */
export function validateCrossOrigin(value: unknown, field: string): CrossOrigin {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} must be a string; got ${typeof value}`,
    );
  }
  if (!CROSSORIGIN_ALLOWLIST.has(value)) {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} must be one of [${
        [...CROSSORIGIN_ALLOWLIST].join(", ")
      }]; got ${JSON.stringify(value)}`,
    );
  }
  return value as CrossOrigin;
}

/**
 * Validate inline CSS body.  Rejects any literal `</style`
 * (case-insensitive) followed by whitespace, `>`, or `/` —
 * those are the only sequences the HTML parser recognises as
 * a style close tag.  Callers who genuinely need `</style>` in
 * a CSS string literal should use the CSS escape `\3C/style>`
 * which preserves meaning without matching this pattern.
 */
export function validateInlineCss(css: unknown, field: string): string {
  if (typeof css !== "string") {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} must be a string; got ${typeof css}`,
    );
  }
  if (STYLE_CLOSE_RE.test(css)) {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} contains a literal </style> sequence; this would close the style block early.  Use \\3C/style> as a CSS escape instead.`,
    );
  }
  return css;
}

/**
 * Optional string field check — `undefined` passes through,
 * any other non-string throws.
 */
export function validateOptionalString(value: unknown, field: string): string | undefined {
  if (value === undefined) return undefined;
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-style-tag-emitter: ${field} must be a string; got ${typeof value}`,
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
