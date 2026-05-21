/**
 * slug.ts — GitHub-style heading slugifier.
 *
 * =============================================================================
 * WHAT IS A SLUG?
 * =============================================================================
 *
 * A "slug" is the URL-safe identifier you see after the `#` in a deep link
 * to a heading on a documentation page:
 *
 *     https://example.com/intro.html#getting-started
 *                                    ^^^^^^^^^^^^^^^
 *                                    this is the slug
 *
 * It's derived deterministically from the heading text.  The point of
 * "deterministic" is that the same `## Getting Started` should always
 * produce the same `#getting-started` no matter how many times you
 * regenerate the site — otherwise every rebuild would break every
 * inbound link from external bookmarks and search engines.
 *
 * =============================================================================
 * GITHUB'S ALGORITHM (THE DE-FACTO STANDARD)
 * =============================================================================
 *
 * GitHub renders Markdown all over the place (READMEs, issues, gists,
 * Pages) and their slugifier is the closest thing to a standard for
 * `.md`-driven sites.  It's implemented in Ruby in `gollum-lib` and the
 * essential transformation is:
 *
 *     text.downcase
 *         .gsub(/[^\w\- ]/u, '')   # drop anything that isn't a
 *                                  # word-char, hyphen, or space
 *         .gsub(/ /, '-')          # spaces become hyphens
 *
 * `\w` is Ruby's "word character" class with the `/u` Unicode flag —
 * letters (any script), digits, and underscores.  Hyphens are explicit.
 *
 * Worked examples (matches GitHub's behaviour):
 *
 *     "Getting Started"          → "getting-started"
 *     "API Reference"            → "api-reference"
 *     "Hello, World!"            → "hello-world"
 *     "  Trim me  "              → "--trim-me--"          (yes, edge spaces)
 *     "Snake_case_works"         → "snake_case_works"     (underscores kept)
 *     "Already-hyphenated"       → "already-hyphenated"
 *     "中文标题"                 → "中文标题"             (Unicode letters kept)
 *     "v2.0 — Release notes"     → "v20--release-notes"   (dots dropped, dash kept)
 *     "100% done"                → "100-done"
 *     ""                         → ""                     (empty → empty)
 *     "!@#$%^&*()"               → ""                     (all stripped → empty)
 *
 * The empty-slug case is real and is what triggers our collision
 * suffixing in the walker — two empty-titled headings (e.g. `# ` `#  `)
 * collide on `""`, and the second one gets bumped to `-2`.
 *
 * =============================================================================
 * DETERMINISM NOTES
 * =============================================================================
 *
 *   - `toLowerCase()` is locale-independent in V8 / SpiderMonkey (it
 *     uses the Unicode default case-folding).  We don't call
 *     `toLocaleLowerCase()` because that introduces locale-dependent
 *     differences (Turkish 'I' folds to 'ı' under tr-TR).  Heading
 *     anchors must be stable across machines, so locale-default is wrong.
 *   - `\p{L}\p{N}` covers every Unicode letter and number.  Combined
 *     with `_` and `-`, those are the four character classes we keep.
 *   - Spaces map 1:1 to hyphens (no run-collapsing).  This matches
 *     GitHub's behaviour and means `# A   B` (3 spaces) becomes
 *     `a---b` (3 hyphens).  Counter-intuitive but stable.
 *
 * @module slug
 */

/**
 * Generate a GitHub-style slug from heading text.  Pure function.
 *
 * @param text - The plain-text content of the heading (no markup).
 *               Call `extractPlainText()` from the walker first.
 * @returns The slug — lowercased, with non-word/hyphen/space characters
 *          stripped, and spaces replaced by hyphens.  May be empty if
 *          the input has no slug-eligible characters.
 */
export function slugify(text: string): string {
  // Step 1: lowercase using Unicode default case-folding (locale-independent).
  const lower = text.toLowerCase();
  // Step 2: strip anything that isn't a Unicode letter, Unicode digit,
  //         underscore, hyphen, or ASCII space.  The `\p{L}` and `\p{N}`
  //         escapes require the `u` flag.
  const stripped = lower.replace(/[^\p{L}\p{N}_\- ]+/gu, "");
  // Step 3: map spaces to hyphens.  1:1, no run-collapsing — matches GitHub.
  return stripped.replace(/ /g, "-");
}
