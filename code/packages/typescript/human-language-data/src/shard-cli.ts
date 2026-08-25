// shard-cli — split a ledger into `X.d/`, put it back together, or check that
// the two agree (spec: HL21).
//
// ---------------------------------------------------------------------------
// What this is for
// ---------------------------------------------------------------------------
//
// `src/shard.ts` taught the loader to READ a ledger that lives as a directory.
// This is the other half: the tool that moves a ledger between the two forms
// and, more importantly, the `--check` that stops them drifting apart.
//
// The reason both forms exist at all is worth stating plainly, because it is a
// compromise and compromises rot when nobody remembers what they bought:
//
//   * `core/spine.d/` is the SOURCE OF TRUTH. It is what authors edit, and the
//     reason it exists is that one file per node means two people adding two
//     nodes touch two different files.
//   * `core/spine.json` is a GENERATED ARTIFACT. It survives because
//     `code/programs/typescript/language-ladder/src/curriculum.ts` does
//
//         import spineJson from ".../core/spine.json";
//
//     a static JSON import, resolved by Vite into a browser bundle. A browser
//     cannot `readdirSync`, so that import cannot follow the shards. Rewriting
//     it as an `import.meta.glob` was considered and rejected: the app has a
//     500 kB eager-bundle budget enforced by `scripts/check-bundle.mjs`, glob
//     imports interact with that budget in ways that would need their own
//     investigation, and this change should not need to reason about bundle
//     chunking to land. Keeping a generated monolith costs one `--check` in CI
//     and nothing else.
//
// The rule that makes that safe is the one from the modality manifest and the
// book chapters: a derived file that nothing verifies is worse than no file. So
// `--check` runs in CI beside `check:books`, and a stale `core/spine.json`
// fails the build rather than quietly serving the app an old spine.
//
// ---------------------------------------------------------------------------
// Why the round trip has to be byte-exact
// ---------------------------------------------------------------------------
//
// `unshard(shard(x))` must equal `x` byte for byte, or the two forms cannot
// both be trusted and `--check` becomes a source of noise that people learn to
// ignore. Two things make that hold here, and both were verified rather than
// assumed:
//
//   1. `JSON.stringify(value, null, 2) + "\n"` reproduces the committed
//      `core/spine.json` exactly. Checked before writing a line of this file.
//   2. The node ORDER is preserved. This is the part that is easy to get wrong.
//      The spine is an ordered ladder — pre-A1 first, C2 last — and within a
//      stage the nodes are NOT alphabetical. Naming shards `<NODE-ID>.json` and
//      reading them in sorted order would silently re-sort the ladder, so the
//      shard filename carries a zero-padded ordinal prefix and sorted filename
//      order reproduces authored order.
//
// The ordinals are spaced by ten — `0010`, `0020`, … — so a node can be
// inserted between two others as `0015` without renaming its neighbours.
// Renaming neighbours would be its own merge conflict, which is the thing this
// whole exercise exists to avoid. `--shard` renumbers to the canonical stride,
// but `--check` compares the REBUILT MONOLITH rather than the filenames, so a
// hand-inserted `0015-SPINE-NEW.json` passes without anyone having to renumber.

