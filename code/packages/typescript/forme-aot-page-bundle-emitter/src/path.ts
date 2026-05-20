/**
 * path.ts — deterministic route → output-path derivation.
 *
 * Rule set:
 *
 *   `/`                  →  `"index.html"`
 *   `/about`             →  `"about/index.html"`
 *   `/posts/x`           →  `"posts/x/index.html"`
 *   `/page.html`         →  `"page.html"`            (extension preserved
 *                                                    when the last segment
 *                                                    has one)
 *   `/feed.xml`          →  `"feed.xml"`
 *   `/posts/x.html`      →  `"posts/x.html"`
 *   `/x/`                →  invalid (trailing slash) — `validateRoute`
 *                          would have already rejected the empty
 *                          trailing segment.
 *
 * "Has an extension" = the last segment contains a `.` after the
 * first character.  We deliberately don't try to enumerate
 * extensions — any `.ext` is preserved.
 *
 * @module path
 */

/**
 * Convert a validated route to its deterministic output path.
 * Caller MUST validate the route first via `validateRoute`.
 */
export function routeToOutputPath(route: string): string {
  if (route === "/") return "index.html";
  // route starts with "/", validated.  Strip leading slash.
  const trimmed = route.slice(1);
  const lastSlash = trimmed.lastIndexOf("/");
  const lastSeg = lastSlash === -1 ? trimmed : trimmed.slice(lastSlash + 1);
  // "Has extension" — `.` at index > 0 in the last segment.
  const dotIdx = lastSeg.indexOf(".");
  if (dotIdx > 0) {
    return trimmed;
  }
  return `${trimmed}/index.html`;
}
