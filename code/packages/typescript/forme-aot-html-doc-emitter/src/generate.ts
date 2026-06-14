/**
 * generate.ts — main `generateHtmlDocument` entry.
 *
 * Two-pass fail-fast: validate the entire config (lang, dir,
 * every htmlAttrs/bodyAttrs key + value), then render the
 * final document.  An exception means the caller has nothing
 * to write — no half-formed `<!doctype html>` reaches disk.
 *
 * Output layout (newline-joined for readability + deterministic
 * diffs):
 *
 *     <!doctype html>
 *     <html lang="…" dir="…" …extra>
 *     <head>
 *     {head}
 *     </head>
 *     <body …extra>
 *     {body}
 *     </body>
 *     </html>
 *
 * `head` and `body` are inserted verbatim — they're already
 * trusted upstream FM00 emitter output.  Attribute order on
 * `<html>` is fixed: `lang → dir → extras` (extras in
 * `Object.keys()` insertion order).  Attribute order on
 * `<body>` is just extras in insertion order.
 *
 * @module generate
 */

import { escapeHtmlAttr } from "./escape.js";
import type { HtmlDocConfig } from "./types.js";
import {
  validateAttrKey,
  validateAttrValue,
  validateDir,
  validateLang,
} from "./validate.js";

interface ResolvedAttrs {
  readonly keys: readonly string[];
  readonly values: ReadonlyMap<string, string>;
}

/**
 * Generate a complete HTML document from a `HtmlDocConfig`.
 *
 * ```ts
 * generateHtmlDocument({
 *   lang: "en",
 *   dir: "ltr",
 *   head: "<title>Hello</title>",
 *   body: "<h1>Hello</h1>",
 * });
 * // <!doctype html>
 * // <html lang="en" dir="ltr">
 * // <head>
 * // <title>Hello</title>
 * // </head>
 * // <body>
 * // <h1>Hello</h1>
 * // </body>
 * // </html>
 * ```
 */
export function generateHtmlDocument(config: HtmlDocConfig): string {
  if (config === null || typeof config !== "object") {
    throw new TypeError(
      `forme-aot-html-doc-emitter: config must be a non-null object; got ${typeof config}`,
    );
  }

  // `head` and `body` are passthrough but they MUST be strings —
  // a `null`/`undefined`/array would corrupt the output.
  if (typeof config.head !== "string") {
    throw new TypeError(
      `forme-aot-html-doc-emitter: head must be a string; got ${typeof config.head}`,
    );
  }
  if (typeof config.body !== "string") {
    throw new TypeError(
      `forme-aot-html-doc-emitter: body must be a string; got ${typeof config.body}`,
    );
  }

  const lang = config.lang === undefined ? undefined : validateLang(config.lang);
  const dir = config.dir === undefined ? undefined : validateDir(config.dir);
  const htmlAttrs = resolveAttrs(config.htmlAttrs, "htmlAttrs");
  const bodyAttrs = resolveAttrs(config.bodyAttrs, "bodyAttrs");

  // Render.
  const htmlOpenParts: string[] = ["<html"];
  if (lang !== undefined) htmlOpenParts.push(`lang="${escapeHtmlAttr(lang)}"`);
  if (dir !== undefined)  htmlOpenParts.push(`dir="${escapeHtmlAttr(dir)}"`);
  for (const key of htmlAttrs.keys) {
    htmlOpenParts.push(`${key}="${escapeHtmlAttr(htmlAttrs.values.get(key)!)}"`);
  }
  const htmlOpen = `${htmlOpenParts.join(" ")}>`;

  const bodyOpenParts: string[] = ["<body"];
  for (const key of bodyAttrs.keys) {
    bodyOpenParts.push(`${key}="${escapeHtmlAttr(bodyAttrs.values.get(key)!)}"`);
  }
  const bodyOpen = `${bodyOpenParts.join(" ")}>`;

  return [
    "<!doctype html>",
    htmlOpen,
    "<head>",
    config.head,
    "</head>",
    bodyOpen,
    config.body,
    "</body>",
    "</html>",
  ].join("\n");
}

function resolveAttrs(
  attrs: Readonly<Record<string, string>> | undefined,
  field: string,
): ResolvedAttrs {
  if (attrs === undefined) return { keys: [], values: new Map() };
  if (attrs === null || typeof attrs !== "object" || Array.isArray(attrs)) {
    throw new TypeError(
      `forme-aot-html-doc-emitter: ${field} must be an object; got ${
        attrs === null ? "null" : Array.isArray(attrs) ? "array" : typeof attrs
      }`,
    );
  }
  // Use `Object.keys` (own enumerable string keys only — won't
  // walk the prototype chain).  Validate each key + value;
  // store in a Map (defensive — avoids any later code-path
  // accidentally exposing prototype-pollution).
  const keys: string[] = [];
  const values = new Map<string, string>();
  for (const key of Object.keys(attrs)) {
    const validatedKey = validateAttrKey(key, field);
    const validatedValue = validateAttrValue(
      (attrs as Record<string, unknown>)[key],
      `${field}["${key}"]`,
    );
    keys.push(validatedKey);
    values.set(validatedKey, validatedValue);
  }
  return { keys, values };
}
