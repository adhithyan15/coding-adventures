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
  BOOK_GENERATION_GROUPED_KEYS,
  KEY_ORDER_FIELD,
  META_SHARD,
  assertRealFile,
  isAbsentErrno,
  isSharded,
  listShardNames,
  readLedgerFile,
  mergeGroupedShards,
  mergeSectionedShards,
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
export interface ShardSection {
  /** The top-level key of the ledger that this section splits up. */
  readonly key: string;
  /**
   * Subdirectory under `X.d/` to put this section's shards in.
   *
   * Omitted for a ledger with only one section, which keeps its shards directly
   * in `X.d/` — that is what `core/spine.d/` and `<track>/chapters.d/` already
   * look like on disk, and moving them would be a rename of every shard for no
   * gain.
   *
   * Present for `<track>/curriculum.d/`, which is three lists sharing one
   * `_meta.json`. Spanish alone would otherwise put 716 files in a single
   * folder, and the people who open these directories are the authors this whole
   * exercise exists for.
   */
  readonly dir?: string;
  /**
   * Whether the key holds an array of elements or an object of them.
   *
   * `"object"` exists for `curriculum.json`'s `spine`, which is a map from node
   * id to that track's realization of the node. Its KEY is the shard id and is
   * not repeated inside the value, so unsharding recovers the id from the
   * filename — which is exactly why the filename validation is not optional.
   */
  readonly kind?: "array" | "object";
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
   * `../../..` or `con` would otherwise decide where this tool writes. For an
   * `"object"` section it is required, because the id is the only way back to
   * the key.
   */
  readonly idOf?: (element: unknown, index: number) => string;
  /**
   * The number that puts this element in its place in the filename ordering.
   *
   * See `ShardPlan.ordinalOf` — same contract, per section.
   */
  readonly ordinalOf?: (element: unknown, index: number) => number;
}

/**
 * How several parallel arrays split into one file per GROUP.
 *
 * The other projection. A `ShardSection` turns one array into one file per
 * element; this turns six arrays into one file per language, holding each
 * array's slice of that language.
 *
 * `core/book-generation.json` is the ledger that needs it. Element-wise it
 * would be 1,153 files nobody wants to open individually; what an author
 * actually touches is "Spanish's slice of everything", so that is the file.
 */
export interface ShardGrouping {
  /** The top-level arrays that get partitioned. All must be arrays. */
  readonly keys: readonly string[];
  /** The group an element belongs to — its filename, minus `.json`. */
  readonly groupOf: (element: unknown, key: string, index: number) => string;
}

export interface ShardPlan {
  /** Ledger path relative to the curriculum root, POSIX-separated. */
  readonly path: string;
  /**
   * The top-level keys this ledger splits up, in the order they are written.
   *
   * Usually one. `<track>/curriculum.json` has three, which is why this is a
   * list rather than the single `listKey` it started as: `path`, `spine` and
   * `extensions` all need splitting and all three sit in the MIDDLE of the
   * document, between `language` and `conceptAliases`.
   */
  readonly sections: readonly ShardSection[];
  /**
   * Parallel arrays partitioned into one file per group, instead of per element.
   *
   * Mutually exclusive with a non-empty `sections`: a ledger is split one way or
   * the other, and a plan claiming both would have two rules for the same bytes.
   */
  readonly grouping?: ShardGrouping;
  /** What becomes of `X.json` once `X.d/` exists. See `MonolithDisposition`. */
  readonly monolith: MonolithDisposition;
}

/**
 * Group names that are safe filenames on every filesystem this repo is cloned
 * onto — the lowercase counterpart of `SAFE_ID`.
 *
 * Language slugs are lowercase (`spanish`, `marwadi`), so `SAFE_ID`'s
 * uppercase-only rule would reject every one of them. The properties that
 * matter are the same and are re-checked here rather than assumed from the
 * shape of today's data: no separators, no `..`, no leading dot, and not a
 * Windows reserved device name. `groupOf` reads a field out of authored JSON,
 * so a `language` of `../../../etc/passwd` would otherwise decide where this
 * tool writes.
 */
const SAFE_GROUP = /^[a-z][a-z0-9-]*$/;

