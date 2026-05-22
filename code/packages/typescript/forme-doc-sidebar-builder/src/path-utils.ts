/**
 * path-utils.ts — path normalisation and index-page detection.
 *
 * Pages arrive with paths in any of several common forms:
 *
 *   "guide/setup.md"          (file path, .md)
 *   "guide/setup.mdx"         (file path, .mdx)
 *   "guide/setup.html"        (already built)
 *   "/guide/setup"            (URL path, leading slash, no ext)
 *   "guide/setup/"            (trailing slash)
 *   "guide/setup/index.md"    (explicit index)
 *
 * We normalise all of them to a single canonical form
 * (`["guide", "setup"]` — directory parts as a list) before
 * building the tree.  Index pages get a flag so the group knows
 * to attach the page's path / metadata to itself rather than as
 * a child entry.
 *
 * @module path-utils
 */

/**
 * Result of normalising a path.
 */
export interface NormalisedPath {
  /**
   * The directory parts up to the filename, after stripping
   * extensions and `index` suffixes.  For an index page, the
   * last part of the original path is consumed into the group
   * and `parts` is the parent directory.  For a non-index page,
   * `parts` ends with the file slug.
   *
   * Examples:
   *   "guide/setup.md"        → ["guide", "setup"]
   *   "guide/index.md"        → ["guide"]                  (index)
   *   "index.md"              → []                          (root index)
   *   "intro.md"              → ["intro"]
   *   "/docs/api/v1.md"       → ["docs", "api", "v1"]
   */
  readonly parts: readonly string[];

  /** True iff the path refers to an index page for its directory. */
  readonly isIndex: boolean;
}

/**
 * Strip leading slashes, trailing slashes, and a recognised
 * extension from a raw path, then split on `/`.
 *
 * @internal
 */
function splitAndClean(rawPath: string): string[] {
  // Strip leading slash.
  let p = rawPath.replace(/^\/+/, "");
  // Strip trailing slash.
  p = p.replace(/\/+$/, "");
  // Strip recognised extension (case-insensitive on the dot+ext).
  p = p.replace(/\.(md|mdx|html|htm)$/i, "");
  if (p === "") return [];
  return p.split("/").filter((s) => s.length > 0);
}

/**
 * Maximum directory nesting depth.  Caps stack usage in
 * `builder.emit` (which recurses one JS frame per level) at well
 * below V8's default stack limit.  Real-world docs sites
 * essentially never exceed 8 levels; 64 leaves a wide safety
 * margin while preventing an adversarial path like
 * `a/b/c/.../z.md` (10k segments) from triggering a `RangeError`
 * on emit.
 */
const MAX_DEPTH = 64;

/**
 * Normalise a raw page path into directory parts + index flag.
 *
 * @param rawPath - The input path string.
 * @returns `{ parts, isIndex }`.
 * @throws `TypeError` if `rawPath` is empty after trimming
 *         leading/trailing whitespace (every page needs a place
 *         in the tree), or if the resulting depth would exceed
 *         `MAX_DEPTH` (64) — prevents stack-overflow DoS from
 *         adversarial deeply-nested inputs.
 */
export function normalisePath(rawPath: string): NormalisedPath {
  const trimmed = rawPath.trim();
  if (trimmed === "") {
    throw new TypeError(
      "forme-doc-sidebar-builder: empty path — every page needs a place in the tree",
    );
  }
  const parts = splitAndClean(trimmed);
  if (parts.length > MAX_DEPTH) {
    throw new TypeError(
      `forme-doc-sidebar-builder: path ${JSON.stringify(rawPath)} exceeds ` +
        `${MAX_DEPTH}-level directory-depth cap`,
    );
  }
  // Detect index: last segment is "index" (case-insensitive).
  if (parts.length > 0 && parts[parts.length - 1]!.toLowerCase() === "index") {
    return { parts: parts.slice(0, -1), isIndex: true };
  }
  return { parts, isIndex: false };
}

/**
 * Apply an optional root prefix: strip it from `parts` if
 * present, return `null` if `parts` doesn't start with it.
 *
 * @param parts - The normalised directory parts.
 * @param root - The root prefix string (`""` = no stripping).
 * @returns The remaining parts, or `null` if the prefix doesn't match.
 */
export function stripRoot(
  parts: readonly string[],
  root: string,
): readonly string[] | null {
  const rootTrim = root.trim();
  if (rootTrim === "") return parts;
  const rootParts = splitAndClean(rootTrim);
  if (rootParts.length > parts.length) return null;
  for (let i = 0; i < rootParts.length; i++) {
    if (parts[i] !== rootParts[i]) return null;
  }
  return parts.slice(rootParts.length);
}
