/**
 * normalise.ts — author-string normalisation.
 *
 * Identical algorithm to `forme-collect-by-tag`'s `normaliseTag`
 * — lowercase + slug-strip via a single-pass `charCodeAt` loop.
 * Why the duplicate copy here instead of a shared sub-package?
 * The collector packages are intentionally standalone so a
 * caller can pull `forme-collect-by-author` without dragging
 * `forme-collect-by-tag` into their dependency graph.  The
 * 50-line normaliser is small enough that the duplication is
 * cheaper than a third "forme-slug" package.
 *
 * If both packages ever need to coordinate on tweaks to the
 * normalisation rules, factor out a shared `forme-slug` package
 * at that point.
 *
 * Substitutions:
 *
 *   1. Lowercase.
 *   2. Strip ASCII control bytes (`\x00-\x1F`, `\x7F`).
 *   3. Walk char-by-char:
 *        - Keep `[a-z0-9]`.
 *        - Collapse `[\t\n\r -]` runs to single `-`.
 *        - Drop everything else (punctuation, underscores,
 *          non-ASCII, quotes, angle brackets, ampersand).
 *   4. Trim trailing `-`.
 *   5. Empty → empty.  Caller-empty input signals "anonymous".
 *
 * Output guarantee: `/^[a-z0-9-]*$/`; safe to interpolate into
 * HTML attributes / URL slugs without escaping.
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
 * Normalise one author name to its canonical bucket key.
 *
 * ```
 * normaliseAuthor("Ada Lovelace")     → "ada-lovelace"
 * normaliseAuthor("AdaLovelace")      → "adalovelace"
 * normaliseAuthor("ada lovelace")     → "ada-lovelace"
 * normaliseAuthor("  Ada  ")          → "ada"
 * normaliseAuthor("<script>")         → "script"
 * normaliseAuthor("日本語")           → ""
 * normaliseAuthor("")                 → ""
 * ```
 *
 * Complexity: O(n) in input length.  No regex, no backtracking.
 */
export function normaliseAuthor(name: string): string {
  const s = String(name).toLowerCase();
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
    // Everything else dropped.
  }
  if (out.length > 0 && out[out.length - 1] === "-") out.pop();
  return out.join("");
}