function assertSafeGroup(group: unknown, plan: ShardPlan, key: string, index: number): string {
  if (typeof group !== "string" || !SAFE_GROUP.test(group)) {
    throw new Error(
      `${plan.path}: ${key}[${index}] has group ${JSON.stringify(group)}, ` +
        `which is not a safe shard filename (want ${SAFE_GROUP.source})`,
    );
  }
  if (WINDOWS_RESERVED.has(group.toUpperCase())) {
    throw new Error(
      `${plan.path}: ${key}[${index}] has group '${group}', a Windows reserved ` +
        `device name — a shard called '${group}.json' cannot be checked out on Windows`,
    );
  }
  return group;
}

/**
 * The default ordinal: `(index + 1) * ORDINAL_STRIDE` — 0010, 0020, …
 *
 * Spaced by ten so a new element can be inserted between two others without
 * renaming its neighbours, because a mass rename is a mass merge conflict and
 * that is the thing being removed.
 *
 * A section that already carries its own ordering number overrides this with
 * `ordinalOf`. `chapters.json` returns the chapter number, so chapter 7 lives
 * in `0007.json` for as long as it is chapter 7 — a name a human can predict
 * without consulting the directory, which matters when the point of the
 * exercise is that two authors write two different files without coordinating.
 */
function ordinalFor(section: ShardSection, element: unknown, index: number): number {
  return section.ordinalOf === undefined
    ? (index + 1) * ORDINAL_STRIDE
    : section.ordinalOf(element, index);
}

