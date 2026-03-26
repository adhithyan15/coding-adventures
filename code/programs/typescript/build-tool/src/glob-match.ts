/**
 * glob-match.ts -- Pure String-Based Glob Matching
 * ==================================================
 *
 * This module implements glob pattern matching for file paths, used by the
 * build tool to filter source files declared in Starlark BUILD files. It
 * supports the three standard glob wildcards:
 *
 *   - `*`  matches zero or more characters within a single path segment
 *           (does NOT cross `/` boundaries)
 *   - `?`  matches exactly one character (not `/`)
 *   - a double-star pattern matches zero or more path segments
 *
 * ==========================================================================
 * Chapter 1: Why Not Use a Library?
 * ==========================================================================
 *
 * Node.js has no built-in `fnmatch` or glob-matching function (only
 * `fs.glob` for filesystem enumeration). We could use a library like
 * `minimatch` or `picomatch`, but this build tool aims for zero external
 * dependencies. A pure-string glob matcher is surprisingly simple -- around
 * 100 lines of code -- and gives us full control over the matching semantics.
 *
 * ==========================================================================
 * Chapter 2: The Algorithm
 * ==========================================================================
 *
 * The matching works by splitting both the pattern and the path on `/` into
 * segments, then comparing segments left-to-right with special handling for
 * the double-star wildcard:
 *
 * 1. Split pattern and path on `/`.
 *    Example: "src/foo.py" -> ["src", "foo.py"]
 *
 * 2. Walk through pattern segments:
 *    - If the segment is a double-star wildcard, try matching the remaining
 *      pattern against every possible suffix of the remaining path segments
 *      (zero or more segments consumed). This is a recursive search.
 *    - Otherwise, match the segment against the corresponding path segment
 *      using single-segment matching (handling `*` and `?`).
 *
 * 3. Single-segment matching uses a two-pointer approach:
 *    - Walk through pattern and text simultaneously.
 *    - `?` matches any single character.
 *    - `*` matches zero or more characters. We record a "star position"
 *      and try advancing the text pointer one character at a time.
 *    - Literal characters must match exactly.
 *
 * ==========================================================================
 * Chapter 3: Truth Table
 * ==========================================================================
 *
 * | Pattern             | Path                    | Match? | Why                          |
 * |---------------------|-------------------------|--------|------------------------------|
 * | "src/foo.py"        | "src/foo.py"            | true   | Exact match                  |
 * | "src/*.py"          | "src/foo.py"            | true   | * matches "foo"              |
 * | "src/*.py"          | "src/bar/foo.py"        | false  | * does not cross /           |
 * | "src/f?o.py"        | "src/foo.py"            | true   | ? matches "o"               |
 * | "src/f?o.py"        | "src/fo.py"             | false  | ? must match exactly one     |
 * | "src/any/foo.py"    | "src/foo.py"            | false  | double-star matches 0+ segs  |
 * | "src/any/foo.py"    | "src/a/b/foo.py"        | false  | literal "any" != "a"         |
 *
 * @module
 */

// ---------------------------------------------------------------------------
// Single-Segment Matching
// ---------------------------------------------------------------------------

/**
 * Match a single path segment against a glob pattern segment.
 *
 * Supports `*` (zero or more characters) and `?` (exactly one character).
 * No `/` characters should appear in either argument -- they are segments.
 *
 * The algorithm uses a "star backtrack" technique:
 *
 * - We maintain two pointers: `pi` into the pattern and `ti` into the text.
 * - When we see `*`, we record where we are (starIdx, matchIdx) and try
 *   matching zero characters first.
 * - If a mismatch occurs later, we backtrack to the star position and try
 *   matching one more character from the text.
 * - This is O(n*m) in the worst case but typically linear for real globs.
 *
 * @param pattern - The glob pattern segment (e.g., "*.py", "f?o").
 * @param text    - The text segment to match against (e.g., "foo.py").
 * @returns true if the text matches the pattern.
 */
