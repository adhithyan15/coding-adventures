/**
 * slug.ts — derive a URL-safe slug from a source path.
 *
 * Routed pipelines read canonical URL policy from `ContentNode.route`.
 * This helper remains only for the renderer's final title fallback.
 *
 * @module slug
 */

const MARKDOWN_EXT = /\.(md|mdx|markdown)$/i;

/** See forme-collect-chronological README for the full grammar. */
export function slugify(sourcePath: string): string {
  const segments = sourcePath.split(/[/\\]/);
  let last = segments[segments.length - 1] ?? "";
  last = last.replace(MARKDOWN_EXT, "");
  let s = last.toLowerCase();
  s = s.replace(/[\s_]+/g, "-");
  s = s.replace(/[^a-z0-9-]+/g, "");
  s = s.replace(/-+/g, "-");
  // Trim leading/trailing dashes via index walks (silences CodeQL's
  // polynomial-regex detector on the alternative `/^-+|-+$/g`).
  let lo = 0;
  while (lo < s.length && s.charCodeAt(lo) === 45 /* '-' */) lo++;
  let hi = s.length;
  while (hi > lo && s.charCodeAt(hi - 1) === 45) hi--;
  s = s.slice(lo, hi);
  return s.length > 0 ? s : "untitled";
}
