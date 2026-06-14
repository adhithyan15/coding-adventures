/**
 * normalise.ts — tag-string normalisation.
 *
 * Why normalise tags at all?  Authors are inconsistent — one post
 * writes `tags: ["TypeScript"]`, another writes
 * `tags: ["typescript"]`, a third writes
 * `tags: ["type-script"]`.  Without normalisation each becomes a
 * separate bucket and the resulting tag cloud looks broken.
 *
 * The normalisation here is a stripped-down version of the
 * slugifier from `forme-transform-autolink-headings`:
 *
 *   1. Lowercase.
 *   2. Walk character-by-character (no regex — zero ReDoS surface).
 *      - Keep `[a-z0-9]`.
 *      - Collapse `[\t\n\r -]` runs into a single `-`.
 *      - Drop everything else (control bytes, punctuation, non-
 *        ASCII).
 *   3. Trim a trailing `-`.
 *   4. Empty result → empty string (caller decides what to do —
 *      the collector treats it as untagged).
 *
 * Single-pass O(n) — same character-loop pattern used by
 * `slugify` in `forme-transform-autolink-headings` to avoid
 * CodeQL polynomial-regex warnings.
 *
 * Why no `"section"` fallback like the heading slugifier?  The
 * caller's tag list is their input; an empty result here is a
 * signal that the input was meaningless ("@@@" or "...").  We
 * surface that as empty rather than synthesising a bucket.
 *
 * @module normalise
 */

// Character codes (named for readability).
const CC_DIGIT_0 = 48;
const CC_DIGIT_9 = 57;
const CC_LOWER_A = 97;
const CC_LOWER_Z = 122;
const CC_SPACE = 32;
const CC_HYPHEN = 45;
const CC_TAB = 9;
const CC_LF = 10;
const CC_CR = 13;

/**
 * Normalise one tag string to its canonical bucket key.
 *
 * ```
 * normaliseTag("TypeScript")       → "typescript"
 * normaliseTag("Type Script")      → "type-script"
 * normaliseTag("type-script")      → "type-script"
 * normaliseTag("  TypeScript  ")   → "typescript"
 * normaliseTag("<script>")         → "script"
 * normaliseTag("__proto__")        → "proto"        ← underscore stripped
 * normaliseTag("日本語")           → ""             ← non-ASCII dropped
 * normaliseTag("")                 → ""
 * ```
 *
 * Output guarantees:
 *   - matches `/^[a-z0-9-]*$/` (note: `*`, not `+` — empty is
 *     possible).
 *   - never begins or ends with `-`.
 *   - never contains consecutive `-` runs.
 *   - safe to interpolate into an HTML attribute or URL slug
 *     without escaping (no `<`, `>`, `&`, `"`, `'`, control
 *     bytes, etc. survive).
 *
 * Complexity: O(n) in input length.  No regex, no backtracking.
 */
export function normaliseTag(tag: string): string {
  const s = String(tag).toLowerCase();
  const out: string[] = [];
  let lastHyphen = true;  // suppress leading hyphen
  for (let i = 0; i < s.length; i++) {
    const c = s.charCodeAt(i);
    if (
      (c >= CC_LOWER_A && c <= CC_LOWER_Z) ||
      (c >= CC_DIGIT_0 && c <= CC_DIGIT_9)
    ) {
      out.push(s[i]!);
      lastHyphen = false;
    } else if (
      c === CC_SPACE || c === CC_HYPHEN ||
      c === CC_TAB || c === CC_LF || c === CC_CR
    ) {
      if (!lastHyphen) {
        out.push("-");
        lastHyphen = true;
      }
    }
    // Everything else (punctuation, control bytes, non-ASCII)
    // dropped silently.
  }
  if (out.length > 0 && out[out.length - 1] === "-") out.pop();
  return out.join("");
}