export function matchSegment(pattern: string, text: string): boolean {
  let pi = 0; // Pattern index
  let ti = 0; // Text index
  let starIdx = -1; // Position of last `*` in pattern
  let matchIdx = -1; // Position in text when we last hit `*`

  while (ti < text.length) {
    if (pi < pattern.length && (pattern[pi] === "?" || pattern[pi] === text[ti])) {
      // Current characters match (or pattern has `?`). Advance both.
      pi++;
      ti++;
    } else if (pi < pattern.length && pattern[pi] === "*") {
      // Star: record position and try matching zero characters.
      starIdx = pi;
      matchIdx = ti;
      pi++;
    } else if (starIdx !== -1) {
      // Mismatch, but we have a star to backtrack to.
      // Try matching one more character from the text against the star.
      pi = starIdx + 1;
      matchIdx++;
      ti = matchIdx;
    } else {
      // Mismatch with no star to backtrack to -- fail.
      return false;
    }
  }

  // Consume any trailing `*` in the pattern (they match empty strings).
  while (pi < pattern.length && pattern[pi] === "*") {
    pi++;
  }

  // Match succeeds only if we consumed the entire pattern.
  return pi === pattern.length;
}

// ---------------------------------------------------------------------------
// Full Path Matching
// ---------------------------------------------------------------------------

/**
 * Match a file path against a glob pattern.
 *
 * Both pattern and path are split on `/` into segments. The matching then
 * proceeds segment-by-segment. A double-star segment in the pattern matches
 * zero or more path segments.
 *
 * Important: both pattern and path should use forward slashes (`/`) as
 * separators. The caller is responsible for normalizing Windows backslashes
 * before calling this function.
 *
 * @param pattern - The glob pattern (e.g., "src/foo.py", "tests/*.test.ts").
 * @param filePath - The file path to test (e.g., "src/lib/foo.py").
 * @returns true if the path matches the pattern.
 *
 * @example
 * ```typescript
 * matchPath("src/*.py", "src/foo.py");        // true
 * matchPath("src/*.py", "src/bar/foo.py");    // false (* doesn't cross /)
 * ```
 */
export function matchPath(pattern: string, filePath: string): boolean {
  // Split into segments, filtering out empty strings from leading/trailing /.
  const patternSegs = pattern.split("/").filter((s) => s.length > 0);
  const pathSegs = filePath.split("/").filter((s) => s.length > 0);

  return matchSegments(patternSegs, 0, pathSegs, 0);
}

/**
 * Recursive segment-level matching engine.
 *
 * This is the heart of the glob matcher. It walks through pattern segments
 * and path segments in parallel, with special handling for double-star:
 *
 * - Double-star: try matching the remaining pattern against every possible
 *   suffix of the remaining path (consuming 0, 1, 2, ... path segments).
 *   This recursive search handles nested directories of any depth.
 *
 * - Normal segment: use matchSegment() for single-segment comparison,
 *   then advance both pointers.
 *
 * @param pSegs - Pattern segments array.
 * @param pi    - Current index into pattern segments.
 * @param tSegs - Path segments array.
 * @param ti    - Current index into path segments.
 * @returns true if the remaining segments match.
 */
function matchSegments(
  pSegs: readonly string[],
  pi: number,
  tSegs: readonly string[],
  ti: number,
): boolean {
  // Base case: both pattern and path exhausted -- match!
  if (pi === pSegs.length && ti === tSegs.length) {
    return true;
  }

  // Pattern exhausted but path segments remain -- no match.
  if (pi === pSegs.length) {
    return false;
  }

  // Handle double-star: matches zero or more path segments.
  //
  // We try consuming 0 segments (skip the double-star entirely), then 1,
  // then 2, etc. up to all remaining path segments. If any attempt
  // succeeds, the whole match succeeds.
  if (pSegs[pi] === "**") {
    // Optimization: collapse consecutive double-stars (they're equivalent
    // to a single one). "a/**/b" and "a/**/**/b" match the same paths.
    let nextPi = pi;
    while (nextPi < pSegs.length && pSegs[nextPi] === "**") {
      nextPi++;
    }

    // Try consuming 0, 1, 2, ... path segments.
    for (let skip = 0; skip <= tSegs.length - ti; skip++) {
      if (matchSegments(pSegs, nextPi, tSegs, ti + skip)) {
        return true;
      }
    }
    return false;
  }

  // Path exhausted but pattern has non-double-star segments remaining -- no match.
  if (ti === tSegs.length) {
    return false;
  }

  // Normal segment: match using single-segment matcher, then advance.
  if (matchSegment(pSegs[pi], tSegs[ti])) {
    return matchSegments(pSegs, pi + 1, tSegs, ti + 1);
  }

  return false;
}