/** A section's shard path within `X.d/`, e.g. `path/0010-ES-PATH-001.json`. */
function sectionShardPath(section: ShardSection, name: string): string {
  return section.dir === undefined ? name : `${section.dir}/${name}`;
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
    sections: [
      {
        key: "chapters",
        ordinalOf: (element) => (element as { chapter?: unknown }).chapter as number,
      },
    ],
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

/**
 * The tracks whose `curriculum.json` has been sharded.
 *
 * All but `marwadi`, whose committed file writes its `lessons` arrays inline on
 * one line and so does not round-trip through `JSON.stringify(…, null, 2)`.
 * Same call, same reason, as the three absent from `CHAPTER_SHARDED_TRACKS`.
 */
const CURRICULUM_SHARDED_TRACKS: readonly string[] = [
  "arabic", "bengali", "chinese", "french", "german", "gujarati", "hindi",
  "italian", "japanese", "kannada", "latin", "malayalam", "marathi", "persian",
  "portuguese", "punjabi", "russian", "sanskrit", "spanish", "tamil", "telugu",
  "urdu",
];

/**
 * `<track>/curriculum.json` -> three sibling directories under `curriculum.d/`.
 *
 * ```text
 * curriculum.d/_meta.json                        version, language, conceptAliases, _keys
 * curriculum.d/path/0010-ES-PATH-001.json        the authored ladder
 * curriculum.d/extensions/0010-ES-EXT-001-….json the track's own additions
 * curriculum.d/spine/0010-SPINE-MEET-GREET.json  ONE FILE PER SPINE NODE
 * ```
 *
 * `spine/` is the whole point. Every content tranche in every track appends to
 * `spine[<node>].segments`, and there are only 33 nodes for 23 tracks' worth of
 * authors to collide on — the single worst conflict point in the corpus. One
 * file per node means two tranches touching two different nodes never meet.
 *
 * ALL THREE SECTIONS CARRY ORDINALS, including `spine`, and that last one
 * contradicts HL21 §5.2's reasoning. The spec argued that `spine` is keyed by
 * node id and "an object has no meaningful order", so `<NODE-ID>.json` would
 * do. That is true of JSON semantics and false of this ledger:
 *
 *   * `JSON.stringify` emits object keys in insertion order, so sorted-filename
 *     order would rewrite the key order and the monolith would not round-trip;
 *   * no track has its spine keys in sorted order — checked, all 23;
 *   * and the order is not arbitrary. Every one of the 23 tracks lists its spine
 *     keys in exactly `core/spine.d/`'s ladder order, pre-A1 -> C2. It is the
 *     same ordered ladder the spine itself is, mirrored per track.
 *
 * So the §2.2 trap applies here too, at the one place the spec said it did not.
 *
 * `path` and `extensions` need ordinals for the ordinary reason: their ids look
 * sequential and do not sort that way. Spanish diverges at index 3, where
 * authored `ES-PATH-004` meets sorted `ES-PATH-003-CASA` — a bare prefix sorts
 * before the same prefix extended.
 *
 * `conceptAliases` is left whole in `_meta.json`: 13 keys, rarely touched, and
 * nobody appends to it.
 */
function curriculumPlan(track: string): ShardPlan {
  const idOf = (element: unknown) => (element as { id?: unknown }).id as string;
  return {
    path: `${track}/curriculum.json`,
    sections: [
      { key: "path", dir: "path", idOf },
      { key: "spine", dir: "spine", kind: "object", idOf },
      { key: "extensions", dir: "extensions", idOf },
    ],
    // Generated, for the reason on `chaptersPlan`: language-ladder globs
    // `*/curriculum.json`, and a glob's key table is eager code. This ledger
    // would be WORSE than chapters — roughly 1,500 keys against a 500 kB
    // ceiling that chapters alone came within 4 kB of breaking.
    monolith: "generated",
  };
}

/**
 * `core/book-generation.json` -> `core/book-generation.d/<language>.json`.
 *
 * ## Why it is the odd one out
 *
 * The only GROUPED ledger, and the only one with no ordinal prefix. Both follow
 * from one measurement: all six arrays are already contiguous by language and
 * in the same alphabetical order, which is also sorted filename order for
 * `<language>.json`. A per-language split reproduces authored order exactly, so
 * there is nothing for an ordinal to fix.
 *
 * Element-wise it would be over a thousand files nobody opens individually.
 * What an author touches is "Spanish's slice of everything", so that is the
 * file: a Spanish tranche edits `spanish.json` and nothing else.
 *
 * ## Both blockers are cleared
 *
 * HL21 §5.3 recorded that `targets` spanned 27 runs for 23 languages and needed
 * a one-time re-sort before a per-language split could be lossless. That
 * resolved ITSELF: re-measured at 1,007 entries (the spec was written at 949) it
 * is 23 runs for 23 languages, the split runs for hindi, kannada, spanish and
 * telugu having closed as later tranches inserted into them. A test pins the
 * contiguity so an append to the END of `targets` reopens the question loudly.
 *
 * The blocker that replaced it was formatting: twelve `marwadi` entries in
 * `targets` were indented two spaces deeper than canonical, so the committed
 * file did not round-trip at all. That was re-indented in its own commit, proved
 * whitespace-only by deep-comparing the parsed structures rather than by reading
 * the diff. `grouped-shards.test.ts` now asserts the file STAYS canonical, so a
 * hand-edit that reintroduces stray indentation fails immediately.
 *
 * That exemption was specific and is worth not generalising: this is a BUILD
 * MANIFEST, a list of `(language, chapter, output, scriptSet)` triples nobody
 * reads for meaning. The other four non-round-tripping files are hand-maintained
 * curriculum data, where whitespace churn buries real edits in review — they
 * keep their monoliths.
 */
export const BOOK_GENERATION_PLAN: ShardPlan = {
  path: "core/book-generation.json",
  sections: [],
  grouping: {
    keys: BOOK_GENERATION_GROUPED_KEYS,
    groupOf: (element) => (element as { language?: unknown }).language as string,
  },
  // Generated, like the rest: `book-cli.ts` and the Python authoring scripts
  // both read this file, and keeping one derived copy current is cheaper than
  // rewriting every consumer in the commit that moves the data.
  monolith: "generated",
};

/** Ledgers HL21 has migrated so far. Grows one entry per follow-on PR. */
export const SHARD_PLANS: readonly ShardPlan[] = [
  {
    path: "core/spine.json",
    sections: [{ key: "nodes", idOf: (element) => (element as { id?: unknown }).id as string }],
    // The one ledger that keeps its monolith, and the reason is at the top of
    // this file: language-ladder's browser bundle imports it statically.
    monolith: "generated",
  },
  ...CHAPTER_SHARDED_TRACKS.map(chaptersPlan),
  ...CURRICULUM_SHARDED_TRACKS.map(curriculumPlan),
  BOOK_GENERATION_PLAN,
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

function assertSafeId(
  id: unknown,
  plan: ShardPlan,
  section: ShardSection,
  where: string,
): string {
  if (typeof id !== "string" || !SAFE_ID.test(id)) {
    throw new Error(
      `${plan.path}: ${section.key}${where} has id ${JSON.stringify(id)}, ` +
        `which is not a safe shard filename (want ${SAFE_ID.source})`,
    );
  }
  if (WINDOWS_RESERVED.has(id)) {
    throw new Error(
      `${plan.path}: ${section.key}${where} has id '${id}', a Windows reserved ` +
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
    // `isAbsentErrno`, shared with `shard.ts`, rather than the local `=== ENOENT`
    // this used to carry. This function already drew the line in the right
    // PLACE — only absence is absence, everything else rethrows — while
    // `isSharded` next door collapsed every errno into `false`. Now that the
    // reader has been fixed there is one definition of "absent" instead of two
    // that can drift, and this side additionally picks up `ENOTDIR`, which means
    // a parent component is a file and so the path cannot exist either.
    if (isAbsentErrno((error as NodeJS.ErrnoException).code)) return undefined;
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
export function metaOf(
  document: Record<string, unknown>,
  shardedKeys: readonly string[],
): Record<string, unknown> {
  const sharded = new Set(shardedKeys);
  const meta = Object.create(null) as Record<string, unknown>;
  const define = (key: string, value: unknown) =>
    Object.defineProperty(meta, key, {
      value,
      enumerable: true,
      writable: true,
      configurable: true,
    });

  // `_keys` FIRST, when it is needed at all. See `needsKeyOrder`.
  const keys = Object.keys(document);
  if (needsKeyOrder(keys, sharded)) define(KEY_ORDER_FIELD, keys);

  for (const [key, value] of Object.entries(document)) {
    if (sharded.has(key)) continue;
    define(key, value);
  }
  return meta;
}

/**
 * The `_meta.json` field that records the monolith's top-level key order.
 *
 * HL21 §2.5 chose NOT to invent this. Its reasoning was that `JSON.stringify`
 * emits keys in insertion order, so a byte-exact rebuild only needs the sharded
 * array to land back where it started — and since `core/spine.json` keeps its
 * array last, appending it last is exact. Rather than record a position no
 * ledger needed, `--shard` refused a ledger whose array was not already last,
 * and left the decision to "whoever migrates it".
 *
 * `<track>/curriculum.json` is that ledger and this is that decision. It is
 * `{version, language, path, spine, extensions}` — Spanish adds
 * `conceptAliases` — so THREE sharded keys sit in the middle of the document
 * with a non-sharded key after them. There is no ordering of "meta keys, then
 * sharded keys" that reproduces it.
 *
 * The alternatives were worse. Reordering the monolith so the sharded keys fall
 * last would rewrite 23 committed files to suit the tool. Hard-coding the key
 * order in the plan would put a fact about the DATA in the CODE, where it goes
 * stale silently the first time a track gains a key.
 *
 * So the order rides in `_meta.json`, next to the other document-level facts,
 * and it is not a conflict point: key order changes approximately never, which
 * is exactly why it was safe to leave implicit for as long as it was.
 */

/**
 * Does this document need its key order written down?
 *
 * Only when the sharded keys are not already a SUFFIX of the key order — that
 * is, when "everything else, then the sharded keys" would not reproduce the
 * original. `core/spine.json` and `<track>/chapters.json` both keep their one
 * array last, so they do not, and their `_meta.json` is byte-identical to what
 * it was before this field existed. That matters: emitting `_keys`
 * unconditionally would have rewritten all 21 already-committed shard sets to
 * add a line none of them needs.
 */
function needsKeyOrder(keys: readonly string[], sharded: ReadonlySet<string>): boolean {
  const present = keys.filter((key) => sharded.has(key));
  const suffix = keys.slice(keys.length - present.length);
  return present.length > 0 && !suffix.every((key) => sharded.has(key));
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
  if (plan.grouping !== undefined) return groupedShardContents(document, plan, plan.grouping);

  const out = new Map<string, string>();
  out.set(META_SHARD, serialize(metaOf(document, plan.sections.map((s) => s.key))));

  for (const section of plan.sections) {
    // An array section reads `[element, …]`; an object section reads
    // `{id: element, …}` and takes its id from the KEY rather than from `idOf`,
    // because that is where an object keeps it.
    const raw = document[section.key];
    const entries: { element: unknown; id: string | undefined; where: string }[] = [];

    if ((section.kind ?? "array") === "object") {
      if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
        throw new Error(`${plan.path}: no top-level '${section.key}' object to shard`);
      }
      let index = 0;
      for (const [key, element] of Object.entries(raw as Record<string, unknown>)) {
        entries.push({
          element,
          id: assertSafeId(key, plan, section, `['${key}']`),
          where: `['${key}']`,
        });
        index += 1;
      }
      void index;
    } else {
      if (!Array.isArray(raw)) {
        throw new Error(`${plan.path}: no top-level '${section.key}' array to shard`);
      }
      raw.forEach((element, index) => {
        entries.push({
          element,
          id: section.idOf === undefined
            ? undefined
            : assertSafeId(section.idOf(element, index), plan, section, `[${index}]`),
          where: `[${index}]`,
        });
      });
    }

    const seen = new Set<string>();
    entries.forEach(({ element, id, where }, index) => {
      if (id !== undefined) {
        if (seen.has(id)) {
          // Two elements with one id would produce one file, and the second
          // would overwrite the first — a silent loss of an element, discovered
          // later by whoever notices the count is wrong.
          throw new Error(`${plan.path}: duplicate ${section.key} id '${id}'`);
        }
        seen.add(id);
      }
      const name = sectionShardPath(
        section,
        shardFilenameFor(ordinalFor(section, element, index), id),
      );
      // The filename check, not just the id check, and it is the one that
      // matters for an id-less section. `chapters.json` names its shards from
      // the chapter NUMBER, so two entries claiming chapter 12 collide on
      // `0012.json` with no duplicate id anywhere to notice — one chapter would
      // overwrite the other and the ledger would come back one entry short,
      // silently. `--check` could not catch it either: both directions would
      // agree about the truncated set.
      if (out.has(name)) {
        throw new Error(
          `${plan.path}: ${section.key}${where} wants shard '${name}', which is ` +
            `already taken — two elements share an ordinal${id === undefined ? "" : ` or id`}`,
        );
      }
      out.set(name, serialize(element));
    });
  }
  return out;
}

/**
 * One file per group, each holding that group's slice of every grouped array.
 *
 * The order within each per-language file is the order the elements appeared in
 * the monolith, and the order BETWEEN files is sorted filename order. Those two
 * facts are what make the rebuild reproduce authored order — but only because
 * every one of the six arrays is already contiguous by language and in the same
 * alphabetical order. That was measured across all six rather than assumed.
 *
 * If a future edit interleaves two languages in one array, this does not
 * silently reorder anything: the rebuild stops matching the monolith and
 * `--check` says so. The failure is loud, which is the property worth having,
 * because "the arrays happen to be grouped" is not something the file format
 * enforces.
 *
 * A group contributes a key only when it has entries for it, so the 17
 * languages with no `referenceAppendices` do not each carry an empty array.
 */
function groupedShardContents(
  document: Record<string, unknown>,
  plan: ShardPlan,
  grouping: ShardGrouping,
): Map<string, string> {
  const slices = new Map<string, Map<string, unknown[]>>();
  for (const key of grouping.keys) {
    const list = document[key];
    if (!Array.isArray(list)) {
      throw new Error(`${plan.path}: no top-level '${key}' array to shard`);
    }
    list.forEach((element, index) => {
      const group = assertSafeGroup(grouping.groupOf(element, key, index), plan, key, index);
      let byKey = slices.get(group);
      if (byKey === undefined) {
        byKey = new Map();
        slices.set(group, byKey);
      }
      const into = byKey.get(key);
      if (into === undefined) byKey.set(key, [element]);
      else into.push(element);
    });
  }

  const out = new Map<string, string>();
  out.set(META_SHARD, serialize(metaOf(document, grouping.keys)));
  // Sorted, so the written order matches the read order and `--shard` is
  // idempotent down to the byte.
  for (const group of [...slices.keys()].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0))) {
    const byKey = slices.get(group)!;
    // Keys in the ledger's own order, not in first-seen order, so two runs over
    // the same data always emit the same bytes.
    const body: Record<string, unknown> = {};
    for (const key of grouping.keys) {
      const value = byKey.get(key);
      if (value !== undefined) body[key] = value;
    }
    out.set(`${group}.json`, serialize(body));
  }
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
  // ONE definition of what these files mean, shared with `loader.ts`. If the
  // generated monolith and the document the app loads were assembled by two
  // different pieces of code, they could disagree — and `--check` compares the
  // monolith against THIS function, so it would report agreement while the app
  // read something else.
  return serialize(
    plan.grouping === undefined
      ? mergeSectionedShards(shards, plan.sections)
      : mergeGroupedShards(shards, plan.grouping.keys),
  );
}

/**
 * Refuse a ledger that cannot round-trip, before anything is written.
 *
 * This USED to be `assertListIsLast`: HL21 §2.5 required the sharded array to
 * be the last top-level key, because the rebuild appended it last and there was
 * no way to say otherwise. `<track>/curriculum.json` is the ledger that
 * refusal was waiting for, and `_keys` is the answer — so the position check is
 * gone and what remains is the one thing `_keys` cannot rescue.
 *
 * A ledger whose `_meta.json` would carry a literal `_keys` of its own. The
 * rebuild reads that field as the recorded key order and deletes it, so a
 * document that genuinely has a top-level `_keys` would lose it silently and
 * come back with its keys in whatever order the field happened to describe. No
 * ledger has one today; the check costs nothing and closes the collision before
 * it can be discovered as a corrupted rebuild.
 */
function assertShardable(document: Record<string, unknown>, plan: ShardPlan): void {
  if (Object.hasOwn(document, KEY_ORDER_FIELD)) {
    throw new Error(
      `${plan.path}: has a top-level '${KEY_ORDER_FIELD}' key, which shard-cli ` +
        `reserves to record the document's key order. Rename it before sharding.`,
    );
  }
  if (plan.grouping !== undefined && plan.sections.length > 0) {
    // Two rules for the same bytes. Whichever ran second would overwrite the
    // first's shards, and `--check` would compare against only one of them.
    throw new Error(
      `${plan.path}: a plan may split by element OR by group, not both`,
    );
  }
  // Two sections may not share a key or a directory.
  //
  // A shared DIRECTORY is the dangerous one: shards are claimed by directory
  // prefix, so both sections would consume the same files and the ledger would
  // come back with the same elements under two keys. Nothing downstream could
  // tell that from a ledger that genuinely repeats itself, and `--check` would
  // agree with itself about the duplicate.
  //
  // Cheap to state here, where a plan is written once, rather than to detect at
  // merge time where it looks like data corruption.
  const seenKeys = new Set<string>();
  const seenDirs = new Set<string>();
  for (const section of plan.sections) {
    if (seenKeys.has(section.key)) {
      throw new Error(`${plan.path}: two sections both shard '${section.key}'`);
    }
    seenKeys.add(section.key);
    const dir = section.dir ?? "";
    if (seenDirs.has(dir)) {
      throw new Error(
        `${plan.path}: two sections share the shard directory ` +
          `'${section.dir ?? "(top level)"}', so each would claim the other's shards`,
      );
    }
    seenDirs.add(dir);
  }

  const keys = plan.grouping?.keys ?? plan.sections.map((section) => section.key);
  for (const key of keys) {
    if (!Object.hasOwn(document, key)) {
      throw new Error(`${plan.path}: no top-level '${key}' to shard`);
    }
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
  //
  // `allowShardedSibling`, and this is the ONE call in the package entitled to
  // it. `readLedgerFile` otherwise refuses a monolith whose `X.d/` exists,
  // because to every other reader that file is a generated artifact that may be
  // stale. Re-sharding is the exception by definition: `--shard` exists to
  // rebuild `X.d/` FROM the monolith, so "the shards already exist" is the
  // ordinary case here rather than the error. The `"removed"` branch above has
  // already ruled out the one shape where reading it WOULD be wrong — a
  // migrated ledger whose monolith is gone on purpose.
  const document = readLedgerFile<Record<string, unknown>>(monolith, {
    allowShardedSibling: true,
  });
  assertShardable(document, plan);
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
  // One directory per section that asked for one. Created up front rather than
  // lazily inside the write loop so that a plan naming a section directory it
  // cannot create fails before any shard is written, not halfway through.
  for (const section of plan.sections) {
    if (section.dir !== undefined) mkdirSync(join(dir, section.dir), { recursive: true });
  }
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
