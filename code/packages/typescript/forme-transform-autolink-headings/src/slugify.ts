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
 * Algorithm (single-pass character loop):
 *
 *   1. Lowercase via `String.prototype.toLowerCase()` — locale-
 *      independent (no `toLocaleLowerCase` Turkish-İ surprise).
 *   2. Walk character-by-character:
 *        - Keep `[a-z0-9]` verbatim.
 *        - Collapse runs of `[\t\n\r -]` (ASCII whitespace + hyphen)
 *          to a single `-`, suppressing leading hyphens with a
 *          `lastHyphen` flag.
 *        - Drop everything else silently (control bytes,
 *          punctuation, non-ASCII).
 *   3. Trim a trailing hyphen.
 *   4. Empty result → `"section"` fallback.
 *
 * Why a manual loop instead of chained `String.prototype.replace`?
 *
 *   - **No regex backtracking surface.**  Patterns like `[\s\-]+`
 *     combined with `/^-+|-+$/g` trigger CodeQL's "polynomial regex
 *     on uncontrolled data" warning even though the actual runtime
 *     is linear.  A character loop is unambiguously O(n) and
 *     trivially passes any ReDoS analysis.
 *   - **One pass, not three.**  The old chained version walked the
 *     string four times (lowercase + 3× replace).  The loop walks
 *     it once.
 *   - **Explicit semantics.**  Each branch is one line; what gets
 *     kept and what gets dropped is obvious from reading.
 *
 * Collision resolution (in `collisions.ts`) makes the `"section"`
 * fallback safe even if multiple empty-text headings appear:
 * `section`, `section-2`, `section-3`, ...
 *
 * @module slugify
 */

// Character codes used in the hot loop — named so the loop reads.
const CC_DIGIT_0 = 48;   // '0'
const CC_DIGIT_9 = 57;   // '9'
const CC_LOWER_A = 97;   // 'a'
const CC_LOWER_Z = 122;  // 'z'
const CC_SPACE = 32;
const CC_HYPHEN = 45;
const CC_TAB = 9;
const CC_LF = 10;
const CC_CR = 13;

/**
 * Convert a heading's plain text into a GitHub-flavoured slug.
 *
 * ```
 * slugify("Hello, World!")                // "hello-world"
 * slugify("Step 2: Install dependencies") // "step-2-install-dependencies"
 * slugify("")                             // "section"
 * slugify("   ")                          // "section"
 * slugify("<script>alert(1)</script>")    // "scriptalert1script"
 * slugify("日本語")                       // "section"  (non-ASCII dropped)
 * slugify("hel\x00lo")                    // "hello"   (control byte dropped)
 * ```
 *
 * Output is guaranteed to:
 *   - be non-empty (fallback `"section"` for collapsed input).
 *   - match `/^[a-z0-9-]+$/`.
 *   - not begin or end with `-`.
 *   - contain no consecutive `-` runs.
 *
 * Complexity: O(n) in input length.  No regex, no backtracking.
 */
export function slugify(text: string): string {
  const s = String(text).toLowerCase();
  const out: string[] = [];
  // Start "true" so leading ASCII whitespace / hyphens never emit
  // a leading "-" in the output.
  let lastHyphen = true;
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
      // Whitespace / hyphen — collapse to one "-" per run.
      if (!lastHyphen) {
        out.push("-");
        lastHyphen = true;
      }
    }
    // All other code points (control bytes, punctuation, symbols,
    // non-ASCII) drop silently — no emission, no state change.
  }
  // Trim a trailing "-" left by terminal whitespace / hyphens.
  if (out.length > 0 && out[out.length - 1] === "-") out.pop();
  return out.length === 0 ? "section" : out.join("");
}
