/**
 * slug.ts — derive a URL-safe slug from a source path.
 *
 * **DRY violation accepted in v0.** This logic is duplicated from
 * `@coding-adventures/forme-collect-chronological/src/slug.ts`.  The
 * "right" answer is a tiny shared utility package (e.g.
 * `@coding-adventures/forme-text-utils`), but standing one up costs
 * more in monorepo wiring (BUILD chains, lockfile drift, peer-dep
 * graphs) than the ~30 lines saves at this size.  When the third
 * stage needs `slugify` we'll extract it.
 *
 * The render stage needs its own slug derivation because — in v0 —
 * `ContentNode.route` is `null` (the collector emits routes on
 * `Collection.entries`, not on the node).  A future v0.2 router stage
 * will fold collection-side routes back onto the node; until then,
 * each renderer derives the route from `sourcePath` independently
 * using the same rules as the collector (so both produce identical
 * routes for the same input).
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

/** Substitute `{slug}` in a template string. */
export function formatRoute(template: string, slug: string): string {
  return template.replace(/\{slug\}/g, slug);
}
