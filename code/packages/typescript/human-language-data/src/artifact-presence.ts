// ---------------------------------------------------------------------------
// artifact-presence.ts — "is this file there?", answered without lying.
//
// Node's `existsSync` returns a boolean, and a boolean has exactly two states
// for a question that has three answers:
//
//     the file is not there            -> false, correctly
//     the file is there                -> true,  correctly
//     I was not allowed to look        -> false, WRONGLY
//
// `existsSync` is `statSync` wrapped in `catch { return false }`, so every
// errno collapses into "absent". EACCES on a directory whose permissions a CI
// runner got wrong, EPERM behind a Windows ACL, EMFILE when a build has run out
// of descriptors, EIO on a flaky network mount — all of them arrive as "this
// artifact does not exist", which is a statement about the repository that the
// process is in no position to make.
//
// Which way that lie points decides whether it matters. Here it points at
// SAFETY in the wrong direction: a checker built on `existsSync` reports MORE
// missing artifacts under an I/O fault, so it fails loudly and someone looks.
// But the same helper is used to decide whether a track HAS an assessment
// contract at all (`loader.ts`), and there the lie points the other way: an
// unreadable `assessment.json` reads as "this track has no contract yet",
// the track drops out of the audit entirely, and every artifact it dangles
// stops being counted. A gate that goes quiet when the disk misbehaves is the
// exact failure this repository has now fixed twice (#12731, #12734).
//
// So: only `ENOENT` (no such file or directory) and `ENOTDIR` (a path
// component that is not a directory — `mocks/a1/rubric.md` where `mocks/a1` is
// a regular file) mean absent. Everything else throws, naming the errno, so the
// build says "I could not determine this" instead of "it is not there".
// ---------------------------------------------------------------------------
import { statSync } from "node:fs";

/**
 * The two errnos that genuinely mean "nothing lives at this path".
 *
 * `ENOENT` is the ordinary answer. `ENOTDIR` is the same answer arriving one
 * component early: the kernel stopped walking because it hit a file where a
 * directory was required, which likewise proves the full path is not there.
 * Every other errno describes the LOOKUP failing, not the FILE being absent.
 */
const ABSENT_CODES = new Set(["ENOENT", "ENOTDIR"]);

/** The one call this module makes, parameterised so its failure paths are testable. */
export type StatProbe = (path: string) => unknown;

/**
 * Does something exist at `path`?
 *
 * @param path  an absolute filesystem path
 * @param probe the stat call; injectable so a test can produce an EACCES without
 *              needing a filesystem that can be made unreadable on all three CI
 *              platforms. The throwing branch below is the entire point of this
 *              module, and a branch no test can reach is a branch nobody has
 *              seen work.
 * @throws when the filesystem answered with anything other than "absent"
 */
export function artifactExists(path: string, probe: StatProbe = statSync): boolean {
  try {
    probe(path);
    return true;
  } catch (error) {
    const code = (error as NodeJS.ErrnoException | null)?.code;
    if (typeof code === "string" && ABSENT_CODES.has(code)) return false;
    throw new Error(
      `artifact presence: could not determine whether '${path}' exists ` +
        `(${code ?? "no errno on the thrown value"}). This is not the same as the file ` +
        `being absent, and must not be reported as such.`,
      { cause: error },
    );
  }
}
