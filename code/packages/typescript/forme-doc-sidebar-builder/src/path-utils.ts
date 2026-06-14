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
 * Strip leading and trailing slashes using explicit index walks.
 *
 * We do this with index loops rather than `.replace(/^\/+/, "")`
 * /  `.replace(/\/+$/, "")` because CodeQL's `js/polynomial-redos`
 * query flags those anchored-`+` patterns as potentially
 * super-linear on adversarial input (even though both ARE
 * linear in V8's regex engine).  Linear index walks make the
 * O(N) cost obvious to both the static analyser and the
 * reader.
 *
 * @internal
 */
function stripSlashes(s: string): string {
  let start = 0;
  while (start < s.length && s.charCodeAt(start) === 0x2f /* "/" */) start++;
  let end = s.length;
  while (end > start && s.charCodeAt(end - 1) === 0x2f) end--;
  if (start === 0 && end === s.length) return s;
  return s.slice(start, end);
}

/**
 * Strip leading slashes, trailing slashes, and a recognised
 * extension from a raw path, then split on `/`.
 *
 * @internal
 */
function splitAndClean(rawPath: string): string[] {
  // Strip leading / trailing slashes (linear, regex-free).
  let p = stripSlashes(rawPath);
  // Strip recognised extension.  Anchored `$`, finite literal
  // alternation, single non-quantified `\.` — not polynomial.
  // We use endsWith() instead of a regex anyway to keep the
  // intent obvious and the analyser happy.
  const lower = p.toLowerCase();
  for (const ext of [".md", ".mdx", ".html", ".htm"]) {
    if (lower.endsWith(ext)) {
      p = p.slice(0, p.length - ext.length);
      break;
    }
  }
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
 * Maximum raw-path length (in characters).  Caps the input size
 * BEFORE any per-character processing runs, defending against
 * adversarial inputs that might otherwise consume disproportionate
 * CPU even with linear-time helpers.  Real filesystem paths max
 * out around 4 KiB on most platforms; we leave a generous margin.
 */
const MAX_PATH_LENGTH = 8192;

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
  // Cheap upfront cap — fail fast before any per-character work.
  // This bounds the CPU cost of normalisation against adversarial
  // inputs (e.g. a 100MB path of slashes) regardless of how
  // efficient the inner helpers are.
  if (rawPath.length > MAX_PATH_LENGTH) {
    throw new TypeError(
      `forme-doc-sidebar-builder: raw path exceeds ${MAX_PATH_LENGTH}-character ` +
        `length cap (got ${rawPath.length})`,
    );
  }
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
