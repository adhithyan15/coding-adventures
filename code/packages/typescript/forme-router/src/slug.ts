/**
 * slug.ts — derive a URL-safe slug from a source path, and
 * substitute a route template.
 *
 * Behaviour is BIT-IDENTICAL to:
 *   - `@coding-adventures/forme-collect-chronological/src/slug.ts`
 *   - `@coding-adventures/forme-render-static/src/slug.ts`
 *
 * Both will eventually be removed in favour of reading
 * `ContentNode.route` (populated by this stage) — see the package
 * CHANGELOG for the wiring story.
 *
 * @module slug
 */

const MARKDOWN_EXT = /\.(md|mdx|markdown)$/i;

/**
 * Convert a source-file path to a URL-safe slug.
 *
 *   posts/Hello World.md   → "hello-world"
 *   archive/2026/intro.md  → "intro"
 *   notes/_draft_.md       → "draft"
 *   what-even.txt          → "what-even"
 *   ___.md                 → "untitled"
 *
 * The full grammar in plain English:
 *
 *   1. Take the basename.  Both POSIX and Windows separators count.
 *   2. Strip any `.md` / `.mdx` / `.markdown` extension (case-insensitive).
 *   3. Lowercase.
 *   4. Replace runs of whitespace or `_` with `-`.
 *   5. Drop every character outside `[a-z0-9-]`.
 *   6. Collapse runs of `-` to a single `-`.
 *   7. Trim leading and trailing `-`.
 *   8. If the result is empty, fall back to `"untitled"`.
 *
 * Step 5 uses a positive character class (drop non-allowed) rather
 * than a negative one (keep allowed), which is equivalent but easier
 * to read.
 *
 * Step 7 uses index walks rather than the obvious `/^-+|-+$/g`
 * regex — the latter is a polynomial-time pattern that CodeQL's
 * security scanner flags as a potential ReDoS amplification target.
 * Index walks are unambiguously linear.
 */
export function slugify(sourcePath: string): string {
  const segments = sourcePath.split(/[/\\]/);
  let last = segments[segments.length - 1] ?? "";
  last = last.replace(MARKDOWN_EXT, "");
  let s = last.toLowerCase();
  s = s.replace(/[\s_]+/g, "-");
  s = s.replace(/[^a-z0-9-]+/g, "");
  s = s.replace(/-+/g, "-");
  // Trim leading/trailing `-` via index walks (see §7 above).
  let lo = 0;
  while (lo < s.length && s.charCodeAt(lo) === 45 /* '-' */) lo++;
  let hi = s.length;
  while (hi > lo && s.charCodeAt(hi - 1) === 45) hi--;
  s = s.slice(lo, hi);
  return s.length > 0 ? s : "untitled";
}

/**
 * Substitute `{slug}` in a template string.
 *
 * v0 supports only the single `{slug}` placeholder.  Date-based
 * placeholders (`{year}`, `{month}`, `{day}`) and collection-aware
 * placeholders (`{section}`, `{tag}`) are explicitly deferred to a
 * future revision that has structured access to the relevant
 * metadata; this stage doesn't.
 *
 * Unrecognised placeholders pass through unchanged — they're not
 * an error (someone may be templating future syntax) but they're
 * also not silently dropped.  A future stricter mode could reject
 * them.
 *
 * **Security note.**  We pass a *function* replacement
 * (`() => slug`), not a string replacement (`slug`).  When the
 * second argument to `String.prototype.replace` is a string, JS
 * honours `$&`, `$$`, `` $` ``, `$'`, `$1`-`$9`, and `$<name>`
 * sequences inside it.  A user-supplied slug containing those —
 * e.g. `slug: "$&"` in frontmatter — would inject regex
 * back-reference syntax into the output route.  The function
 * form is immune; `slug` is used verbatim regardless of `$`
 * content.  The same fix should land in the legacy duplicates
 * in `forme-collect-chronological/src/slug.ts` and
 * `forme-render-static/src/slug.ts` — done in the same commit
 * that introduced this file.
 */
export function formatRoute(template: string, slug: string): string {
  return template.replace(/\{slug\}/g, () => slug);
}
