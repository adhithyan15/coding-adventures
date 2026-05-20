/**
 * validate.ts — lang / dir / attribute-key / attribute-map
 * validators.
 *
 * The attribute-key validator is the security-critical piece.
 * Keys go straight into the rendered tag (`<html ${key}="...">`)
 * so they MUST be constrained.  Without the constraint an
 * attacker-controlled key like `onload`, `style="x:1" onload`,
 * or `__proto__` would either ship an event handler or land in
 * the wrong prototype slot.  Our key shape:
 *
 *   - Lowercase ASCII letters / digits / dashes / colons only.
 *   - Must start with a letter.
 *   - Length 1..64 (cap protects against pathological inputs in
 *     error messages).
 *   - Reserved keys (`lang`, `dir`, `xmlns`) rejected so the
 *     dedicated config fields stay the single source of truth.
 *   - Any `on*` key rejected (event handlers are
 *     attacker-controlled JS execution sinks).
 *
 * @module validate
 */

import type { DocDirection } from "./types.js";

const DIR_ALLOWLIST: ReadonlySet<string> = new Set(["ltr", "rtl", "auto"]);

// Conservative BCP-47 subset: primary alpha subtag, optional
// dash-separated alphanumeric subsequent subtags.  Doesn't
// cover every legal BCP-47 form (extensions, private-use, etc.)
// but covers ~all real-world tags people actually write
// (`en`, `en-US`, `zh-Hant-HK`, `pt-BR`, `de-CH-1996`).
const BCP47_RE = /^[A-Za-z]{1,8}(?:-[A-Za-z0-9]{1,8})*$/;

// Attribute-name shape (lowercase, identifier-ish).  Allows
// dashes (`data-*`, `aria-*`) and colons (XML-namespaced attrs
// like `xml:lang`).  Length cap = 64.
const ATTR_KEY_RE = /^[a-z][a-z0-9\-:]{0,63}$/;

// Reserved attribute names — handled by dedicated config fields
// so the caller can't supply them via the attribute map and
// shadow the validator-checked values.
const RESERVED_ATTRS: ReadonlySet<string> = new Set([
  "lang",
  "dir",
  "xmlns",
]);

// Reject ASCII control bytes anywhere in an attribute value —
// `escapeHtmlAttr` would silently strip them and that could
// change the value's meaning.
// eslint-disable-next-line no-control-regex
const ATTR_VALUE_CONTROL_RE = /[\x00-\x1F\x7F]/;

/**
 * Validate the `lang` field.  Returns the validated tag.
 * Throws for non-string, empty, or syntactically-invalid tag.
 */
export function validateLang(value: unknown): string {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-html-doc-emitter: lang must be a string; got ${typeof value}`,
    );
  }
  if (value.length === 0) {
    throw new TypeError(
      `forme-aot-html-doc-emitter: lang must be non-empty`,
    );
  }
  if (!BCP47_RE.test(value)) {
    throw new TypeError(
      `forme-aot-html-doc-emitter: lang must be a BCP-47-shaped tag (e.g. "en", "en-US", "zh-Hant-HK"); got ${JSON.stringify(value)}`,
    );
  }
  return value;
}

/**
 * Validate the `dir` field against the allowlist.
 */
export function validateDir(value: unknown): DocDirection {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-html-doc-emitter: dir must be a string; got ${typeof value}`,
    );
  }
  if (!DIR_ALLOWLIST.has(value)) {
    throw new TypeError(
      `forme-aot-html-doc-emitter: dir must be one of [${
        [...DIR_ALLOWLIST].join(", ")
      }]; got ${JSON.stringify(value)}`,
    );
  }
  return value as DocDirection;
}

/**
 * Validate an attribute key.  Throws for:
 *   - non-string
 *   - empty / over-length
 *   - bad shape (uppercase, leading digit, special chars)
 *   - reserved (`lang`, `dir`, `xmlns`)
 *   - any `on*` event handler
 */
export function validateAttrKey(key: unknown, field: string): string {
  if (typeof key !== "string") {
    throw new TypeError(
      `forme-aot-html-doc-emitter: ${field} attribute key must be a string; got ${typeof key}`,
    );
  }
  if (key.length === 0) {
    throw new TypeError(
      `forme-aot-html-doc-emitter: ${field} attribute key must be non-empty`,
    );
  }
  if (!ATTR_KEY_RE.test(key)) {
    throw new TypeError(
      `forme-aot-html-doc-emitter: ${field} attribute key must be lowercase ASCII letter/digit/dash/colon, starting with a letter, length 1..64; got ${JSON.stringify(key)}`,
    );
  }
  if (RESERVED_ATTRS.has(key)) {
    throw new TypeError(
      `forme-aot-html-doc-emitter: ${field} attribute key "${key}" is reserved; use the dedicated config field instead`,
    );
  }
  // `onload`, `onclick`, `onerror`, ... — any attribute starting
  // with "on" followed by alpha is treated as an event handler
  // by HTML parsers.  Reject the whole `on*` namespace so
  // callers can't accidentally (or intentionally) ship one.
  if (key.startsWith("on")) {
    throw new TypeError(
      `forme-aot-html-doc-emitter: ${field} attribute key "${key}" is in the on* event-handler namespace; rejected (attacker-controlled JS execution sink)`,
    );
  }
  return key;
}

/**
 * Validate an attribute value.  String required, control bytes
 * rejected (otherwise `escapeHtmlAttr` would silently strip
 * them).
 */
export function validateAttrValue(value: unknown, field: string): string {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-html-doc-emitter: ${field} attribute value must be a string; got ${typeof value}`,
    );
  }
  if (ATTR_VALUE_CONTROL_RE.test(value)) {
    throw new TypeError(
      `forme-aot-html-doc-emitter: ${field} attribute value must not contain ASCII control bytes (\\x00-\\x1F, \\x7F)`,
    );
  }
  return value;
}
