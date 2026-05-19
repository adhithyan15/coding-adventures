/**
 * generate.ts — main `generateScriptTags` entry.
 *
 * Two-pass fail-fast: validate every entry into a fresh resolved
 * array, then render each to a `<script>` tag string joined by
 * newlines.  No trailing newline.
 *
 * Attribute order (deterministic, matches WHATWG conventions):
 *
 *     type → src → integrity → crossorigin → referrerpolicy →
 *     async → defer → nomodule
 *
 * Boolean attributes (`async`, `defer`, `nomodule`) are emitted
 * as bare attribute names with no value — this is the
 * spec-canonical form and what every modern HTML formatter
 * produces.
 *
 * @module generate
 */

import { escapeHtmlAttr } from "./escape.js";
import type { ScriptTag } from "./types.js";
import {
  validateCrossOrigin,
  validateIntegrity,
  validateReferrerPolicy,
  validateScriptSrc,
  validateScriptType,
} from "./validate.js";

interface ResolvedScript {
  readonly type: string | undefined;
  readonly src: string;
  readonly integrity: string | undefined;
  readonly crossorigin: string | undefined;
  readonly referrerpolicy: string | undefined;
  readonly async: boolean;
  readonly defer: boolean;
  readonly nomodule: boolean;
}

/**
 * Generate one or more `<script src="...">` tags from one or
 * many `ScriptTag` descriptors.
 *
 * Accepts a single object or an array.  Returns the
 * concatenated HTML string (one tag per line; no trailing
 * newline).  Empty array → empty string.
 *
 * Throws `TypeError` synchronously on validation failure BEFORE
 * any output is built.  An exception means the caller has
 * nothing to write — there's no risk of partial `<script>`
 * tags reaching the page.
 *
 * ```ts
 * generateScriptTags({
 *   src: "/main.js",
 *   type: "module",
 *   integrity: "sha384-oqVuAfXRKap7fdgcCY5uykM6+R9GqQ8K/uxy9rx7HNQlGYl1kPzQho1wx4JwY8wC",
 *   crossorigin: "anonymous",
 * });
 * // <script type="module" src="/main.js" integrity="sha384-..." crossorigin="anonymous"></script>
 *
 * generateScriptTags([
 *   { src: "/app.js", async: true },
 *   { src: "/analytics.js", defer: true, referrerpolicy: "no-referrer" },
 * ]);
 * ```
 *
 * **`async` + `defer` rule.**  The HTML spec says when both are
 * present on a classic script, `defer` is ignored.  We treat
 * setting both as a caller bug and throw — emitting both bytes
 * masks the bug downstream.
 */
export function generateScriptTags(
  input: ScriptTag | readonly ScriptTag[],
): string {
  const tags = Array.isArray(input) ? input : [input];

  const resolved: ResolvedScript[] = new Array(tags.length);
  for (let i = 0; i < tags.length; i++) {
    resolved[i] = validateOne(tags[i]!, i);
  }

  const lines: string[] = new Array(resolved.length);
  for (let i = 0; i < resolved.length; i++) {
    lines[i] = renderTag(resolved[i]!);
  }
  return lines.join("\n");
}

function validateOne(tag: ScriptTag, i: number): ResolvedScript {
  if (tag === null || typeof tag !== "object") {
    throw new TypeError(
      `forme-aot-script-tag-emitter: input[${i}] must be a non-null object; got ${typeof tag}`,
    );
  }
  const src = validateScriptSrc(tag.src);
  const type = tag.type === undefined ? undefined : validateScriptType(tag.type);
  const integrity = tag.integrity === undefined ? undefined : validateIntegrity(tag.integrity);
  const crossorigin = tag.crossorigin === undefined ? undefined : validateCrossOrigin(tag.crossorigin);
  const referrerpolicy = tag.referrerpolicy === undefined
    ? undefined
    : validateReferrerPolicy(tag.referrerpolicy);
  const async_ = validateBool(tag.async, `input[${i}].async`);
  const defer = validateBool(tag.defer, `input[${i}].defer`);
  const nomodule = validateBool(tag.nomodule, `input[${i}].nomodule`);

  if (async_ && defer) {
    throw new TypeError(
      `forme-aot-script-tag-emitter: input[${i}] cannot set both async and defer (spec: defer is ignored when async is present; reject as caller bug)`,
    );
  }

  return { type, src, integrity, crossorigin, referrerpolicy, async: async_, defer, nomodule };
}

function validateBool(value: unknown, field: string): boolean {
  if (value === undefined || value === false) return false;
  if (value === true) return true;
  throw new TypeError(
    `forme-aot-script-tag-emitter: ${field} must be a boolean; got ${typeof value}`,
  );
}

function renderTag(s: ResolvedScript): string {
  const parts: string[] = ["<script"];
  if (s.type !== undefined)           parts.push(`type="${escapeHtmlAttr(s.type)}"`);
  parts.push(`src="${escapeHtmlAttr(s.src)}"`);
  if (s.integrity !== undefined)      parts.push(`integrity="${escapeHtmlAttr(s.integrity)}"`);
  if (s.crossorigin !== undefined)    parts.push(`crossorigin="${escapeHtmlAttr(s.crossorigin)}"`);
  if (s.referrerpolicy !== undefined) parts.push(`referrerpolicy="${escapeHtmlAttr(s.referrerpolicy)}"`);
  if (s.async)    parts.push("async");
  if (s.defer)    parts.push("defer");
  if (s.nomodule) parts.push("nomodule");
  return `${parts.join(" ")}></script>`;
}
