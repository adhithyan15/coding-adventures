/**
 * generate.ts — main `generateManifest` entry.
 *
 * Two-pass: validate every URL/colour/display field first
 * into a fresh validated record, then serialise via
 * `JSON.stringify` with sorted keys for deterministic output.
 *
 * Sorted keys?  `JSON.stringify` doesn't sort by default; we
 * pass an explicit key array to control ordering.  Two
 * benefits:
 *
 *   1. **Byte-determinism.**  Same input → identical output
 *      regardless of object property insertion order
 *      (V8 mostly preserves it, but defensive sorting locks
 *      it in).
 *   2. **Diff-friendly.**  Sites that check the manifest into
 *      git see clean diffs on field changes — no noise from
 *      property reordering.
 *
 * @module generate
 */

import type { ManifestIcon, WebAppManifest } from "./types.js";
import { validateColor, validateDisplay, validateManifestUrl } from "./validate.js";

/**
 * Generate the manifest.json string from a `WebAppManifest`
 * config.
 *
 * Throws `TypeError` synchronously on any validation failure
 * BEFORE any output is built.
 *
 * Output is pretty-printed with 2-space indentation per
 * common web convention (one tap to read in a browser); call
 * `JSON.parse(out)` if you want the object back.
 *
 * Reproducibility: same input → byte-identical output.
 * Input config is never mutated.
 *
 * ```ts
 * generateManifest({
 *   name: "My App",
 *   short_name: "App",
 *   start_url: "/",
 *   display: "standalone",
 *   theme_color: "#0066cc",
 *   background_color: "#ffffff",
 *   icons: [
 *     { src: "/icon-192.png", sizes: "192x192", type: "image/png" },
 *     { src: "/icon-512.png", sizes: "512x512", type: "image/png" },
 *   ],
 * });
 * ```
 */
export function generateManifest(config: WebAppManifest): string {
  const validated: Record<string, unknown> = {};

  // Plain string fields — passed through but only if defined.
  if (config.name !== undefined) validated["name"] = assertString(config.name, "name");
  if (config.short_name !== undefined) validated["short_name"] = assertString(config.short_name, "short_name");
  if (config.description !== undefined) validated["description"] = assertString(config.description, "description");
  if (config.lang !== undefined) validated["lang"] = assertString(config.lang, "lang");
  if (config.dir !== undefined) validated["dir"] = assertString(config.dir, "dir");
  if (config.orientation !== undefined) validated["orientation"] = assertString(config.orientation, "orientation");

  // URL-valued fields.
  if (config.start_url !== undefined) {
    validated["start_url"] = validateManifestUrl(config.start_url, "start_url");
  }
  if (config.scope !== undefined) {
    validated["scope"] = validateManifestUrl(config.scope, "scope");
  }

  // Display allowlist.
  if (config.display !== undefined) {
    validated["display"] = validateDisplay(config.display);
  }

  // Hex colour fields.
  if (config.theme_color !== undefined) {
    validated["theme_color"] = validateColor(config.theme_color, "theme_color");
  }
  if (config.background_color !== undefined) {
    validated["background_color"] = validateColor(config.background_color, "background_color");
  }

  // Icons array.
  if (config.icons !== undefined) {
    if (!Array.isArray(config.icons)) {
      throw new TypeError(
        `forme-aot-manifest-emitter: icons must be an array; got ${typeof config.icons}`,
      );
    }
    validated["icons"] = config.icons.map((icon, i) => validateIcon(icon, i));
  }

  // Sort top-level keys for deterministic output.
  const sortedKeys = Object.keys(validated).sort();
  const sortedObj: Record<string, unknown> = {};
  for (const key of sortedKeys) {
    sortedObj[key] = validated[key];
  }

  return JSON.stringify(sortedObj, null, 2);
}

function assertString(value: unknown, field: string): string {
  if (typeof value !== "string") {
    throw new TypeError(
      `forme-aot-manifest-emitter: ${field} must be a string; got ${typeof value}`,
    );
  }
  return value;
}

/**
 * Validate a single icon entry.  Returns a fresh object with
 * keys in a deterministic order (src first; then sizes, type,
 * purpose alphabetically).
 */
function validateIcon(icon: ManifestIcon, index: number): Record<string, string> {
  if (icon === null || typeof icon !== "object") {
    throw new TypeError(
      `forme-aot-manifest-emitter: icons[${index}] must be a non-null object; got ${typeof icon}`,
    );
  }
  const out: Record<string, string> = {};
  out["src"] = validateManifestUrl(icon.src, `icons[${index}].src`);
  // Other fields are plain strings — passed through with type guard.
  if (icon.purpose !== undefined) {
    out["purpose"] = assertString(icon.purpose, `icons[${index}].purpose`);
  }
  if (icon.sizes !== undefined) {
    out["sizes"] = assertString(icon.sizes, `icons[${index}].sizes`);
  }
  if (icon.type !== undefined) {
    out["type"] = assertString(icon.type, `icons[${index}].type`);
  }
  // Re-sort with src first, then alphabetical for the rest.
  // src is the only required field; emitting it first matches
  // the W3C example output style.
  const sorted: Record<string, string> = { src: out["src"]! };
  for (const k of Object.keys(out).filter((x) => x !== "src").sort()) {
    sorted[k] = out[k]!;
  }
  return sorted;
}
