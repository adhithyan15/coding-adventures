/**
 * path-utils.ts — route → on-disk path mapping.
 *
 * A `RenderedPage.route` is a URL path like `/blog/hello.html`.
 * The emit stage needs the corresponding filesystem path *under* a
 * configured `outDir`.  Two non-obvious rules:
 *
 *   1. **Strip the leading slash.**  `join("dist", "/blog/x")` gives
 *      `"/blog/x"` on POSIX because the absolute path on the right
 *      wins — exactly the bug a static-site generator should never
 *      ship.
 *
 *   2. **Reject `..` segments.**  A malicious route like
 *      `../../etc/passwd` would otherwise escape outDir.  We resolve
 *      the candidate path and assert it stays inside the resolved
 *      outDir; any failure throws synchronously *before* any file
 *      open / write.
 *
 * Both rules are pure path math — no filesystem reads required.
 *
 * @module path-utils
 */

import { resolve, sep } from "node:path";

/**
 * Map a route + outDir to an absolute on-disk path, with safety
 * checks.  Throws on traversal attempts; never accesses the
 * filesystem itself.
 */
export function routeToOutPath(outDir: string, route: string): string {
  // Defensive: a route MUST be non-empty and MUST start with "/".  The
  // renderer always produces these (template `/blog/{slug}.html`), but
  // a future or third-party renderer might not.
  if (typeof route !== "string" || route.length === 0) {
    throw new Error(`forme-emit-fs: empty route is not a valid output path`);
  }
  // Strip exactly one leading slash so the join doesn't see an
  // "absolute" right-hand-side.  We don't `replace(/^\/+/, "")` —
  // multiple leading slashes are suspicious enough to flag explicitly
  // rather than silently coalesce.
  const relative = route.startsWith("/") ? route.slice(1) : route;
  if (relative.length === 0) {
    throw new Error(`forme-emit-fs: route "/" has no filename component`);
  }
  if (relative.startsWith("/")) {
    throw new Error(`forme-emit-fs: route ${JSON.stringify(route)} starts with multiple slashes`);
  }
  // Compose + resolve.  resolve() normalises away `.` and `..`
  // segments syntactically.
  const absOutDir = resolve(outDir);
  const candidate = resolve(absOutDir, relative);
  // Containment check.  Append a trailing separator to absOutDir so
  // `absOutDir = "/a/b"` does not accidentally permit
  // `candidate = "/a/bad"` (prefix-but-not-parent attack).
  const guard = absOutDir.endsWith(sep) ? absOutDir : absOutDir + sep;
  if (candidate !== absOutDir && !candidate.startsWith(guard)) {
    throw new Error(
      `forme-emit-fs: route ${JSON.stringify(route)} would escape outDir ${JSON.stringify(outDir)}`,
    );
  }
  return candidate;
}
