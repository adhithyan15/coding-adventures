// doc-shard-cli — split a Markdown document into `X.d/`, put it back together,
// or check that the two agree (spec: HL21, extended to prose by HL22).
//
// ---------------------------------------------------------------------------
// What this is for
// ---------------------------------------------------------------------------
//
// `src/doc-shard.ts` taught this package to split and rejoin a Markdown
// document. This is the other half: the tool that moves a document between the
// two forms and, more importantly, the `--check` that stops them drifting apart.
//
// The compromise is the same one `shard-cli` makes, and compromises rot when
// nobody remembers what they bought:
//
//   * `BACKLOG.d/` and `CHANGELOG.d/` are the SOURCE OF TRUTH. They are what
//     authors edit, and the reason they exist is that one file per entry means
//     five parallel level-authoring agents touch five different files.
//   * `BACKLOG.md` and `CHANGELOG.md` are GENERATED ARTIFACTS.
//
// The monoliths survive here for DIFFERENT reasons than `core/spine.json`'s,
// and it is worth being precise because the obvious guess is wrong. HL21 kept
// `core/spine.json` because `language-ladder` imports it statically into a
// browser bundle with a 500 kB eager budget. That constraint was checked here
// and does NOT apply: every Vite glob in `language-ladder/src` requires a
// subdirectory segment (`*/lessons/*.md`, `*/curriculum.json`, …), so neither of
// these files can match one, and neither is bundled at all. Nothing in the
// repository READS either file programmatically — no loader, no test, no
// script, no workflow.
//
// They are kept for two prosaic reasons instead:
//
//   1. Seven documents link to `BACKLOG.md` by relative path, two of them to a
//      section ANCHOR (`../BACKLOG.md#findings-from-hl-c30`). Nothing in CI
//      validates those links, so removing the file would break them silently —
//      the worst way for a link to break.
//   2. `CLAUDE.md` requires every package to carry a `CHANGELOG.md`. That rule
//      is policy rather than machinery — there is no changelog gate in CI, which
//      was checked — but it is the repo's convention and a reader opening the
//      package expects the file to be there.
//
// The rule that makes a generated monolith safe is the one from the modality
// manifest and the book chapters: a derived file that nothing verifies is worse
// than no file. So `--check` runs in CI beside `check:shards`, and a stale
// `BACKLOG.md` fails the build rather than quietly showing a reader an old
// backlog.
//
// ---------------------------------------------------------------------------
// Why the round trip is byte-exact, and why that is cheap here
// ---------------------------------------------------------------------------
//
// `unshard(shard(x))` must equal `x` byte for byte, or the two forms cannot both
// be trusted and `--check` becomes noise that people learn to ignore.
//
// On the JSON side that took work: `JSON.stringify(value, null, 2) + "\n"` had
// to be verified to reproduce the committed bytes, and the array had to be the
// last top-level key so it would land back where it started. None of that
// applies to prose. This tool never re-serializes anything — it slices the
// file's bytes at heading boundaries and writes the slices out verbatim, so the
// rebuild is `concat`, and `concat(split(x)) === x` for the same reason cutting
// a rope and taping it back together gives you the rope. `splitDocument`
// asserts it anyway, on every run.
//
// The migration therefore introduces NO normalization whatsoever. The
// regenerated monoliths are byte-identical to the pre-migration files, which was
// demonstrated by diffing them rather than by reasoning about them.

