// ---------------------------------------------------------------------------
// manifest-path.ts — the shape check every manifest path must pass first.
//
// Six generator CLIs take a path out of a checked-in manifest, join it to the
// curriculum root, and write a file there. Each guards containment the same
// way: resolve, then ask `path.relative(root, resolved)` whether the answer
// escapes upward.
//
// That inference is sound on POSIX and WRONG on win32. When the two paths sit
// on different roots, `path.relative()` cannot express the journey as a series
// of `..` steps, so it gives up and returns the target absolute and unchanged.
// The result never begins with `../`, so an upward-escape check waves it
// through. Measured on Node 24, with root `C:/repo/curriculum`:
//
//     "../../../evil.tex"          -> "../../evil.tex"          rejected
//     "C:\\Windows\\Temp\\evil.tex" -> "../../Windows/..."       rejected
//     "/absolute/evil.tex"         -> "../../absolute/evil.tex" rejected
//     "D:\\evil.tex"                -> "D:/evil.tex"             ACCEPTED
//     "\\\\server\\share\\evil.tex"  -> "//server/share/evil.tex" ACCEPTED
//
// The last two write outside the curriculum root — onto another local volume,
// or onto an SMB share, which turns a build step into an outbound write.
//
// The fix is to stop inferring the input's shape from the output. A manifest
// path is supposed to be RELATIVE; check that directly, before resolving, and
// the platform's join semantics stop mattering. This is deliberately a
// separate, cheap, total function rather than a rewrite of the six containment
// checks: they stay as they are and keep catching `..` traversal, and this runs
// in front of them.
// ---------------------------------------------------------------------------
import { isAbsolute } from "node:path";

/** `D:\evil.tex`, `d:/evil.tex` — a drive-relative or drive-absolute path. */
const DRIVE_QUALIFIED = /^[A-Za-z]:/;

/** `\\server\share\evil.tex`, `//server/share/evil.tex` — a UNC path. */
const UNC = /^[\\/]{2}/;

/**
 * Throw unless `relative` is a plain relative path.
 *
 * `isAbsolute` alone is not enough: it is platform-dependent, so a POSIX CI box
 * would not recognise `D:\evil.tex` as absolute and would happily pass it to a
 * Windows developer's manifest check later. Both extra patterns are therefore
 * applied on every platform, so the rule a manifest must satisfy is the same
 * everywhere the manifest is read.
 *
 * @param relative the raw manifest value
 * @param message  what to say when it is rejected; the caller owns the wording
 *                 so the existing per-CLI error text is preserved
 */
export function assertRelativeManifestPath(relative: string, message: string): void {
  if (isAbsolute(relative) || DRIVE_QUALIFIED.test(relative) || UNC.test(relative)) {
    throw new Error(message);
  }
}
