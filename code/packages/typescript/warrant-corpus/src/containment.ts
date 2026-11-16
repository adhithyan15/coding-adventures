/**
 * # Is this path inside that directory?
 *
 * A containment check is a security boundary: the corpus (and the library under
 * test) must live inside the repository, or a crafted `--corpus` could point the
 * runner at files it was never meant to read.
 *
 * ## The trap: `relative` does not always return a relative path
 *
 * The usual idiom is "compute `relative(root, child)` and see whether it climbs
 * out with `..`". That is not enough on Windows. Drives are separate roots, so
 * there is no way to climb from `D:\repo` to `C:\tmp\x`; `path.relative` gives up
 * and returns the *absolute* target:
 *
 * ```
 * win32.relative("D:\\repo", "C:\\tmp\\x")   // "C:\\tmp\\x"  -- no leading ".."
 * ```
 *
 * A check that only looks for `..` waves that through. So there are three ways
 * to be outside:
 *
 * 1. the result is absolute (a different drive or UNC share);
 * 2. the result is exactly `..`;
 * 3. the result starts with `..` *followed by a separator*.
 *
 * Case 3 must not be a bare `startsWith("..")`: a directory legitimately named
 * `..foo` is inside the root, and `relative` reports it as `..foo`.
 *
 * Taking the path module as a parameter lets tests drive `path.win32` from any
 * host, so the Windows behaviour is checked on Linux CI too.
 */

import * as nodePath from "node:path";

/** The slice of a `node:path` module this check needs (`posix` or `win32`). */
export type PathModule = Pick<typeof nodePath.posix, "isAbsolute" | "relative" | "resolve" | "sep">;

/**
 * `child` relative to `root` when it lies at or beneath `root`, otherwise `null`.
 * The root itself yields the empty string; callers that need a *strict*
 * descendant reject `""` as well.
 */
export function relativeInside(root: string, child: string, mod: PathModule = nodePath): string | null {
  const path = mod.relative(mod.resolve(root), mod.resolve(child));
  if (mod.isAbsolute(path)) return null;
  if (path === ".." || path.startsWith(`..${mod.sep}`)) return null;
  return path;
}