// `existsSync` is deliberately NOT imported. It uses `stat`, so it follows
// symlinks and reports a dangling link as absent — which silently skipped a
// guard placed behind it, twice, in two consecutive review rounds of the JSON
// sharder. Use `statIfPresent` below. Leaving the import out makes the next
// reach for it a compile error rather than a judgement call.
import {
  lstatSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, normalize, relative as pathRelative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { assertRelativeManifestPath } from "./manifest-path.js";
import {
  type DocShardPlan,
  assertRealDocFile,
  docShardContents,
  docShardDirectoryFor,
  isDocSharded,
  joinDocShards,
  listDocShardNames,
  readDocShards,
  readDocumentFile,
} from "./doc-shard.js";

/**
 * The repository root.
 *
 * Not `defaultCurriculumRoot()`, and that difference is the one structural thing
 * this CLI does that `shard-cli` does not. The two documents live in two
 * different trees — one under `code/learning/human-languages`, one under
 * `code/packages/typescript/human-language-data` — so there is no single
 * narrower anchor that contains both.
 *
 * Widening the root widens what a path could reach, so the CLI never accepts an
 * arbitrary path to operate on: `--shard <path>` FILTERS `DOC_SHARD_PLANS` by
 * exact match and refuses anything not already in the table. A path that is not
 * a plan cannot be written, whatever it says. `safeDocumentPath` below is
 * belt-and-braces on top of that, applied to the table's own entries so that a
 * later edit to the table cannot quietly introduce an escape.
 *
 * The `..` count is the same from `src/` and from `dist/`, because `tsc` emits
 * `dist/` as a sibling of `src/` with `rootDir: "./src"`. That was checked
 * against `tsconfig.json`, not assumed — a CLI that resolves a different root
 * when compiled than when tested is a bug that only ever appears in CI.
 */
export function defaultRepoRoot(): string {
  const here = dirname(fileURLToPath(import.meta.url));
  // src|dist -> human-language-data -> typescript -> packages -> code -> repo
  return resolve(here, "..", "..", "..", "..", "..");
}

/**
 * The Markdown documents HL22 has migrated. Grows one entry per follow-on PR.
 *
 * These two were chosen by measurement, not by size. Of the last 200
 * human-languages commits on `main`, `BACKLOG.md` was touched by 100 and this
 * package's `CHANGELOG.md` by 75, while the per-language `<track>/CHANGELOG.md`
 * files were touched 4-11 times each. The per-language changelogs are already
 * partitioned by track and are deliberately left alone: sharding a file that
 * does not conflict buys nothing and costs a directory.
 */
export const DOC_SHARD_PLANS: readonly DocShardPlan[] = [
  {
    // 107 `##` sections, newest at the top. One of them is not an `HL-` entry
    // (`## Three rules this work keeps re-deriving`), which is why the shard
    // name is derived from the heading text rather than from an id pattern —
    // an id-based scheme would have had to special-case it or reject the file.
    path: "code/learning/human-languages/BACKLOG.md",
    headingLevel: 2,
    newestFirst: true,
  },
  {
    // 436 `###` entries, newest at the top. Split at level 3 rather than level
    // 2 because level 2 is the version heading and the hot spot is the entry
    // list underneath `## Unreleased` — every PR prepends there. The frozen
    // `## [0.3.0]` markers ride along inside the entry above them; see the
    // finding at the bottom of this file for the state they are in.
    path: "code/packages/typescript/human-language-data/CHANGELOG.md",
    headingLevel: 3,
    newestFirst: true,
  },
];

/**
 * `lstatSync`, or `undefined` if the path is not there at all.
 *
 * The replacement for `existsSync` throughout this file. `existsSync` uses
 * `stat`, which follows symlinks, so a DANGLING link reads as "not there" and
 * every guard placed behind one is skipped. `lstatSync` describes the link
 * itself, which is the thing being asked about.
 */
function statIfPresent(path: string): ReturnType<typeof lstatSync> | undefined {
  try {
    return lstatSync(path);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return undefined;
    throw error;
  }
}

/**
 * Resolve a document path inside the repository root, or throw.
 *
 * Lifted from `shard-cli`'s `safeLedgerPath`, for the same reason and with the
 * same shape. Containment is decided AFTER `resolve`, never by inspecting the
 * input string for `..` — `a/b/../../../etc` contains no leading `..` and still
 * escapes.
 *
 * `assertRelativeManifestPath` runs FIRST and is the part that is easy to think
 * unnecessary. `path.relative()` is not a containment check on Windows: when the
 * two paths sit on different roots it cannot express the journey as `..` steps,
 * so it returns the target unchanged. `relative('C:\\repo', 'D:\\evil.md')` is
 * `'D:\\evil.md'`, which is neither `".."` nor `"../"`-prefixed and sails
 * through the lexical test below. UNC paths (`\\server\share\evil.md`) do the
 * same, and turn a build step into an outbound write.
 *
 * `isAbsolute` alone cannot express that rule either, because it is
 * PLATFORM-DEPENDENT: on POSIX `D:\evil.md` is an ordinary relative filename, so
 * the check silently does nothing. The shared helper applies the drive-letter
 * and UNC patterns on EVERY platform, so the rule a document path must satisfy
 * is the same wherever it is read, and the seven CLIs in this package now share
 * one definition of it rather than seven.
 */
export function safeDocumentPath(root: string, relative: string): string {
  assertRelativeManifestPath(
    relative,
    `doc-shard-cli: document path must be relative to the repository root, got '${relative}'`,
  );
  const output = resolve(root, relative);
  const fromRoot = normalize(pathRelative(resolve(root), output)).replaceAll("\\", "/");
  if (
    fromRoot === "" ||
    fromRoot === ".." ||
    fromRoot.startsWith("../") ||
    !fromRoot.endsWith(".md")
  ) {
    throw new Error(`doc-shard-cli: unsafe document path '${relative}'`);
  }

  // And now the part that lexical containment cannot see.
  //
  // `lstat` does not follow the FINAL component — which is what every other
  // guard in this feature checks — but it follows every component BEFORE it. So
  // `BACKLOG.d` committed as a link is refused, and `human-languages` committed
  // as a link sails through: `rmSync` deletes out-of-tree files and
  // `writeFileSync` overwrites them, with every earlier guard satisfied.
  //
  // Only `realpath` can see this. Resolving the PARENT closes every intermediate
  // component at once, and returning the resolved path means the downstream
  // operations act on the real location rather than re-walking the links.
  //
  // Comparing realpath to realpath, not realpath to `root`, matters: on macOS
  // `/var` is a link to `/private/var`, so a checkout under a symlinked path
  // would otherwise fail this check for no reason. This repository is routinely
  // checked out under OneDrive on Windows and under `/tmp` worktrees on CI,
  // both of which are exactly that situation.
  const realParent = realpathSync(dirname(output));
  const inside = normalize(pathRelative(realpathSync(root), realParent)).replaceAll("\\", "/");
  if (inside === ".." || inside.startsWith("../")) {
    throw new Error(
      `doc-shard-cli: '${relative}' resolves outside the repository root — ` +
        `a parent directory is a symbolic link`,
    );
  }
  return join(realParent, basename(output));
}

/**
 * The document bytes that the shards on disk currently mean.
 *
 * Pure enough to be the shared definition: `--unshard` writes what this returns
 * and `--check` compares against what this returns, so the two cannot disagree
 * about what the shards say.
 */
export function unshardDocContents(root: string, plan: DocShardPlan): string {
  const monolith = safeDocumentPath(root, plan.path);
  const shards = readDocShards(monolith);
  if (shards === null) {
    throw new Error(`${plan.path}: no ${docShardDirectoryFor(plan.path)} to rebuild from`);
  }
  return joinDocShards(shards, plan);
}

/**
 * Split a document into its `.d/` directory.
 *
 * THE GATE HERE IS `isDocSharded`, NOT `existsSync`, and the difference is the
 * whole security of this function. `existsSync` FOLLOWS symlinks, so with
 * `BACKLOG.d` committed as a symlink — which git tracks as a first-class object,
 * so a pull request can contain one — an `existsSync`-gated cleanup would delete
 * every `*.md` in the link's TARGET and then write shards there. Pointed at a
 * notes directory or a sibling checkout, `npm run shard:docs` on that branch is
 * arbitrary file deletion. `listDocShardNames`'s per-entry symlink check does
 * not help: entries reached THROUGH a symlinked parent report
 * `isSymbolicLink() === false`.
 *
 * `isDocSharded` uses `lstatSync` and throws on a link, so the whole branch is
 * refused before anything is unlinked. This is the JSON sharder's lesson applied
 * without having to relearn it: a guard that lives only in the reader is a guard
 * the writer forgets, and the writer is the dangerous one.
 */
export function shardDocument(root: string, plan: DocShardPlan): string[] {
  const monolith = safeDocumentPath(root, plan.path);
  const text = readDocumentFile(monolith);
  const contents = docShardContents(text, plan);
  const dir = docShardDirectoryFor(monolith);

  // Remove shards the document no longer produces. Leaving them behind would
  // make the next `--check` fail with an entry nobody can find in the source —
  // the "unexpected stale shard" case `modality-cli` already guards.
  if (isDocSharded(monolith)) {
    for (const name of listDocShardNames(monolith)) rmSync(join(dir, name));
  } else if (statIfPresent(dir) !== undefined) {
    // `isDocSharded` returned false for a reason other than "absent" only if
    // something that is not a directory is squatting on the name. Refuse it
    // rather than letting `mkdirSync(..., { recursive: true })` no-op over it.
    throw new Error(`doc-shard-cli: '${dir}' exists and is not a directory`);
  }

  mkdirSync(dir, { recursive: true });
  const written: string[] = [];
  for (const [name, body] of contents) {
    // `wx` is O_EXCL, which does not follow symlinks and fails if the path
    // exists. Between the `rmSync` above and this write, anyone able to write
    // into the shard directory could otherwise plant a symlink at a shard name
    // and have the write follow it. Every name was just removed, so a collision
    // here means someone else is writing concurrently — which should fail the
    // run, not be papered over.
    writeFileSync(join(dir, name), body, { encoding: "utf8", flag: "wx" });
    written.push(name);
  }
  return written;
}

/**
 * Rebuild the monolith from the shards. Returns the bytes written.
 *
 * `assertRealDocFile` before the write, unconditionally, because `open(2)` with
 * `O_WRONLY|O_TRUNC` follows symlinks: with `CHANGELOG.md` committed as a link,
 * this would truncate and overwrite the link's target. The file is expected to
 * exist already — it is a generated artifact under version control — so a
 * missing one is a broken checkout worth reporting rather than silently
 * creating, and `--check`'s own failure message ("run npm run unshard:docs") is
 * exactly what would otherwise walk a maintainer into it.
 */
export function unshardDocument(root: string, plan: DocShardPlan): string {
  const body = unshardDocContents(root, plan);
  const monolith = safeDocumentPath(root, plan.path);
  assertRealDocFile(monolith);
  writeFileSync(monolith, body, "utf8");
  return body;
}

export function runDocShardCli(
  args = process.argv.slice(2),
  root = defaultRepoRoot(),
): number {
  const usage =
    "usage: doc-shard-cli (--shard <path> | --unshard <path> | --check [<path>])\n";
  const mode = args[0];
  if (mode !== "--shard" && mode !== "--unshard" && mode !== "--check") {
    process.stderr.write(usage);
    return 2;
  }
  if ((mode === "--shard" || mode === "--unshard") && args.length !== 2) {
    process.stderr.write(usage);
    return 2;
  }
  if (mode === "--check" && args.length > 2) {
    process.stderr.write(usage);
    return 2;
  }

  const requested = args[1];
  const plans = requested
    ? DOC_SHARD_PLANS.filter((plan) => plan.path === requested.replaceAll("\\", "/"))
    : DOC_SHARD_PLANS;
  if (requested && plans.length === 0) {
    process.stderr.write(
      `doc-shard-cli: '${requested}' is not a sharded document. ` +
        `Known: ${DOC_SHARD_PLANS.map((plan) => plan.path).join(", ")}\n`,
    );
    return 2;
  }

  let failed = false;
  for (const plan of plans) {
    if (mode === "--shard") {
      const written = shardDocument(root, plan);
      process.stdout.write(`sharded ${plan.path} into ${written.length} files\n`);
      continue;
    }
    if (mode === "--unshard") {
      unshardDocument(root, plan);
      process.stdout.write(`rebuilt ${plan.path}\n`);
      continue;
    }

    // --check: do the two representations agree?
    const monolith = safeDocumentPath(root, plan.path);
    if (!isDocSharded(monolith)) {
      process.stderr.write(`${plan.path}: ${docShardDirectoryFor(plan.path)} is missing\n`);
      failed = true;
      continue;
    }
    const expected = unshardDocContents(root, plan);
    // Guarded even though this only READS: `--check` is the one mode CI runs, so
    // it is the one mode a hostile branch can reach on a maintainer's runner. A
    // symlinked monolith would otherwise have its target's bytes read, compared,
    // and the comparison result reported.
    //
    // `lstatSync`, not `existsSync`, for the third time in this file: a dangling
    // link is invisible to `existsSync`, so the guard behind one never runs.
    let actual: string | undefined;
    if (statIfPresent(monolith) !== undefined) {
      assertRealDocFile(monolith);
      actual = readFileSync(monolith, "utf8");
    }
    if (actual !== expected) {
      // Same failure, same fix, one message — as `modality-cli` puts it.
      process.stderr.write(
        `${plan.path}: generated monolith is missing or stale. ` +
          `Run 'npm run unshard:docs' and commit the result.\n`,
      );
      failed = true;
    }

    // And the reverse direction: a file in the shard directory that the join
    // silently ignored.
    //
    // This deliberately does NOT compare filenames against what `--shard` would
    // emit today, which is what the JSON `--check` does. Filenames here carry an
    // ordinal that authors are expected to choose by hand — an entry wedged in
    // as `00155-…` is a legitimate, conflict-free way to insert between two
    // others, and HL21 §2.2 explicitly promises that passes without a renumber.
    // Requiring canonical names would break that promise, and would also fail
    // any shard whose heading was edited without its file being renamed.
    //
    // A missing or corrupted `.md` shard needs no separate check: the rebuilt
    // document IS the shards, so losing one shows up immediately in the byte
    // comparison above. What that comparison cannot see is a file the join never
    // looked at. `listDocShardNames` takes only `*.md`, so an entry saved as
    // `.markdown`, `.txt` or `.md.orig` — the last being what a botched merge
    // leaves behind — sits in the directory looking like content and contributes
    // nothing. The document still rebuilds cleanly, which is exactly why nobody
    // would notice.
    const ignored = readdirSync(docShardDirectoryFor(monolith), { withFileTypes: true })
      .filter((entry) => !entry.isDirectory() && !entry.name.endsWith(".md"))
      .map((entry) => entry.name);
    for (const name of ignored) {
      process.stderr.write(
        `${plan.path}: '${name}' is in ${docShardDirectoryFor(plan.path)} but is not a ` +
          `*.md shard, so its contents are not in the document. Rename it or remove it.\n`,
      );
      failed = true;
    }
  }
  return failed ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runDocShardCli());
}
