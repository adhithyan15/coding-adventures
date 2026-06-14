/**
 * filter.ts — apply minLevel / maxLevel range filter to a flat heading list.
 *
 * Done as a separate step (before tree construction) so the
 * hierarchy reflects only the surviving headings.  Two
 * consequences worth knowing:
 *
 *   - **Filtered-out headings do not preserve their nesting.**
 *     If `<h1>` is filtered out by `minLevel: 2`, the surviving
 *     `<h2>`s become roots — they don't get an artificial
 *     `<h1>` parent.
 *   - **No level remapping.**  An `<h2>` stays an `<h2>` in the
 *     output even if `minLevel: 2`.  Renderers that want
 *     "shift everything to start at h1" should do that
 *     themselves; this package preserves source levels for
 *     callers that need them (e.g. for level-specific styling).
 *
 * @module filter
 */

import type { HeadingSlug } from "@coding-adventures/forme-transform-autolink-headings";

/**
 * Drop slugs whose level is outside `[minLevel, maxLevel]`.
 * Returns a fresh array; input is never mutated.
 *
 * Defaults: `minLevel: 1`, `maxLevel: 6` (no filtering).  Out-of-
 * range options are clamped to the 1-6 valid range so callers
 * who pass `minLevel: 0` or `maxLevel: 10` get sensible
 * behaviour instead of throwing.
 */
export function filterByLevel(
  slugs: readonly HeadingSlug[],
  minLevel: number,
  maxLevel: number,
): HeadingSlug[] {
  const lo = clampLevel(minLevel);
  const hi = clampLevel(maxLevel);
  // If the range is inverted (minLevel > maxLevel) we return an
  // empty array rather than the unintuitive "no headings match
  // the impossible range" — which is the same outcome but
  // explicit short-circuit avoids an O(N) walk.
  if (lo > hi) return [];
  const out: HeadingSlug[] = [];
  for (let i = 0; i < slugs.length; i++) {
    const s = slugs[i]!;
    if (s.level >= lo && s.level <= hi) out.push(s);
  }
  return out;
}

/**
 * Snap any number into the `[1, 6]` heading-level range.
 *
 * Order of guards matters:
 *   - `NaN` first — NaN comparisons always return false, so the
 *     `<` / `>` checks would fall through and `Math.floor(NaN)`
 *     would return `NaN`.  Treat as "1" (most permissive lower
 *     bound; safe default).
 *   - `+Infinity` and `-Infinity` naturally fall through to the
 *     `< 1` / `> 6` checks, so they clamp to 1 and 6
 *     respectively (no special-case needed).
 *   - Fractional values get `Math.floor`-ed so `2.7` → `2`.
 */
function clampLevel(n: number): number {
  if (Number.isNaN(n)) return 1;
  if (n < 1) return 1;
  if (n > 6) return 6;
  return Math.floor(n);
}
