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
 *
 * ## Two more win32 rules (WD-100b)
 *
 * `path.win32.relative` compares with `toLowerCase`, which is full Unicode: the
 * Kelvin sign (U+212A) lower-cases to `k`. A containment check must fail
 * *closed*, so this module normalises with `resolve` and then compares
 * *components* itself, folding `A`..`Z` to `a`..`z` and nothing else
 * ({@link asciiFold}). It also refuses a win32 child with a reserved device
 * name (`CON`, `nul.txt`, `COM1`, ...) or a component ending in a dot or space
 * ({@link refusedComponent}).
 */

import * as nodePath from "node:path";

const DEVICE = /^[\\/]{2}[?.][\\/]/;
/** Exactly two leading slashes: implementation-defined in POSIX, refused (VER04i 9.1). */
const POSIX_DOUBLE_SLASH = /^\/\/(?!\/)/;

/** The Windows device names, upper-case (VER04i 9.1, WD-100b). */
export const RESERVED_NAMES: ReadonlySet<string> = new Set([
  "CON",
  "PRN",
  "AUX",
  "NUL",
  ...Array.from({ length: 9 }, (_, i) => `COM${i + 1}`),
  ...Array.from({ length: 9 }, (_, i) => `LPT${i + 1}`),
  "CONIN$",
  "CONOUT$",
  ...["COM", "LPT"].flatMap((d) => ["\u00b9", "\u00b2", "\u00b3"].map((sup) => d + sup)),
]);

/** `A`..`Z` mapped to `a`..`z` and nothing else (`toLowerCase` would fold U+212A to `k`). */
export function asciiFold(text: string): string {
  return text.replace(/[A-Z]/g, (c) => String.fromCharCode(c.charCodeAt(0) + 32));
}

/**
 * Is this win32 component refused: a colon, a trailing dot or space, or a device stem?
 * A colon means an alternate data stream or device suffix (`CON:`, `nul::$DATA`,
 * `file.txt:NUL`). The stem is the part before the first dot with trailing
 * spaces removed, so `CON.txt`, `com1.tar.gz` and `CON .txt` are devices while
 * `console.txt`, `COM10`, `LPT0` and `CONx` are not. The set has `CONIN$`,
 * `CONOUT$` and the superscript digits; only `A`..`Z` fold, so those match
 * exactly, and fullwidth `CON` and `CLOCK$` stay ordinary.
 */
export function refusedComponent(name: string): boolean {
  if (name.includes(":") || name.endsWith(".") || name.endsWith(" ")) return true;
  const dot = name.indexOf(".");
  const stem = (dot < 0 ? name : name.slice(0, dot)).replace(/ +$/, "");
  return RESERVED_NAMES.has(stem.replace(/[a-z]/g, (c) => String.fromCharCode(c.charCodeAt(0) - 32)));
}

/** The slice of a `node:path` module this check needs (`posix` or `win32`). */
export type PathModule = Pick<typeof nodePath.posix, "parse" | "resolve" | "sep">;

/** A resolved path as its anchor (`/`, `D:\`, `\\server\share\`) and its whole components. */
function split(resolved: string, mod: PathModule): [string, string[]] {
  const { root } = mod.parse(resolved);
  return [root, resolved.slice(root.length).split(mod.sep).filter((part) => part !== "")];
}

/**
 * `child` relative to `root` when it lies at or beneath `root`, otherwise `null`.
 * The root itself yields the empty string; callers that need a *strict*
 * descendant reject `""` as well.
 */
export function relativeInside(root: string, child: string, mod: PathModule = nodePath): string | null {
  const win32 = mod.sep === "\\";
  // A device-namespace path (`\\?\`, `\\.\`) is never inside anything, and nothing is inside it.
  if (win32 && (DEVICE.test(root) || DEVICE.test(child))) return null;
  if (!win32 && (POSIX_DOUBLE_SLASH.test(root) || POSIX_DOUBLE_SLASH.test(child))) return null;
  const [rootAnchor, rootParts] = split(mod.resolve(root), mod);
  const [childAnchor, childParts] = split(mod.resolve(child), mod);
  const fold = win32 ? asciiFold : (text: string): string => text;
  if (fold(rootAnchor) !== fold(childAnchor) || childParts.length < rootParts.length) return null;
  if (rootParts.some((part, i) => fold(part) !== fold(childParts[i] as string))) return null;
  if (win32 && childParts.some(refusedComponent)) return null;
  return childParts.slice(rootParts.length).join(mod.sep);
}