// `existsSync` is deliberately NOT imported. It uses `stat`, so it follows
// symlinks and reports a dangling link as absent — which silently skipped a
// guard placed behind it, twice, in two consecutive review rounds. Use
// `statIfPresent` below. Leaving the import out makes the next reach for it a
// compile error rather than a judgement call.
import { lstatSync, mkdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, isAbsolute, join, normalize, relative as pathRelative, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { defaultCurriculumRoot } from "./loader.js";
import { assertRelativeManifestPath } from "./manifest-path.js";
import {
  META_SHARD,
  assertRealFile,
  isSharded,
  listShardNames,
  readLedgerFile,
  readShards,
  shardDirectoryFor,
} from "./shard.js";

/**
 * What happens to `X.json` once `X.d/` exists.
 *
 * This is the decision that decides whether the migration actually BUYS
 * anything, and it is easy to get backwards, so it is a required field rather
 * than a defaulted one.
 *
 *   * `"generated"` — the monolith stays, as a derived artifact, and `--check`
 *     compares its bytes against the rebuild. `core/spine.json` is the only
 *     ledger in this mode, and only because language-ladder statically imports
 *     it into a BROWSER bundle, where `readdirSync` does not exist.
 *
 *   * `"removed"` — the monolith is deleted and `--check` fails if it comes
 *     back. This is the mode that removes the conflict.
 *
 * The distinction matters more than it looks. A `"generated"` monolith is
 * regenerated by every tranche that appends an element, so every pair of
 * tranches still collides on it — the merge conflict this whole exercise exists
 * to remove survives, wearing a different hat. Sharding a hot ledger and
 * KEEPING its monolith buys nothing at all. `core/spine.json` gets away with it
 * only because it has 33 nodes that almost never change; `spanish/chapters.json`
 * grows several times a week and would not.
 *
 * So the rule is: keep the monolith only when something that cannot read a
 * directory genuinely needs it, and say which thing, in the plan, in writing.
 */
export type MonolithDisposition = "generated" | "removed";

/**
 * How one ledger splits.
 *
 * A table rather than a `switch`, because HL21 migrates several more ledgers
 * after this one — `<track>/curriculum.json`'s four top-level keys,
 * `core/book-generation.json` — and each is the same handful of facts: which
 * file, which array, what to call each element's shard, and what becomes of the
 * monolith afterwards.
 */
export interface ShardPlan {
  /** Ledger path relative to the curriculum root, POSIX-separated. */
  readonly path: string;
  /** The top-level array key that becomes one file per element. */
  readonly listKey: string;
  /**
   * The stable part of an element's filename, without ordinal or extension.
   *
   * OPTIONAL. When it is absent the shard is named by its ordinal alone —
   * `0007.json` — which is what a ledger keyed by a number rather than by a
   * string id wants. `<track>/chapters.json` is the worked example: a chapter's
   * identity IS its number, and `0007-7.json` would be saying it twice.
   *
   * When present, the returned string must be a safe filename. It is validated,
   * not trusted: `idOf` reads a field out of authored JSON, and an id of
   * `../../..` or `con` would otherwise decide where this tool writes.
   */
  readonly idOf?: (element: unknown, index: number) => string;
  /**
   * The number that puts this element in its place in the filename ordering.
   *
   * OPTIONAL. The default is `(index + 1) * ORDINAL_STRIDE` — 0010, 0020, … —
   * which spaces elements so a new one can be inserted between two others
   * without renaming its neighbours, because a mass rename is a mass merge
   * conflict and that is the thing being removed.
   *
   * A ledger that already carries its own ordering number should return it
   * instead. `chapters.json` returns the chapter number, so chapter 7 lives in
   * `0007.json` for as long as it is chapter 7 — a name a human can predict
   * without consulting the directory, which matters when the point of the
   * exercise is that two authors write two different files without coordinating.
   */
  readonly ordinalOf?: (element: unknown, index: number) => number;
  /** What becomes of `X.json` once `X.d/` exists. See `MonolithDisposition`. */
  readonly monolith: MonolithDisposition;
}

/**
 * The tracks whose `chapters.json` has been sharded.
 *
 * NOT every track. `french`, `japanese` and `marwadi` are deliberately absent:
 * their committed `chapters.json` is hand-formatted with inline one-line arrays
 * (`"spineNodes": ["SPINE-MEET-GREET"]`), which `JSON.stringify(…, null, 2)`
 * expands over three lines. The DATA is identical either way — that was checked
 * by deep comparison, not assumed — but the BYTES are not, so those three
 * cannot be migrated without a reformatting commit that rewrites lines nobody
 * asked to change.
 *
 * HL21's rule is that a ledger which does not round-trip byte-exactly is
 * reported rather than quietly reformatted into agreement with the serialiser,
 * so those three keep their monolith and the loader's fallback keeps reading
 * them. That is precisely what the fallback is for: a migration that can be
 * done in pieces gets done, and the pieces that need a decision wait for one.
 */
const CHAPTER_SHARDED_TRACKS: readonly string[] = [
  "arabic", "bengali", "chinese", "german", "gujarati", "hindi", "italian",
  "kannada", "latin", "malayalam", "marathi", "persian", "portuguese",
  "punjabi", "russian", "sanskrit", "spanish", "tamil", "telugu", "urdu",
];

/**
 * `<track>/chapters.json` -> `<track>/chapters.d/<NNNN>.json`, one per chapter.
 *
 * The chapter number is both the id and the ordinal, so the filename is the
 * zero-padded number and nothing else. Zero-padded because `10.json` sorts
 * before `9.json` and the chapter list is an ordered ladder — this is the §2.2
 * trap, and it is LIVE here rather than hypothetical: every track with ten or
 * more chapters (all but one of them) has an authored order that plain
 * lexicographic sorting of unpadded numbers would scramble.
 */
function chaptersPlan(track: string): ShardPlan {
  return {
    path: `${track}/chapters.json`,
    listKey: "chapters",
    ordinalOf: (element) => (element as { chapter?: unknown }).chapter as number,
    // GENERATED, not removed — and this was measured rather than chosen.
    //
    // Deleting the monolith is what fully removes the conflict, and it was the
    // intent here. It is blocked by the browser, in a way worth writing down
    // because it applies to every ledger language-ladder reads.
    //
    // The app cannot `readdirSync`, so it reads these ledgers through
    // `import.meta.glob`. A glob's MODULES are lazy, but the KEY TABLE it
    // expands to — one string plus one `() => import(…)` arrow per matching
    // file — is ordinary code in the importing module, and that module is
    // eager. Sharding turned 23 matches into ~1,020, which grew the eager
    // chunk by 191 kB (312,216 -> 503,765) and broke the app's hard 500 kB
    // budget in `scripts/check-bundle.mjs`.
    //
    // That budget is a ceiling on debt, so raising it is not an option, and
    // the obvious dodge — moving the capability fields into the generated hash
    // manifest the app already loads — silently costs a real check: the app
    // recomputes each chapter's fingerprint from the CURRENTLY AUTHORED
    // capability, so a capability edited without regenerating shows up as
    // "stale". Reading the capability from the same generated file as the hash
    // would make that comparison agree with itself.
    //
    // So the monolith stays, on HL21 §3's terms: a derived file, gated by
    // `--check`, never hand-edited and never hand-merged. Two authors adding
    // two chapters still write two different shard files and do not collide
    // there; they collide only on the regenerated monolith, where the fix is
    // `npm run unshard` rather than a hand-merge of JSON. That is a smaller win
    // than deletion and it is the one available without either raising a debt
    // ceiling or weakening a drift check.
    monolith: "generated",
  };
}

/** Ledgers HL21 has migrated so far. Grows one entry per follow-on PR. */
export const SHARD_PLANS: readonly ShardPlan[] = [
  {
    path: "core/spine.json",
    listKey: "nodes",
    idOf: (element) => (element as { id?: unknown }).id as string,
    // The one ledger that keeps its monolith, and the reason is at the top of
    // this file: language-ladder's browser bundle imports it statically.
    monolith: "generated",
  },
  ...CHAPTER_SHARDED_TRACKS.map(chaptersPlan),
];

/** Ordinal stride, and the width it is padded to. See the header. */
const ORDINAL_STRIDE = 10;
const ORDINAL_WIDTH = 4;

/**
 * Filenames that are safe on every filesystem this repo is cloned onto.
 *
 * Uppercase, digits and hyphens only. The spine's ids already look like this —
 * that was CHECKED across all 33 rather than assumed from the two that were
 * looked at — but the check stays, because the first id that does not will
 * arrive in a pull request rather than in this file, and `idOf` is reading a
 * field out of authored JSON. An id of `../../../etc/passwd` deciding where
 * this tool writes is exactly the bug that a "the ids are fine" comment
 * produces two years later.
 *
 * Windows reserved device names are refused too. `CON.json` and `PRN.json`
 * cannot be created on Windows, so an id of `CON` would produce a shard set
 * that silently fails to check out on half the machines that use it.
 */
const SAFE_ID = /^[A-Z][A-Z0-9-]*$/;
const WINDOWS_RESERVED = new Set([
  "CON", "PRN", "AUX", "NUL",
  "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
  "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
]);

function assertSafeId(id: unknown, plan: ShardPlan, index: number): string {
  if (typeof id !== "string" || !SAFE_ID.test(id)) {
    throw new Error(
      `${plan.path}: ${plan.listKey}[${index}] has id ${JSON.stringify(id)}, ` +
        `which is not a safe shard filename (want ${SAFE_ID.source})`,
    );
  }
  if (WINDOWS_RESERVED.has(id)) {
    throw new Error(
      `${plan.path}: ${plan.listKey}[${index}] has id '${id}', a Windows reserved ` +
        `device name — a shard called '${id}.json' cannot be checked out on Windows`,
    );
  }
  return id;
}

/**
 * `0010-SPINE-MEET-GREET.json`
 *
 * Throws rather than overflowing the pad width, and this is the whole reason the
 * check exists: at 1000 elements the ordinal becomes `10000`, which sorts BEFORE
 * `1010`, so sorted-filename order silently stops reproducing authored order.
 * `--check` cannot catch that — both directions use the same broken order, so
 * the round trip still closes — and the result is a re-sorted ladder nobody sees.
 *
 * A loud failure is right rather than auto-widening. Widening renumbers every
 * shard in the directory, which is a mass rename and therefore a mass merge
 * conflict — precisely what this work exists to avoid. Whoever first needs 1000
 * elements should do that deliberately, at a quiet moment, not have it happen
 * to them on an ordinary append.
 *
 * `core/spine.json` has 33 nodes so this is latent today. It is checked now
 * because HL21 migrates much larger ledgers next — `spanish/chapters.json` is
 * already at 305 and grows daily — and the convention gets copied before the
 * bug would ever be noticed.
 */
export function shardFilename(index: number, id: string): string {
  return shardFilenameFor((index + 1) * ORDINAL_STRIDE, id);
}

/**
 * The same, from an ordinal that the caller has already chosen.
 *
 * Split out from `shardFilename` because a plan may carry its OWN ordering
 * number rather than accepting `(index + 1) * 10`. `chapters.json` does: the
 * chapter number is the ordering, so chapter 7 belongs in `0007.json` and not
 * in whatever position it currently occupies in the array.
 *
 * That difference is not cosmetic. Under the index-derived stride, inserting a
 * chapter renumbers every shard after it — a mass rename, and therefore the
 * mass merge conflict this whole exercise exists to remove. Under the
 * chapter-number ordinal, appending chapter 346 writes exactly one new file and
 * touches nothing else, which is the property that lets two authors append two
 * chapters without ever meeting.
 *
 * `id` is optional. When a ledger's identity IS its number there is nothing to
 * append, and `0007-7.json` would only be saying it twice.
 */
export function shardFilenameFor(ordinal: number, id?: string): string {
  if (!Number.isInteger(ordinal) || ordinal < 0) {
    // A non-integer ordinal would pad to something like `1.5`, which sorts in a
    // place nobody predicted. Refuse rather than produce a filename whose
    // ordering is an accident.
    throw new Error(
      `shard-cli: ordinal ${JSON.stringify(ordinal)} is not a non-negative integer`,
    );
  }
  const padded = String(ordinal).padStart(ORDINAL_WIDTH, "0");
  if (padded.length > ORDINAL_WIDTH) {
    throw new Error(
      `shard-cli: ordinal ${ordinal} does not fit ${ORDINAL_WIDTH} digits — ` +
        `this ledger has outgrown the shard numbering. Widen ORDINAL_WIDTH and ` +
        `re-run --shard for every plan, in one commit, when no branch is in flight.`,
    );
  }
  return id === undefined ? `${padded}.json` : `${padded}-${id}.json`;
}

/** The one serialization, used by every writer here so the round trip closes. */
function serialize(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}

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
 * Resolve a path inside the curriculum root, or throw.
 *
 * Lifted from `modality-cli.ts`'s `safeOutput`, for the same reason and with the
 * same shape: this CLI takes a path from `--shard <path>` on the command line,
 * and `../../.github/workflows/release.yml` is a perfectly good relative path
 * and a perfectly terrible thing to overwrite. Containment is decided AFTER
 * `resolve`, never by inspecting the input string for `..` — `a/b/../../../etc`
 * contains no leading `..` and still escapes.
 */
export function safeLedgerPath(root: string, relative: string): string {
  // Non-relative rejected up front, because on Windows `path.relative` returns
  // the TARGET unchanged when the two paths sit on different roots —
  // `relative('C:\\a', 'D:\\b')` is `'D:\\b'`, which is neither `".."` nor
  // `"../"`-prefixed and so passes the containment test below.
  //
  // `isAbsolute` alone cannot express that rule, because it is PLATFORM-
  // DEPENDENT: on POSIX `D:\evil\x.json` is an ordinary relative filename, so
  // the check silently does nothing and the value falls through to a lexical
  // containment test it passes. That is not hypothetical — asserting this guard
  // unconditionally is what turned this file red on ubuntu and macos while it
  // was green on Windows.
  //
  // `assertRelativeManifestPath` applies the drive-letter and UNC patterns on
  // EVERY platform, so the rule a ledger path must satisfy is the same wherever
  // it is read, and the six generator CLIs already share it.
  assertRelativeManifestPath(
    relative,
    `shard-cli: ledger path must be relative to the curriculum root, got '${relative}'`,
  );
  const output = resolve(root, relative);
  const fromRoot = normalize(pathRelative(resolve(root), output)).replaceAll("\\", "/");
  if (
    fromRoot === "" ||
    fromRoot === ".." ||
    fromRoot.startsWith("../") ||
    !fromRoot.endsWith(".json")
  ) {
    throw new Error(`shard-cli: unsafe ledger path '${relative}'`);
  }

  // And now the part that three rounds of `lstat` guards did not cover.
  //
  // `lstat` does not follow the FINAL component — which is what every guard in
  // this feature was checking — but it follows every component BEFORE it. So
  // `core/spine.d` committed as a link is refused, and `core` committed as a
  // link sails through: `rmSync` deletes out-of-tree files and `writeFileSync`
  // overwrites them, with all four earlier guards satisfied.
  //
  // Lexical containment cannot see this; only `realpath` can. Resolving the
  // PARENT closes every intermediate component at once, and returning the
  // resolved path means the downstream operations act on the real location
  // rather than re-walking the links.
  //
  // Comparing realpath to realpath, not realpath to `root`, matters: on macOS
  // `/var` is a link to `/private/var`, so a checkout under a symlinked path
  // would otherwise fail this check for no reason.
  const realParent = realpathSync(dirname(output));
  const inside = normalize(pathRelative(realpathSync(root), realParent)).replaceAll("\\", "/");
  if (inside === ".." || inside.startsWith("../")) {
    throw new Error(
      `shard-cli: '${relative}' resolves outside the curriculum root — ` +
        `a parent directory is a symbolic link`,
    );
  }
  return join(realParent, basename(output));
}

/**
 * The `_meta.json` body: every top-level key except the sharded array.
 *
 * Built with `defineProperty` onto a null-prototype object, not `meta[key] =`.
 * Plain assignment goes through `[[Set]]`, which INVOKES the `__proto__` setter
 * — so a monolith carrying `"__proto__": {...}` swapped this object's prototype
 * and silently dropped the key from the emitted `_meta.json`. Contained (it
 * never reached `Object.prototype`), but it was both the exact sink
 * `rejectDangerousKeys` exists to close and a silent data-loss bug on top.
 *
 * `rejectDangerousKeys` now runs on the parsed monolith as well, so this is
 * belt and braces. Both, deliberately: the key check protects this function
 * from its callers, and `defineProperty` protects it from a caller that forgets.
 */
export function metaOf(document: Record<string, unknown>, listKey: string): Record<string, unknown> {
  const meta = Object.create(null) as Record<string, unknown>;
  for (const [key, value] of Object.entries(document)) {
    if (key === listKey) continue;
    Object.defineProperty(meta, key, {
      value,
      enumerable: true,
      writable: true,
      configurable: true,
    });
  }
  return meta;
}

/**
 * The shard files a monolith would produce, as a filename -> contents map.
 *
 * Pure: it computes bytes and touches no disk, so `--shard` and `--check` share
 * one definition of what the shards ARE and cannot disagree about it.
 */
export function shardContents(
  document: Record<string, unknown>,
  plan: ShardPlan,
): Map<string, string> {
  const list = document[plan.listKey];
  if (!Array.isArray(list)) {
    throw new Error(`${plan.path}: no top-level '${plan.listKey}' array to shard`);
  }
  const out = new Map<string, string>();
  out.set(META_SHARD, serialize(metaOf(document, plan.listKey)));
  const seen = new Set<string>();
  list.forEach((element, index) => {
    const id = plan.idOf === undefined
      ? undefined
      : assertSafeId(plan.idOf(element, index), plan, index);
    if (id !== undefined) {
      if (seen.has(id)) {
        // Two elements with one id would produce one file, and the second would
        // overwrite the first — a silent loss of a node, discovered later by
        // whoever notices the count is wrong.
        throw new Error(`${plan.path}: duplicate ${plan.listKey} id '${id}'`);
      }
      seen.add(id);
    }
    const ordinal = plan.ordinalOf === undefined
      ? (index + 1) * ORDINAL_STRIDE
      : plan.ordinalOf(element, index);
    const name = shardFilenameFor(ordinal, id);
    // The filename check, not just the id check, and it is the one that matters
    // for an id-less plan. `chapters.json` names its shards from the chapter
    // NUMBER, so two entries claiming chapter 12 collide on `0012.json` with no
    // duplicate id anywhere to notice — one chapter would overwrite the other
    // and the ledger would come back one entry short, silently. `--check` could
    // not catch it either: both directions would agree about the truncated set.
    if (out.has(name)) {
      throw new Error(
        `${plan.path}: ${plan.listKey}[${index}] wants shard '${name}', which is ` +
          `already taken — two elements share an ordinal${id === undefined ? "" : ` or id`}`,
      );
    }
    out.set(name, serialize(element));
  });
  return out;
}

/**
 * The monolith bytes that the shards on disk currently mean.
 *
 * `JSON.stringify` emits keys in insertion order, so a byte-exact round trip
 * needs the sharded array to land back in its original position among the
 * top-level keys. Rather than record that position as metadata inside
 * `_meta.json` — inventing a `_listAfter` key that is neither data nor
 * documentation, to solve a problem no current ledger has — the array is always
 * appended last, and `--shard` REFUSES a ledger whose array is not already last.
 *
 * `core/spine.json` is `{ version, stages, strands, strandNote, nodes }`, so
 * appending is exact for it. If a later ledger keeps its array in the middle,
 * whoever migrates it gets a clear refusal and decides what to do then, which
 * beats a silent reordering discovered by a `--check` failing for reasons
 * nobody can read.
 */
export function unshardContents(root: string, plan: ShardPlan): string {
  const monolith = safeLedgerPath(root, plan.path);
  const shards = readShards(monolith);
  if (shards === null) {
    throw new Error(`${plan.path}: no ${shardDirectoryFor(plan.path)} to rebuild from`);
  }
  const meta = shards.find((shard) => shard.name === META_SHARD);
  if (meta === undefined) throw new Error(`${plan.path}: shards are missing ${META_SHARD}`);
  return serialize({
    ...(meta.value as Record<string, unknown>),
    [plan.listKey]: shards
      .filter((shard) => shard.name !== META_SHARD)
      .map((shard) => shard.value),
  });
}

/**
 * The array must already be the last top-level key. See `unshardContents`.
 *
 * Checked at `--shard` time, where the fix is obvious and local, rather than
 * discovered at `--check` time as a diff nobody can account for.
 */
function assertListIsLast(document: Record<string, unknown>, plan: ShardPlan): void {
  const keys = Object.keys(document);
  // An absent array is a different complaint with a different fix, and
  // `shardContents` words it properly. Saying "'nodes' must be the last key (it
  // is followed by ["version"])" about a document with no `nodes` at all sends
  // the reader to reorder keys that are already in the right order.
  if (!keys.includes(plan.listKey)) return;
  if (keys.at(-1) !== plan.listKey) {
    throw new Error(
      `${plan.path}: '${plan.listKey}' must be the last top-level key to round-trip ` +
        `byte-exactly (it is followed by ${JSON.stringify(keys.slice(keys.indexOf(plan.listKey) + 1))}). ` +
        `Move it last in the monolith, or teach shard-cli to record its position.`,
    );
  }
}

/**
 * Split a monolith into its `.d/` directory.
 *
 * THE GATE HERE IS `isSharded`, NOT `existsSync`, and the difference is the
 * whole security of this function. `existsSync` FOLLOWS symlinks. This used to
 * read:
 *
 *     if (existsSync(dir)) { for (const name of listShardNames(monolith)) rmSync(...) }
 *
 * With `core/spine.d` committed as a symlink — which git tracks as a
 * first-class object, so a pull request can contain one — that deleted every
 * `*.json` in the link's target and then wrote shards into it. Pointed at
 * `../../.git` or `~/.ssh`, `npm run shard` on that branch is arbitrary file
 * deletion. `listShardNames`'s per-entry symlink check does not help: entries
 * reached THROUGH a symlinked parent report `isSymbolicLink() === false`.
 *
 * `isSharded` uses `lstatSync` and throws on a link, so the whole branch is
 * refused before anything is unlinked. The reader had this guard from the
 * start; the writer skipped it, which is the lesson — a guard that lives only
 * in the reader is a guard the writer forgets, and the writer is the dangerous
 * one.
 */
export function shardLedger(root: string, plan: ShardPlan): string[] {
  const monolith = safeLedgerPath(root, plan.path);
  // A `"removed"` ledger has no monolith to read once it has been migrated, so
  // say that plainly instead of letting `readLedgerFile` report ENOENT on a
  // file the migration deleted on purpose. Without this the second `--shard`
  // reads as "the checkout is broken" when it actually means "this is done".
  if (plan.monolith === "removed" && statIfPresent(monolith) === undefined && isSharded(monolith)) {
    throw new Error(
      `${plan.path}: already sharded into ${shardDirectoryFor(plan.path)}, and the ` +
        `monolith was removed by that migration. The shards are the source of ` +
        `truth — edit them directly; there is nothing left to split.`,
    );
  }
  // `readLedgerFile`, not a bare `JSON.parse(readFileSync(...))`. The bare form
  // skipped the symlink refusal, the dangerous-key check and the parse-error
  // scrubbing all at once — three controls lost to one convenience.
  const document = readLedgerFile<Record<string, unknown>>(monolith);
  assertListIsLast(document, plan);
  const contents = shardContents(document, plan);
  const dir = shardDirectoryFor(monolith);

  // Remove shards that the monolith no longer produces. Leaving them behind
  // would make the next `--check` fail with a node nobody can find in the
  // source — the "unexpected stale shard" case `modality-cli` already guards.
  if (isSharded(monolith)) {
    for (const name of listShardNames(monolith)) rmSync(join(dir, name));
  } else {
    // `isSharded` returned false for a reason other than "absent" only if
    // something that is not a directory is squatting on the name. Refuse it
    // rather than letting `mkdirSync(..., { recursive: true })` no-op over it.
    if (statIfPresent(dir) !== undefined) {
      throw new Error(`shard-cli: '${dir}' exists and is not a directory`);
    }
  }

  mkdirSync(dir, { recursive: true });
  const written: string[] = [];
  // The monolith goes LAST, after every shard is safely on disk, so an
  // interrupted run leaves the ledger readable in one form or the other rather
  // than in neither. Deleting first and crashing halfway through the writes
  // would lose the ledger outright.
  //
  // This is the step that actually buys the migration something. Keeping the
  // monolith as a generated artifact would leave every tranche regenerating it
  // and therefore still colliding on it — see `MonolithDisposition`.
  const removeMonolithAfter = plan.monolith === "removed";
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
  if (removeMonolithAfter && statIfPresent(monolith) !== undefined) {
    // `assertRealFile` first: `rmSync` on a symlink removes the LINK rather than
    // its target, which is harmless — but a monolith that is a symlink at all
    // means this checkout is not what it claims to be, and the shards just
    // written were derived from whatever it pointed at. Refuse loudly instead
    // of tidying the evidence away.
    assertRealFile(monolith);
    rmSync(monolith);
  }
  return written;
}

/**
 * Rebuild the monolith from the shards. Returns the bytes written.
 *
 * `assertRealFile` before the write, because `open(2)` with `O_WRONLY|O_TRUNC`
 * follows symlinks: with `core/spine.json` committed as a link, this would
 * truncate and overwrite the link's target. The file is expected to exist
 * already — it is a generated artifact under version control — so a missing one
 * is a broken checkout worth reporting rather than silently creating.
 */
export function unshardLedger(root: string, plan: ShardPlan): string {
  // Refused rather than supported for a `"removed"` ledger, and the reason is
  // that writing the file would immediately break `--check`, which asserts that
  // very file is absent. A command whose documented effect is "now CI fails" is
  // a trap, however useful it looks at the moment somebody reaches for it.
  //
  // The pull is real: `--unshard` is the obvious move when you want to eyeball a
  // whole ledger. But `unshardContents` is exported and pure, so anything that
  // genuinely needs the merged bytes can ask for them without leaving a file on
  // disk for a later `git add -A` to sweep up.
  if (plan.monolith === "removed") {
    throw new Error(
      `${plan.path}: this ledger's monolith was removed by its migration, so ` +
        `rebuilding it would only reintroduce the file that '--check' refuses. ` +
        `The shards in ${shardDirectoryFor(plan.path)} are the source of truth.`,
    );
  }
  const body = unshardContents(root, plan);
  const monolith = safeLedgerPath(root, plan.path);
  // UNCONDITIONAL. This read `if (existsSync(monolith)) assertRealFile(monolith)`
  // for one round — reintroducing, in the writer, the exact call the previous
  // round had removed from `shardLedger` for following symlinks.
  //
  // `existsSync` uses `stat`, so for a DANGLING link it returns false and the
  // guard never runs; `writeFileSync` then opens `O_CREAT` through the link and
  // creates the target. A branch committing `core/spine.json` as a link to a
  // not-yet-existing path under `$HOME` gets attacker-chosen JSON written there
  // — and `--check`'s own failure message ("Run 'npm run unshard'") is what
  // walks the maintainer into it.
  //
  // The docstring above already said a missing monolith is a broken checkout
  // worth reporting rather than silently creating. The code disagreed with the
  // comment; the comment was right.
  assertRealFile(monolith);
  writeFileSync(monolith, body, "utf8");
  return body;
}

export function runShardCli(
  args = process.argv.slice(2),
  root = defaultCurriculumRoot(),
): number {
  const usage =
    "usage: shard-cli (--shard <path> | --unshard <path> | --check [<path>])\n";
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
    ? SHARD_PLANS.filter((plan) => plan.path === requested.replaceAll("\\", "/"))
    : SHARD_PLANS;
  if (requested && plans.length === 0) {
    process.stderr.write(
      `shard-cli: '${requested}' is not a sharded ledger. Known: ${SHARD_PLANS.map((p) => p.path).join(", ")}\n`,
    );
    return 2;
  }

  let failed = false;
  for (const plan of plans) {
    if (mode === "--shard") {
      const written = shardLedger(root, plan);
      process.stdout.write(`sharded ${plan.path} into ${written.length} files\n`);
      continue;
    }
    if (mode === "--unshard") {
      unshardLedger(root, plan);
      process.stdout.write(`rebuilt ${plan.path}\n`);
      continue;
    }
    // --check: do the two representations agree?
    const monolith = safeLedgerPath(root, plan.path);
    if (!isSharded(monolith)) {
      process.stderr.write(`${plan.path}: ${shardDirectoryFor(plan.path)} is missing\n`);
      failed = true;
      continue;
    }
    const expected = unshardContents(root, plan);

    // A `"removed"` ledger is checked for the OPPOSITE thing: the monolith must
    // not be there. Resurrection is not hypothetical — it is what a bad merge
    // does when one side of the history still has the file, and it is what an
    // author does out of habit when the old filename is still in their fingers.
    //
    // Either way the result is a file that looks authoritative, is committed,
    // and is IGNORED by the loader, because `X.d/` wins. So the edits go into a
    // file nothing reads, the app serves the shards, and the two disagree in
    // silence — the precise failure mode `--check` exists to prevent, arriving
    // from the other direction.
    if (plan.monolith === "removed") {
      if (statIfPresent(monolith) !== undefined) {
        process.stderr.write(
          `${plan.path}: this ledger is sharded into ${shardDirectoryFor(plan.path)}, ` +
            `but the monolith is present again. The loader ignores it, so any edits ` +
            `in it are silently dead. Move them into the shards and delete the file.\n`,
        );
        failed = true;
      }
      // The rebuild still has to be exercised: it is what proves the shards
      // parse, carry a `_meta.json`, and fold back into one coherent document.
      // Without this the check would pass for a shard directory full of
      // unreadable files, having asserted nothing but an absence.
      JSON.parse(expected);
      const present = new Set(listShardNames(monolith));
      for (const name of shardContents(JSON.parse(expected) as Record<string, unknown>, plan).keys()) {
        if (!present.has(name)) {
          process.stderr.write(`${plan.path}: expected shard '${name}' is missing\n`);
          failed = true;
        }
      }
      continue;
    }

    // Guarded even though this only READS: `--check` is the one mode CI runs, so
    // it is the one mode a hostile branch can reach on a maintainer's runner.
    // A symlinked monolith would otherwise have its target's bytes read and
    // compared, and the comparison result reported.
    //
    // `lstatSync`, not `existsSync`, for the third time in this file: a dangling
    // link is invisible to `existsSync`, so the guard behind one never runs. Not
    // exploitable here — a dangling link just reports "stale" — but it is the
    // same anti-pattern, and this is the message that funnels maintainers into
    // `--unshard`, which is where it WAS exploitable.
    let actual: string | undefined;
    if (statIfPresent(monolith) !== undefined) {
      assertRealFile(monolith);
      actual = readFileSync(monolith, "utf8");
    }
    if (actual !== expected) {
      // BOTH directions are named, and that is a correction rather than a
      // flourish. This message used to say only "run 'npm run unshard'", which
      // was right while `core/spine.json` was the only sharded ledger: the
      // spine's monolith is purely derived, so rebuilding it from the shards is
      // always the recovery.
      //
      // It stopped being right when AUTHORED ledgers were sharded.
      // `<track>/chapters.json` is edited directly by the Python authoring
      // scripts in `learning/human-languages/data/scripts/`, which read the
      // monolith, append a chapter, and write the whole file back. An author who
      // does that and then follows this message runs `--unshard` and DISCARDS
      // the chapter they just wrote, because unshard overwrites the monolith
      // from shards that never saw it.
      //
      // A drift message that walks the reader into destroying their own work is
      // worse than no message. So say which side is newer and let them choose:
      // the tool cannot know which edit was intended, but the person who just
      // made it can.
      process.stderr.write(
        `${plan.path}: the monolith and ${shardDirectoryFor(plan.path)} disagree.\n` +
          `  If you edited the shards:   npm run unshard ${plan.path}\n` +
          `  If you edited the monolith: npm run shard ${plan.path}\n` +
          `  Then commit the result. Do not hand-merge either side.\n`,
      );
      failed = true;
    }
    // And the reverse direction: shards that the monolith could not have
    // produced. Catches a shard deleted by a bad merge, whose absence would
    // otherwise only show up as a node quietly missing from the ladder.
    const names = new Set(listShardNames(monolith));
    for (const name of shardContents(JSON.parse(expected) as Record<string, unknown>, plan).keys()) {
      if (!names.has(name)) {
        process.stderr.write(`${plan.path}: expected shard '${name}' is missing\n`);
        failed = true;
      }
    }
  }
  return failed ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runShardCli());
}
