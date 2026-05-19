/**
 * validate.ts — URL, SRI integrity, type / crossorigin /
 * referrerpolicy allowlist validators.
 *
 * SRI integrity is the most subtle bit — `<script integrity>` is
 * the W3C Subresource Integrity spec, format
 * `<algo>-<base64>[ <algo>-<base64> ...]`.  We accept only the
 * three browser-implemented algos (sha256 / sha384 / sha512) and
 * validate the base64 length matches the expected digest size
 * (32 / 48 / 64 bytes respectively).  Multiple hashes in one
 * `integrity` string are allowed (the spec lets browsers pick
 * the strongest one they support).
 *
 * @module validate
 */

import type { CrossOrigin, ReferrerPolicy, ScriptType } from "./types.js";

const SCRIPT_TYPE_ALLOWLIST: ReadonlySet<string> = new Set([
  "module",
  "importmap",
]);

const CROSSORIGIN_ALLOWLIST: ReadonlySet<string> = new Set([
  "anonymous",
  "use-credentials",
]);

const REFERRER_POLICY_ALLOWLIST: ReadonlySet<string> = new Set([
  "no-referrer",
  "no-referrer-when-downgrade",
  "origin",
  "origin-when-cross-origin",
  "same-origin",
  "strict-origin",
  "strict-origin-when-cross-origin",
  "unsafe-url",
]);

// Each algo prefix → expected raw digest byte length.  Base64
// representation length, including any trailing `=` padding, is
// derived from the byte length.
//
// Backed by a `Map` (not a plain object) so attacker-supplied
// keys like `"__proto__"`, `"constructor"`, `"toString"` can't
// walk Object.prototype and accidentally satisfy the
// "is-known-algo" check with a function value.
const SRI_ALGOS: ReadonlyMap<string, number> = new Map([
  ["sha256", 32],
  ["sha384", 48],
  ["sha512", 64],
]);

// Base64 alphabet, case-sensitive.  We accept standard (A-Z a-z 0-9 + /)
// + `=` padding.  SRI does NOT permit URL-safe (- _) variants.
const BASE64_RE = /^[A-Za-z0-9+/]+={0,2}$/;

// Disallow ASCII control bytes (`\x00-\x1F`, `\x7F`) anywhere in
// a URL — `escapeHtmlAttr` would silently strip them downstream
// and change the URL's meaning (e.g. `/path\x00.js` → `/path.js`,
// a different file).  Rejecting up-front means the caller always
// gets the URL they asked for or a TypeError.
// eslint-disable-next-line no-control-regex
const URL_CONTROL_RE = /[\x00-\x1F\x7F]/;

/**
 * Validate the `src` URL against http(s)://-or-root-relative
 * accept-list.  Same logic as the sibling emitters.
 */
export function validateScriptSrc(url: unknown): string {
  if (typeof url !== "string" || url.length === 0) {
    throw new TypeError(
      `forme-aot-script-tag-emitter: src must be a non-empty string; got ${
        url === null ? "null" : typeof url
      }`,
    );
  }
  if (URL_CONTROL_RE.test(url)) {
    throw new TypeError(
      `forme-aot-script-tag-emitter: src must not contain ASCII control bytes (\\x00-\\x1F, \\x7F)`,
    );
  }
  if (isHttpUrl(url)) return url;
  if (isRootRelative(url)) return url;
  const shown = url.length > 200 ? `${url.slice(0, 200)}…` : url;
  throw new TypeError(
    `forme-aot-script-tag-emitter: src must be http(s):// or root-relative /path; got ${JSON.stringify(shown)}`,
  );
}

/**
 * Validate an SRI `integrity` string.
 *
 * Accepts: one or more whitespace-separated `<algo>-<base64>`
 * tokens, where algo ∈ {sha256, sha384, sha512} and the base64
 * decodes to the algo's digest byte length.
 *
 * Trims surrounding whitespace; collapses internal runs of
 * whitespace into single spaces in the returned canonical form
 * for deterministic output.
 *
 * Throws `TypeError` on:
 *   - non-string or empty
 *   - unknown algo prefix (e.g. `md5-...`)
 *   - missing or malformed base64 portion (wrong charset, wrong
 *     length for the named algo)
 */
