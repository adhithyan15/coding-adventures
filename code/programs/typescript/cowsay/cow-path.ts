/**
 * Choosing a cow file safely
 * ==========================
 *
 * `-f NAME` / `--file NAME` picks which cow to draw. The name is glued into a
 * path — `<cows dir>/NAME.cow` — so it is untrusted input that decides which
 * file this program opens and echoes to stdout. Left unchecked, a name can
 * walk out of the cows directory (issue #12169):
 *
 *   name given                    path actually opened
 *   ───────────────────────────── ────────────────────────────────────
 *   tux                           code/specs/cows/tux.cow     (fine)
 *   ../../../../home/me/notes     /home/me/notes.cow          (escape)
 *   C:\Users\me\x                 C:\Users\me\x.cow           (Windows)
 *
 * The forced `.cow` suffix limits what can be read, and a local CLI already
 * runs with its caller's privileges — but a wrapper script, web service or CI
 * job that forwards someone else's string to `cowsay -f` would hand that
 * someone a file-read primitive. So we defend in two layers, mirroring the
 * C#, F#, Java, Kotlin, Perl, Haskell, Dart, Lua and Swift ports:
 *
 * 1. Syntactic check (`isSafeCowName`). A cow name is a bare file stem, so
 *    anything that could mean "some other directory" is refused:
 *
 *      /     path separator
 *      \     path separator on Windows
 *      ..    parent-directory step
 *      :     Windows drive letter (`C:x`) / alternate data stream
 *      NUL   C APIs stop reading at it, so the OS would see a different
 *            name from the one we checked
 *      ""    the empty name
 *
 *    An absolute path always contains `/`, `\` or `:`, so it falls to the
 *    same rule.
 *
 * 2. Containment check (`resolveCowPath`). After joining, both the cows
 *    directory and the candidate are canonicalized with `fs.realpathSync`
 *    (symlinks and `.`/`..` resolved by the OS) and the candidate must still
 *    sit inside the cows directory. This catches what string inspection
 *    cannot see — e.g. a symlink planted in the cows directory that points
 *    somewhere else.
 *
 * A name failing either layer is treated exactly like a cow that does not
 * exist: we quietly draw `default.cow`, matching every other port and
 * cowsay's long-standing "unknown cow → default cow" behaviour. Nothing is
 * URL-decoded: `%2F` is three literal characters to the OS, never a slash.
 *
 * This module imports only Node built-ins so it can be tested on its own.
 */

import * as fs from "fs";
import * as path from "path";

/** Characters that let a "name" smuggle in a directory, a drive, or a truncation. */
const FORBIDDEN_CHARS = ["/", "\\", ":", "\0"];

/** True when `cowName` is a bare file stem that cannot name another directory. */
export function isSafeCowName(cowName: unknown): cowName is string {
  if (typeof cowName !== "string" || cowName === "" || cowName.includes("..")) {
    return false;
  }
  return !FORBIDDEN_CHARS.some((ch) => cowName.includes(ch));
}

/**
 * Map `cowName` to a `.cow` file inside `cowsDir`, falling back to
 * `cowsDir/default.cow` when the name is unsafe, the file is missing, or the
 * resolved file escapes `cowsDir`.
 */
export function resolveCowPath(cowName: unknown, cowsDir: string): string {
  const defaultPath = path.join(cowsDir, "default.cow");
  if (!isSafeCowName(cowName)) {
    return defaultPath;
  }

  try {
    // realpathSync throws for a path that does not exist — the "unknown cow"
    // case — so any error here simply means "use the default".
    const root = fs.realpathSync(cowsDir);
    const resolved = fs.realpathSync(path.join(cowsDir, `${cowName}.cow`));

    // path.relative yields a path starting with ".." exactly when `resolved`
    // lies outside `root`, and an absolute path when they sit on different
    // Windows drives.
    const rel = path.relative(root, resolved);
    const inside =
      rel !== "" &&
      rel !== ".." &&
      !rel.startsWith(`..${path.sep}`) &&
      !path.isAbsolute(rel);
    if (inside && fs.statSync(resolved).isFile()) {
      return resolved;
    }
  } catch {
    // Missing file, permission problem, or broken symlink: fall through.
  }
  return defaultPath;
}
