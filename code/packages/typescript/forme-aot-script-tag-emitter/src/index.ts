/**
 * @coding-adventures/forme-aot-script-tag-emitter
 *
 * Emit HTML `<script src="...">` tags with optional Subresource
 * Integrity (SRI), crossorigin, async/defer/nomodule, type
 * (module / importmap), and referrerpolicy.  Pure transform —
 * returns the tag string(s); caller drops it into the page.
 *
 * ```ts
 * import { generateScriptTags } from "@coding-adventures/forme-aot-script-tag-emitter";
 *
 * // Single module script with SRI:
 * generateScriptTags({
 *   src: "https://cdn.example.com/app.js",
 *   type: "module",
 *   integrity: "sha384-oqVuAfXRKap7fdgcCY5uykM6+R9GqQ8K/uxy9rx7HNQlGYl1kPzQho1wx4JwY8wC",
 *   crossorigin: "anonymous",
 * });
 *
 * // Multiple scripts:
 * generateScriptTags([
 *   { src: "/main.js",      type: "module", async: true },
 *   { src: "/legacy.js",    nomodule: true, defer: true },
 *   { src: "/analytics.js", defer: true, referrerpolicy: "no-referrer" },
 * ]);
 * ```
 *
 * Validation runs BEFORE emission — bad URLs, bad SRI strings,
 * bad allowlist values, async+defer conflicts all throw
 * `TypeError` synchronously.
 *
 * Sixteenth FM00 v0 stage package.
 *
 * @module index
 */

export { generateScriptTags } from "./generate.js";
export { escapeHtmlAttr, stripAsciiControl } from "./escape.js";
export {
  validateScriptSrc,
  validateIntegrity,
  validateScriptType,
  validateCrossOrigin,
  validateReferrerPolicy,
} from "./validate.js";
export type {
  ScriptTag,
  ScriptType,
  CrossOrigin,
  ReferrerPolicy,
} from "./types.js";
