/**
 * slugify.ts — text → URL-safe slug, GitHub-flavoured.
 *
 * Why "GitHub-flavoured" specifically?  Markdown authors writing
 * Forme-targeted content overwhelmingly read content elsewhere
 * (GitHub READMEs, GitLab wikis, gitiles).  Matching the de-facto
 * convention means a heading `## Step 2: Install dependencies`
 * gets the same slug here as it would in a `README.md` rendered
 * on GitHub: `step-2-install-dependencies`.
 *
 * Algorithm (in order):
 *
 *   1. Lowercase via `String.prototype.toLowerCase()` — locale-
 *      independent (no `toLocaleLowerCase` Turkish-İ surprise).
 *   2. Strip ASCII control bytes (`U+0000-U+001F`, `U+007F`) so
 *      a hostile heading text can't smuggle a NUL or DEL into the
 *      slug — even though the regex below would already strip
 *      them, doing it explicitly is faster and self-documenting.
 *   3. Strip everything except `[a-z0-9 -]` (kept set: ASCII
 *      letters, digits, spaces, hyphens).  Note: Unicode letters
 *      (CJK, Cyrillic, accented Latin) get stripped — matches
 *      GitHub's behaviour and avoids percent-encoding decisions.
 *   4. Replace runs of whitespace + hyphens with a single hyphen.
 *   5. Trim leading + trailing hyphens.
 *
 * Empty input or input that reduces to empty → `"section"`
 * fallback.  Collision resolution (in `collisions.ts`) makes the
 * fallback safe even if multiple empty-textHeadings appear:
 * `section`, `section-2`, `section-3`, ...
 *
 * @module slugify
 */

/** ASCII control bytes — stripped before anything else. */
const ASCII_CONTROL_RE = /[\x00-\x1F\x7F]/g;

/** Everything except a-z, 0-9, space, hyphen. */
const NON_SLUG_RE = /[^a-z0-9 \-]+/g;

/** Runs of whitespace and hyphens collapse to one hyphen. */
const SPACE_HYPHEN_RUN_RE = /[\s\-]+/g;

/** Leading / trailing hyphens. */
const TRIM_HYPHEN_RE = /^-+|-+$/g;

/**
 * Convert a heading's plain text into a GitHub-flavoured slug.
 *
 * ```
 * slugify("Hello, World!")               // "hello-world"
 * slugify("Step 2: Install dependencies") // "step-2-install-dependencies"
 * slugify("")                             // "section"
 * slugify("   ")                          // "section"
 * slugify("<script>alert(1)</script>")    // "scriptalert1script"
 * slugify("日本語")                       // "section"  (non-ASCII stripped)
 * ```
 *
 * Output is guaranteed to:
 *   - be non-empty (fallback `"section"` for collapsed input).
 *   - match `/^[a-z0-9-]+$/`.
 *   - not begin or end with `-`.
 *   - contain no consecutive `-` runs.
 */
export function slugify(text: string): string {
  const cleaned = String(text)
    .toLowerCase()
    .replace(ASCII_CONTROL_RE, "")
    .replace(NON_SLUG_RE, "")
    .replace(SPACE_HYPHEN_RUN_RE, "-")
    .replace(TRIM_HYPHEN_RE, "");
  return cleaned === "" ? "section" : cleaned;
}
