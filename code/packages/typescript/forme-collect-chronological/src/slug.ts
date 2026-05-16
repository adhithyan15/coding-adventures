/**
 * slug.ts — derive a URL-safe slug from a source path.
 *
 * Used as the fallback when frontmatter doesn't supply an explicit
 * `slug:` field.  Rules (kept intentionally tiny — Jekyll-style):
 *
 *   1. Take the basename (last path segment).
 *   2. Strip the trailing markdown extension if present
 *      (`.md`, `.mdx`, `.markdown`; case-insensitive).
 *   3. Lowercase.
 *   4. Replace runs of whitespace or `_` with a single `-`.
 *   5. Drop any character that isn't `[a-z0-9-]`.
 *   6. Collapse multiple `-` into one; trim leading/trailing `-`.
 *
 * The result is safe to drop straight into a URL path segment and
 * stable across operating systems (we split on both `/` and `\`).
 *
 * @module slug
 */

const MARKDOWN_EXT = /\.(md|mdx|markdown)$/i;

/**
 * Compute a slug for a `ContentNode`'s `sourcePath`.
 *
 * @example
 * slugify("posts/2026-05-15-hello world.md") === "2026-05-15-hello-world";
 * slugify("Drafts\\My Post.MDX")              === "my-post";
 * slugify("a/b/c.markdown")                   === "c";
 * slugify("a/b/c")                             === "c";   // no ext → keep
 */
export function slugify(sourcePath: string): string {
  // Split on both POSIX and Windows separators so the same input
  // produces the same slug regardless of where it was scanned.
  const segments = sourcePath.split(/[/\\]/);
  let last = segments[segments.length - 1] ?? "";

  // Strip the trailing markdown extension (but only markdown — leaving
  // unknown extensions intact protects against accidentally treating
  // "post.notes" as "post").
  last = last.replace(MARKDOWN_EXT, "");

  // Lowercase, normalise separators, drop disallowed chars.
  let s = last.toLowerCase();
  s = s.replace(/[\s_]+/g, "-");
  s = s.replace(/[^a-z0-9-]+/g, "");
  s = s.replace(/-+/g, "-");
  // Trim leading/trailing dashes via index walks rather than a regex.
  // Anchored regexes like `/-+$/` still trip CodeQL's polynomial-regex
  // detector on library input even though the worst case is O(n).
  // Explicit single-pass index walks are unambiguously linear and
  // pattern-detector-proof.
  let lo = 0;
  while (lo < s.length && s.charCodeAt(lo) === 45 /* '-' */) lo++;
  let hi = s.length;
  while (hi > lo && s.charCodeAt(hi - 1) === 45) hi--;
  s = s.slice(lo, hi);

  // Empty result (e.g. input was "@@@.md") falls back to "untitled" so
  // route templating never produces "/blog/.html".
  return s.length > 0 ? s : "untitled";
}

/**
 * Format a route template by substituting `{slug}` (and only `{slug}`
 * in v0).  Future templates may add `{year}`, `{month}`, `{day}` etc.
 *
 * @example
 * formatRoute("/blog/{slug}.html", "hello") === "/blog/hello.html";
 */
export function formatRoute(template: string, slug: string): string {
  return template.replace(/\{slug\}/g, slug);
}