export function validateIntegrity(value: unknown): string {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-script-tag-emitter: integrity must be a string; got ${typeof value}`,
    );
  }
  const trimmed = value.trim();
  if (trimmed.length === 0) {
    throw new TypeError(
      `forme-aot-script-tag-emitter: integrity must be non-empty`,
    );
  }
  const tokens = trimmed.split(/\s+/);
  for (const tok of tokens) {
    validateSriToken(tok);
  }
  return tokens.join(" ");
}

function validateSriToken(token: string): void {
  const dashIdx = token.indexOf("-");
  if (dashIdx <= 0 || dashIdx >= token.length - 1) {
    throw new TypeError(
      `forme-aot-script-tag-emitter: integrity token must be "<algo>-<base64>"; got ${JSON.stringify(token)}`,
    );
  }
  const algo = token.slice(0, dashIdx);
  const b64 = token.slice(dashIdx + 1);
  const expectedBytes = SRI_ALGOS.get(algo);
  if (expectedBytes === undefined) {
    throw new TypeError(
      `forme-aot-script-tag-emitter: integrity algo must be one of [${
        [...SRI_ALGOS.keys()].join(", ")
      }]; got ${JSON.stringify(algo)}`,
    );
  }
  if (!BASE64_RE.test(b64)) {
    throw new TypeError(
      `forme-aot-script-tag-emitter: integrity base64 has invalid characters; got ${JSON.stringify(b64)}`,
    );
  }
  // Expected base64 length: ceil(bytes / 3) * 4.
  const expectedB64Len = Math.ceil(expectedBytes / 3) * 4;
  if (b64.length !== expectedB64Len) {
    throw new TypeError(
      `forme-aot-script-tag-emitter: integrity ${algo} expects ${expectedB64Len}-char base64; got ${b64.length}`,
    );
  }
  // Per-algo padding check.  Without this, a sha256 string with
  // `==` instead of `=` would still pass the length check but
  // decode to 31 bytes, and browsers would silently disable SRI.
  //   bytes % 3 == 0 → 0 pad chars (e.g. sha384)
  //   bytes % 3 == 1 → 2 pad chars (e.g. sha512)
  //   bytes % 3 == 2 → 1 pad char  (e.g. sha256)
  const expectedPad = (3 - (expectedBytes % 3)) % 3;
  const actualPad = b64.length - b64.replace(/=+$/, "").length;
  if (actualPad !== expectedPad) {
    throw new TypeError(
      `forme-aot-script-tag-emitter: integrity ${algo} requires ${expectedPad} '=' padding char${expectedPad === 1 ? "" : "s"}; got ${actualPad}`,
    );
  }
}

/**
 * Validate `type` against `{module, importmap}` allowlist.
 */
export function validateScriptType(value: unknown): ScriptType {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-script-tag-emitter: type must be a string; got ${typeof value}`,
    );
  }
  if (!SCRIPT_TYPE_ALLOWLIST.has(value)) {
    throw new TypeError(
      `forme-aot-script-tag-emitter: type must be one of [${
        [...SCRIPT_TYPE_ALLOWLIST].join(", ")
      }]; got ${JSON.stringify(value)}`,
    );
  }
  return value as ScriptType;
}

/**
 * Validate `crossorigin` against `{anonymous, use-credentials}`.
 */
export function validateCrossOrigin(value: unknown): CrossOrigin {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-script-tag-emitter: crossorigin must be a string; got ${typeof value}`,
    );
  }
  if (!CROSSORIGIN_ALLOWLIST.has(value)) {
    throw new TypeError(
      `forme-aot-script-tag-emitter: crossorigin must be one of [${
        [...CROSSORIGIN_ALLOWLIST].join(", ")
      }]; got ${JSON.stringify(value)}`,
    );
  }
  return value as CrossOrigin;
}

/**
 * Validate `referrerpolicy` against the Referrer Policy spec
 * enum.
 */
export function validateReferrerPolicy(value: unknown): ReferrerPolicy {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-script-tag-emitter: referrerpolicy must be a string; got ${typeof value}`,
    );
  }
  if (!REFERRER_POLICY_ALLOWLIST.has(value)) {
    throw new TypeError(
      `forme-aot-script-tag-emitter: referrerpolicy must be one of [${
        [...REFERRER_POLICY_ALLOWLIST].join(", ")
      }]; got ${JSON.stringify(value)}`,
    );
  }
  return value as ReferrerPolicy;
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
