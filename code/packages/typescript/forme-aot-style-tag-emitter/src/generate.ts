/**
 * generate.ts — main `generateStyleTags` entry.
 *
 * Two-pass fail-fast: validate every entry into a fresh
 * resolved array, then render each to its tag string joined by
 * newlines.  No trailing newline.
 *
 * Output order:
 *   1. external `<link rel="stylesheet">` entries (in caller's
 *      order)
 *   2. inline `<style>` entries (in caller's order)
 *
 * External-first is the conventional order — external
 * stylesheets start loading earlier, and the cascade resolves
 * predictably regardless of where in the head this block lands.
 *
 * Attribute order per `<link>`:
 *   `rel → href → media → integrity → crossorigin → disabled`
 *
 * Attribute order per `<style>`:
 *   `media` (if present), then body.
 *
 * @module generate
 */

import { escapeHtmlAttr } from "./escape.js";
import type { InlineStyle, StyleConfig, StylesheetLink } from "./types.js";
import {
  validateCrossOrigin,
  validateInlineCss,
  validateIntegrity,
  validateOptionalString,
  validateStyleHref,
} from "./validate.js";

interface ResolvedLink {
  readonly kind: "link";
  readonly href: string;
  readonly media: string | undefined;
  readonly integrity: string | undefined;
  readonly crossorigin: string | undefined;
  readonly disabled: boolean;
}
interface ResolvedInline {
  readonly kind: "inline";
  readonly css: string;
  readonly media: string | undefined;
}
type Resolved = ResolvedLink | ResolvedInline;

/**
 * Generate `<link rel="stylesheet">` + `<style>` tags from a
 * structured config.  Returns the empty string for an empty
 * config (no fields set).  Throws `TypeError` synchronously
 * for any validation failure before emitting anything.
 *
 * ```ts
 * generateStyleTags({
 *   stylesheets: [
 *     { href: "/main.css" },
 *     { href: "https://cdn.example.com/print.css", media: "print",
 *       integrity: "sha384-...", crossorigin: "anonymous" },
 *   ],
 *   inline: [
 *     { css: ":root { --c: blue; }" },
 *     { media: "(prefers-color-scheme: dark)", css: ":root { --c: lightblue; }" },
 *   ],
 * });
 * ```
 */
export function generateStyleTags(config: StyleConfig): string {
  if (config === null || typeof config !== "object") {
    throw new TypeError(
      `forme-aot-style-tag-emitter: config must be a non-null object; got ${typeof config}`,
    );
  }

  const resolved: Resolved[] = [];

  if (config.stylesheets !== undefined) {
    if (!Array.isArray(config.stylesheets)) {
      throw new TypeError(
        `forme-aot-style-tag-emitter: stylesheets must be an array; got ${typeof config.stylesheets}`,
      );
    }
    for (let i = 0; i < config.stylesheets.length; i++) {
      resolved.push(validateLink(config.stylesheets[i]!, i));
    }
  }

  if (config.inline !== undefined) {
    if (!Array.isArray(config.inline)) {
      throw new TypeError(
        `forme-aot-style-tag-emitter: inline must be an array; got ${typeof config.inline}`,
      );
    }
    for (let i = 0; i < config.inline.length; i++) {
      resolved.push(validateInline(config.inline[i]!, i));
    }
  }

  const lines: string[] = new Array(resolved.length);
  for (let i = 0; i < resolved.length; i++) {
    lines[i] = renderResolved(resolved[i]!);
  }
  return lines.join("\n");
}

function validateLink(link: StylesheetLink, i: number): ResolvedLink {
  if (link === null || typeof link !== "object") {
    throw new TypeError(
      `forme-aot-style-tag-emitter: stylesheets[${i}] must be a non-null object; got ${typeof link}`,
    );
  }
  const href = validateStyleHref(link.href, `stylesheets[${i}].href`);
  const media = validateOptionalString(link.media, `stylesheets[${i}].media`);
  const integrity = link.integrity === undefined
    ? undefined
    : validateIntegrity(link.integrity, `stylesheets[${i}].integrity`);
  const crossorigin = link.crossorigin === undefined
    ? undefined
    : validateCrossOrigin(link.crossorigin, `stylesheets[${i}].crossorigin`);
  const disabled = validateBool(link.disabled, `stylesheets[${i}].disabled`);
  return { kind: "link", href, media, integrity, crossorigin, disabled };
}

function validateInline(entry: InlineStyle, i: number): ResolvedInline {
  if (entry === null || typeof entry !== "object") {
    throw new TypeError(
      `forme-aot-style-tag-emitter: inline[${i}] must be a non-null object; got ${typeof entry}`,
    );
  }
  const css = validateInlineCss(entry.css, `inline[${i}].css`);
  const media = validateOptionalString(entry.media, `inline[${i}].media`);
  return { kind: "inline", css, media };
}

function validateBool(value: unknown, field: string): boolean {
  if (value === undefined || value === false) return false;
  if (value === true) return true;
  throw new TypeError(
    `forme-aot-style-tag-emitter: ${field} must be a boolean; got ${typeof value}`,
  );
}

function renderResolved(r: Resolved): string {
  return r.kind === "link" ? renderLink(r) : renderInline(r);
}

function renderLink(r: ResolvedLink): string {
  const parts: string[] = [
    `<link rel="stylesheet"`,
    `href="${escapeHtmlAttr(r.href)}"`,
  ];
  if (r.media !== undefined)       parts.push(`media="${escapeHtmlAttr(r.media)}"`);
  if (r.integrity !== undefined)   parts.push(`integrity="${escapeHtmlAttr(r.integrity)}"`);
  if (r.crossorigin !== undefined) parts.push(`crossorigin="${escapeHtmlAttr(r.crossorigin)}"`);
  // `disabled` is a boolean attribute — emit bare attribute name.
  if (r.disabled)                  parts.push("disabled");
  return `${parts.join(" ")}>`;
}

function renderInline(r: ResolvedInline): string {
  const open = r.media !== undefined
    ? `<style media="${escapeHtmlAttr(r.media)}">`
    : `<style>`;
  return `${open}${r.css}</style>`;
}
