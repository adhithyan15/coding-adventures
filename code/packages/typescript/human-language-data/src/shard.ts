// shard.ts — read a ledger that may live as one file OR as a directory of pieces.
//
// ---------------------------------------------------------------------------
// The problem this exists to solve
// ---------------------------------------------------------------------------
//
// This curriculum is authored by many people at once. On the day this module was
// written, four tranches of Spanish content were in flight simultaneously, and
// every one of them had to append to the same handful of files:
//
//     core/book-generation.json     6,333 lines   shared by ALL 23 tracks
//     spanish/curriculum.json       7,769 lines   path + extensions + spine segments
//     spanish/chapters.json         5,888 lines   one chapter appended per tranche
//     core/spine.json                 592 lines   shared by ALL 23 tracks
//
// Appending to the end of a JSON array is the single most conflict-prone edit
// there is: git sees two branches that both changed the last few lines, and
// every pair of tranches collides. Not *sometimes* — every pair, every time.
// The merge is always mechanical and always manual, which is the worst
// combination: no judgement required, but a human has to be there anyway.
//
// The fix is the one every large repo eventually reaches for: stop storing a
// list as one file. Store it as a DIRECTORY, one file per element. Two agents
// adding two different elements now write two different filenames, and git
// merges them without noticing there was ever a question.
//
// ---------------------------------------------------------------------------
// The convention: `X.json` may instead be `X.d/`
// ---------------------------------------------------------------------------
//
// For any shardable ledger at path `X.json`, the sharded form is a sibling
// directory named `X.d`, holding one `*.json` file per piece:
//
//     core/spine.json          the monolith
//     core/spine.d/            the shards        <- read in preference
//       _meta.json
//       0010-SPINE-MEET-GREET.json
//       0020-SPINE-COURTESY-THANK.json
//
// The `.d` suffix is borrowed from the Unix `conf.d`/`rc.d` idiom, where it has
// meant exactly this — "a directory whose contents are concatenated to form the
// file this would otherwise be" — since long before anyone here was writing
// curricula. Reusing a convention readers already know beats inventing one.
//
// It also matches where this repo was already heading. PR #12443 sharded the
// generated hash ledgers, and `core/generated-book-hashes/<lang>.json`,
// `core/lesson-modality/<lang>.json` and friends are already directories of
// per-language files read in sorted order. This module generalises that shape
// rather than adding a second one beside it.
//
// ---------------------------------------------------------------------------
// Two rules that are not optional
// ---------------------------------------------------------------------------
//
// 1. SORTED ORDER, BY CODE UNIT.
//
//    `readdirSync` returns whatever the filesystem hands back. That differs
//    between APFS and ext4 and NTFS, and it shifts as files are added. If the
//    merged result depended on it, the same commit would produce different
//    bytes on two machines and every generated-artifact `--check` in this
//    package would flake. So shards are always read in sorted filename order.
//
//    Specifically `a < b` on the raw string, NOT `localeCompare`. `localeCompare`
//    consults the host's collation: under `en-US` it folds case and ignores
//    punctuation, so `_meta.json` and `0010-A.json` can swap places depending on
//    the machine's locale — the exact non-determinism sorting was supposed to
//    remove. loader.ts's own `sortedEntries` already made this choice for the
//    same reason; this is that reasoning applied one layer down.
//
//    Because order is filename order, any ledger whose element order carries
//    MEANING must encode that order in the filenames. `core/spine.json`'s nodes
//    run pre-A1 -> C2 and are not alphabetical, so its shards are named
//    `NNNN-<NODE-ID>.json` with a numeric prefix. Zero-padded, so that string
//    sort and numeric sort agree — `10` sorting before `9` is a bug that has
//    been rediscovered in every language that has ever had a `sort`.
//
// 2. FALL BACK, NEVER GUESS.
//
//    If `X.d/` does not exist, `X.json` is read exactly as before. That is what
//    lets sharding land one ledger at a time, with no flag day and no PR that
//    has to move data and change code at once. A migration that can be done in
//    small pieces will be; one that cannot, won't.
//
//    But if `X.d/` DOES exist and holds no shards, that is an error, not an
//    empty dataset. "No spine on disk" and "a spine with no nodes" are opposite
//    facts, and a loader that returns the second when it means the first hands
//    every downstream gate a clean bill of health for a corpus that is not
//    there. loader.ts makes the same call for the modality manifest, in the same
//    words: a missing manifest throws rather than returning an empty one.

import { lstatSync, readFileSync, readdirSync } from "node:fs";
import { basename, join } from "node:path";

/** The suffix that turns a monolith path into its shard directory. */
export const SHARD_DIR_SUFFIX = ".d";

/**
 * A ledger file that is present and readable but is not valid JSON.
 *
 * A distinct type rather than a plain `Error`, because two callers legitimately
 * want to TOLERATE this one failure and no others. `trackScript` falls back to
 * the built-in script map for "a malformed `track.json`"; `listExamInventories`
 * omits an unparseable file so that one bad inventory cannot stop the whole
 * plan from being computed. Both used a bare `catch {}`, which was accurate
 * when the only thing that could go wrong was a parse.
 *
 * It is not accurate any more, and this class exists because widening
 * `readLedgerFile`'s throw surface silently widened what those catches
 * absorbed. A symlinked ledger, a `__proto__` key, a file squatting at `X.d`,
 * an `EBUSY` from the Windows indexer — all of them landed in `catch {}` and
 * came back as "no declaration". For `trackScript` the consequence is concrete:
 * `parse.ts` resolves an absent script to `latin`, so a track whose script is
 * declared only in `track.json` would silently re-parse as Latin rather than
 * failing. That is exactly the "I could not tell, so I said no" fallback
 * `isAbsentErrno` was written to remove, reintroduced one layer up.
 *
 * A tagged class rather than matching on message text: the message is a
 * human-readable string that a later edit is entitled to reword, and a control
 * that breaks when somebody improves an error message is not a control.
 */
export class LedgerParseError extends Error {
  override readonly name = "LedgerParseError";
}

/** One shard, as read off disk. */
export interface Shard<T = unknown> {
  /** Bare filename, e.g. `0010-SPINE-MEET-GREET.json`. Sorted on. */
  readonly name: string;
  /** Full path on disk, for error messages that a human can act on. */
  readonly path: string;
  /** The parsed contents. */
  readonly value: T;
}

/**
 * `.../core/spine.json` -> `.../core/spine.d`.
 *
 * Throws on anything not ending in `.json`. The alternative — appending `.d` to
 * whatever it was handed — would happily produce `book.tex.d` and then spend an
 * afternoon of somebody's time explaining why it is empty.
 */
export function shardDirectoryFor(monolithPath: string): string {
  if (!monolithPath.endsWith(".json")) {
    throw new Error(`shard: '${monolithPath}' is not a .json ledger, so it has no .d directory`);
  }
  return `${monolithPath.slice(0, -".json".length)}${SHARD_DIR_SUFFIX}`;
}

/**
 * True when the sharded form of this ledger is the one on disk.
 *
 * `lstatSync`, deliberately, and not `existsSync` + `statSync`.
 *
 * `statSync` FOLLOWS symlinks, and git tracks symlinks as first-class objects —
 * so a pull request could commit `core/spine.d` as a link to `~/.docker` or to a
 * sibling checkout, and this loader would cheerfully `readdirSync` the target
 * and merge whatever `*.json` it found into the curriculum. That is not a
 * hypothetical on a corpus this many people can open a PR against, and the
 * later steps of HL21 make it worse rather than better: once `shard-cli` writes
 * the monolith back out from the shards, "reads a file outside the tree"
 * becomes "commits a file from outside the tree".
 *
 * So a symlink here is refused loudly rather than followed, and rather than
 * silently falling back to the monolith — a link named `spine.d` is never an
 * accident worth papering over, and a quiet fallback would hide it.
 *
 * The single `lstatSync` in a `try` also closes the check-then-use window that
 * `existsSync(dir) && statSync(dir)` leaves open: between those two calls the
 * directory can vanish, and the `statSync` then throws ENOENT out of a function
 * whose whole job is to answer yes or no.
 */
export function isSharded(monolithPath: string): boolean {
  const dir = shardDirectoryFor(monolithPath);
  let stat;
  try {
    stat = lstatSync(dir);
  } catch (cause) {
    const code = (cause as NodeJS.ErrnoException).code;
    if (!isAbsentErrno(code)) {
      // Never assert absence you did not establish. See `isAbsentErrno`.
      throw new Error(
        `shard: cannot tell whether '${dir}' exists (${code ?? "unknown error"}) — ` +
          `refusing to report it as absent, because that would silently fall back ` +
          `to '${basename(monolithPath)}', which since HL21 is a generated artifact ` +
          `and may be stale`,
        { cause },
      );
    }
    return false;
  }
  if (stat.isSymbolicLink()) {
    throw new Error(
      `shard: '${dir}' is a symbolic link — a shard directory must be a real ` +
        `directory beside its ledger, so that reads cannot leave the checkout`,
    );
  }
  if (!stat.isDirectory()) {
    // The second conflation in this function, and the same shape as the first.
    // `return stat.isDirectory()` reported a FILE squatting at `X.d` as "not
    // sharded", which sends the reader to restore a directory whose name is
    // already taken — and, worse, silently falls back to the monolith on the
    // way. `shardLedger` refuses this case explicitly on the WRITE side; the
    // reader let it through, which is the same reader/writer asymmetry
    // `assertRealFile` exists to record.
    throw new Error(
      `shard: '${dir}' exists and is not a directory — the shard directory's name ` +
        `is already taken, so this ledger can be neither read as shards nor ` +
        `honestly fallen back to`,
    );
  }
  return true;
}

/**
 * Does this `lstat` errno mean "there is nothing there"?
 *
 * A pure predicate rather than an inline `catch {}`, and that is the entire
 * fix. `lstat` fails for many reasons and exactly two of them mean ABSENT:
 *
 *   ENOENT   nothing at that path
 *   ENOTDIR  a parent component is a file, so the path cannot exist
 *
 * Everything else — `EACCES`/`EPERM`, `EBUSY` (on Windows the search indexer,
 * an antivirus scanner or a sync client holding the directory), `EMFILE` /
 * `ENFILE` (out of descriptors, which a 102-file parallel vitest run genuinely
 * reaches), `EIO`, `ELOOP` — means "I could not tell". Collapsing those into
 * `false` is not a conservative default; it is an assertion of a fact nobody
 * checked, and since #12690 it is the dangerous direction: "not sharded" sends
 * every reader to a monolith that is now a generated artifact.
 *
 * The sibling bug in `doc-shard.ts` (PR #12731) is what this is mirroring. There
 * it made `--check` print "BACKLOG.d is missing" and exit 1 for a directory
 * sitting on disk with 109 shards in it. `SHARD_PLANS` covers 44 ledgers, so
 * this path has 44 chances per run to do the same thing.
 *
 * Pure and exported because it is the only way to TEST it: `vi.spyOn` cannot
 * patch a `node:fs` export under ESM — the module namespace is not
 * configurable — so the classification has to be reachable without provoking a
 * real `EBUSY`. `shard-cli`'s `statIfPresent` already drew this line correctly
 * and now shares this predicate rather than keeping a second copy of it.
 */
export function isAbsentErrno(code: string | undefined): boolean {
  return code === "ENOENT" || code === "ENOTDIR";
}

/**
 * The shard filenames of `X.d/`, in the one order every machine agrees on.
 *
 * Non-`.json` entries and subdirectories are ignored rather than rejected, so a
 * stray `README.md` or an editor's `.swp` file cannot break a build. A `.json`
 * file, on the other hand, is always taken to be a shard — there is no opt-out
 * marker, because a marker is a thing to forget.
 *
 * A `*.json` entry that is a SYMLINK is refused, for the reason in `isSharded`
 * and for one more: `Dirent.isFile()` is false for a symlink, so such an entry
 * would otherwise be dropped from the merge in silence. A shard that vanishes
 * without a word is worse than one that fails, because the result still looks
 * like a complete ledger.
 */
export function listShardNames(monolithPath: string): string[] {
  const dir = shardDirectoryFor(monolithPath);
  const names: string[] = [];
  collectShardNames(dir, "", names, true);
  return names.sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
}

/**
 * Gather `*.json` under `dir`, descending ONE level into subdirectories.
 *
 * The one level exists for `<track>/curriculum.d/`, which is not one list but
 * three — `path/`, `extensions/` and `spine/` — sharing a `_meta.json`. A flat
 * directory would work and was rejected: Spanish alone would put 716 files in
 * one folder, and the people who open these directories are the authors this
 * whole exercise is for. Three named subdirectories are the difference between
 * a ledger somebody can navigate and a wall of filenames.
 *
 * Exactly one level, not arbitrary recursion. A depth limit that is a CONSTANT
 * cannot be walked into a cycle by a committed symlink, and no ledger shape in
 * HL21 needs more. Unbounded recursion here would be a directory-traversal
 * primitive driven by repository contents, which is a bigger promise than this
 * module wants to make.
 *
 * Subdirectory names are returned POSIX-joined (`path/0010-ES-PATH-001.json`)
 * so that sorting the returned list sorts sections together and orders elements
 * within each — the same single sort the flat case relies on, with no special
 * casing anywhere downstream.
 *
 * A SYMLINKED SUBDIRECTORY is refused, exactly as a symlinked shard is. The
 * per-file check would not catch it: entries reached THROUGH a symlinked parent
 * report `isSymbolicLink() === false`, so a link named `spine` pointing at
 * `~/.ssh` would have its contents merged into the curriculum and, once
 * `--unshard` runs, committed.
 */
function collectShardNames(
  dir: string,
  prefix: string,
  out: string[],
  descend: boolean,
): void {
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const label = prefix === "" ? entry.name : `${prefix}/${entry.name}`;
    if (entry.isSymbolicLink()) {
      // Refused for `*.json` for the reason in `isSharded`, and for a
      // DIRECTORY because following one would read outside the checkout.
      // Anything else that is a link is ignored, as a non-shard would be.
      if (entry.name.endsWith(".json") || descend) {
        throw new Error(
          `shard '${label}' in '${dir}': is a symbolic link — ` +
            `a shard must be a real file inside its shard directory`,
        );
      }
      continue;
    }
    if (entry.isFile() && entry.name.endsWith(".json")) {
      out.push(label);
      continue;
    }
    if (descend && entry.isDirectory()) {
      collectShardNames(join(dir, entry.name), label, out, false);
    }
  }
}

/**
 * Read every shard of `X.d/`, in sorted order, or `null` if there is no `X.d/`.
 *
 * `null` rather than `[]` for the absent case, so callers can tell "not sharded,
 * go read the monolith" from "sharded, and here is what was in it" without a
 * second `existsSync`. The empty-directory case never reaches a caller: it
 * throws here, for the reason in the header.
 */
export function readShards<T = unknown>(monolithPath: string): Shard<T>[] | null {
  if (!isSharded(monolithPath)) return null;
  const dir = shardDirectoryFor(monolithPath);
  const names = listShardNames(monolithPath);
  if (names.length === 0) {
    throw new Error(
      `shard: '${dir}' exists but holds no *.json shards — ` +
        `an empty shard directory is a broken checkout, not an empty ledger. ` +
        `Delete the directory to fall back to '${basename(monolithPath)}', or restore its shards.`,
    );
  }
  return names.map((name) => {
    const path = join(dir, name);
    let text: string;
    try {
      text = readFileSync(path, "utf8");
    } catch (cause) {
      throw new Error(`shard '${name}' in '${dir}': cannot be read — ${describe(cause)}`, { cause });
    }
    let value: T;
    try {
      value = JSON.parse(text) as T;
    } catch (cause) {
      // The filename is the whole point of this message. A bare
      // "Unexpected token } in JSON at position 412" against a merged read of 33
      // files tells the reader nothing they can open an editor on.
      throw new LedgerParseError(
        `shard '${name}' in '${dir}': malformed JSON — ${describeParseFailure(cause)}`,
        { cause: scrubbedCause(cause) },
      );
    }
    rejectDangerousKeys(value, `shard '${name}' in '${dir}'`);
    return { name, path, value };
  });
}

/**
 * The ledger, however it happens to be stored.
 *
 * `merge` is supplied by the caller because there is no one right way to fold a
 * list of shards back into a document: the spine wants `_meta` plus an ordered
 * `nodes` array, a chapters ledger wants a keyed map, a per-language ledger
 * wants a shallow object merge. Pushing that decision to the caller keeps this
 * module about FILES and leaves the SHAPE to the module that owns the shape.
 */
export function readMaybeSharded<T>(
  monolithPath: string,
  merge: (shards: Shard[]) => T,
): T {
  const shards = readShards(monolithPath);
  if (shards !== null) return merge(shards);
  return readLedgerFile<T>(monolithPath);
}

/** Stable code-point identity for one script glyph on every filesystem. */
export function scriptEntryId(glyph: unknown): string {
  if (typeof glyph !== "string" || glyph.length === 0) {
    throw new Error(`script shard entry has no non-empty glyph: ${JSON.stringify(glyph)}`);
  }
  return [...glyph]
    .map((character) => `U-${character.codePointAt(0)!.toString(16).toUpperCase()}`)
    .join("-");
}

const SCRIPT_SECTIONS = [
  { key: "letters", dir: "letters" },
  { key: "marks", dir: "marks" },
] as const;

const SCRIPT_ENTRY_NAME =
  /^(letters|marks)\/(\d{4})-(U-[0-9A-F]+(?:-U-[0-9A-F]+)*)\.json$/;

function scriptEntryGlyph(kind: "letters" | "marks", value: unknown, name: string): string {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`script shard '${name}': must contain one JSON object`);
  }
  const field = kind === "letters" ? "glyph" : "mark";
  const glyph = (value as Record<string, unknown>)[field];
  if (typeof glyph !== "string" || glyph.length === 0) {
    throw new Error(`script shard '${name}': must carry one non-empty '${field}'`);
  }
  return glyph;
}

/**
 * Reassemble a script inventory while treating each filename as an identity,
 * not merely a sorting hint. Kept in this dependency-free module so both Node
 * loaders and Vite config-time consumers execute the exact same guards.
 */
export function mergeScriptInventoryShards<T = Record<string, unknown>>(shards: Shard[]): T {
  const ordinals = new Map<string, Set<string>>([
    ["letters", new Set<string>()],
    ["marks", new Set<string>()],
  ]);
  const glyphOwners = new Map<string, string>();

  for (const shard of shards) {
    if (shard.name === META_SHARD) continue;
    const match = SCRIPT_ENTRY_NAME.exec(shard.name);
    if (match === null) {
      throw new Error(
        `script shard '${shard.name}': expected ` +
          `letters/NNNN-U-<CODEPOINT>.json or marks/NNNN-U-<CODEPOINT>.json`,
      );
    }
    const kind = match[1] as "letters" | "marks";
    const ordinal = match[2];
    const id = match[3];
    const seenOrdinals = ordinals.get(kind)!;
    if (seenOrdinals.has(ordinal)) {
      throw new Error(`script shard '${shard.name}': duplicate ${kind} ordinal '${ordinal}'`);
    }
    seenOrdinals.add(ordinal);

    const glyph = scriptEntryGlyph(kind, shard.value, shard.name);
    const expected = scriptEntryId(glyph);
    if (id !== expected) {
      throw new Error(
        `script shard '${shard.name}': filename id '${id}' does not match ` +
          `${JSON.stringify(glyph)} (${expected})`,
      );
    }
    const owner = glyphOwners.get(glyph);
    if (owner !== undefined) {
      throw new Error(
        `script shard '${shard.name}': glyph ${JSON.stringify(glyph)} is already owned by '${owner}'`,
      );
    }
    glyphOwners.set(glyph, shard.name);
  }

  return mergeSectionedShards(shards, SCRIPT_SECTIONS) as T;
}

/**
 * Refuse a path that is not a real file in the tree.
 *
 * Exported because EVERY writer needs it too, and the first version of
 * `shard-cli` proved that a guard living only inside the reader is a guard the
 * writer forgets. Its `--shard` gated on `existsSync`, which FOLLOWS symlinks —
 * so a committed `core/spine.d -> ../../.git` would have had `rmSync` delete the
 * target's contents and `writeFileSync` put shards there. Reading past a
 * symlink is a disclosure; writing past one is destruction.
 */
export function assertRealFile(path: string, what = "ledger"): void {
  let stat;
  try {
    stat = lstatSync(path);
  } catch (cause) {
    throw new Error(`'${path}': cannot be read — ${describe(cause)}`, { cause });
  }
  if (stat.isSymbolicLink()) {
    throw new Error(`'${path}' is a symbolic link — a ${what} must be a real file in the tree`);
  }
  if (!stat.isFile()) {
    throw new Error(`'${path}' is not a regular file`);
  }
}

/** How to treat a monolith whose `X.d/` is sitting right next to it. */
export interface ReadLedgerOptions {
  /**
   * Read the monolith even when `X.d/` exists.
   *
   * Exactly one kind of caller may say yes: the shard TOOLING, whose whole job
   * is to hold the two representations side by side — `--shard` re-splits the
   * monolith, `--check` byte-compares it against the rebuild. For everybody
   * else this flag is the bug, which is why it is opt-IN and named for what it
   * permits rather than for what it disables.
   */
  readonly allowShardedSibling?: boolean;
}

/**
 * Read one JSON ledger file with every guard applied.
 *
 * The single door for reading a monolith. `shard-cli` used to have its own bare
 * `JSON.parse(readFileSync(...))`, which meant it skipped the symlink refusal,
 * the dangerous-key check and the parse-error scrubbing all at once — three
 * controls lost to one convenience. Nothing outside this module should be
 * calling `readFileSync` on a ledger.
 *
 * ---------------------------------------------------------------------------
 * The fourth guard: a monolith that is no longer the source of truth
 * ---------------------------------------------------------------------------
 *
 * The three guards above all answer "is this file safe to read". This one
 * answers a different and, in day-to-day terms, likelier question: "is this
 * file still the ANSWER?"
 *
 * Since HL21, `<track>/chapters.d/`, `<track>/curriculum.d/` and
 * `core/book-generation.d/` are the source of truth, and the `.json` beside
 * them is a GENERATED artifact — kept only because a browser bundle cannot
 * `readdirSync` (§4.3). A generated artifact is current exactly as long as
 * somebody remembers to re-run the generator, and `--check` is what catches
 * them when they do not. In the window between an edit to the shards and that
 * check, the monolith holds STALE bytes that still parse, still validate, and
 * still look like a complete ledger.
 *
 * So a reader that opens the monolith directly gets a plausible wrong answer
 * with no error anywhere — the worst shape a failure can take, and the one this
 * package keeps rediscovering. `loadTrackChapters` already carries a long
 * comment about the migration silently shrinking the corpus; that was the same
 * fault arriving through the other door.
 *
 * The fix is to make the mistake impossible to make QUIETLY. If `X.d/` exists,
 * this function refuses rather than serving the derived copy, and names the
 * directory that actually holds the data. Callers that know the ledger's shape
 * use `readMaybeSharded`, which reads the shards and never reaches this line;
 * callers that do not are told to, instead of being handed stale bytes.
 *
 * Refusing rather than guessing a merge is deliberate, and it is spec §2.3's
 * rule ("fall back, never guess") pointed the other way. This function cannot
 * know whether `X.d/` folds into `{...meta, list}`, into several sections, or
 * per-language — three shapes exist today — and inventing one would produce a
 * document no generator would ever emit.
 */
export interface ReadLedgerSource<T> {
  value: T;
  text: string;
}

/** Guarded ledger parse that also retains the exact bytes for canonical checks. */
export function readLedgerFileWithSource<T = unknown>(
  path: string,
  options: ReadLedgerOptions = {},
): ReadLedgerSource<T> {
  // Only for a `.json` path, because only a `.json` path HAS an `X.d`:
  // `shardDirectoryFor` refuses anything else outright, and this guard must not
  // turn "you passed a .tex" into the confusing error that would produce.
  if (options.allowShardedSibling !== true && path.endsWith(".json") && isSharded(path)) {
    throw new Error(
      `'${path}': is sharded into '${shardDirectoryFor(path)}', which is the source ` +
        `of truth — this file is a generated artifact and may be stale. Read the ` +
        `shards through 'readMaybeSharded' with this ledger's merge, rather than ` +
        `reading the monolith directly.`,
    );
  }
  assertRealFile(path);
  let text: string;
  try {
    text = readFileSync(path, "utf8");
  } catch (cause) {
    throw new Error(`'${path}': cannot be read — ${describe(cause)}`, { cause });
  }
  let value: T;
  try {
    value = JSON.parse(text) as T;
  } catch (cause) {
    throw new LedgerParseError(`'${path}': malformed JSON — ${describeParseFailure(cause)}`, {
      cause: scrubbedCause(cause),
    });
  }
  rejectDangerousKeys(value, `'${path}'`);
  return { value, text };
}

export function readLedgerFile<T = unknown>(
  path: string,
  options: ReadLedgerOptions = {},
): T {
  return readLedgerFileWithSource<T>(path, options).value;
}

/**
 * The filename that carries a ledger's non-list keys.
 *
 * Leading underscore so it sorts away from the element shards under code-unit
 * order — `_` is 0x5F, above every digit and every uppercase letter — and so it
 * reads as "not one of the things" to anyone listing the directory.
 */
export const META_SHARD = "_meta.json";

/**
 * Fold `_meta.json` plus N element shards into `{ ...meta, [listKey]: [...] }`.
 *
 * This is the shape almost every ledger here wants, because almost every ledger
 * here is "a few document-level fields, plus one long array that everybody
 * appends to". `core/spine.json` is `{ version, stages, strands, strandNote,
 * nodes[] }`; a chapters ledger is header fields plus `chapters[]`. The array is
 * the part that causes the conflicts, so the array is the part that becomes one
 * file per element, and everything else rides along in `_meta.json` — which
 * nobody appends to, and which therefore nobody collides on.
 *
 * `_meta.json` is required. Defaulting it to `{}` would let a ledger silently
 * lose its `version` and its `stages` the first time somebody's rebase dropped
 * the file, and every consumer downstream would read a spine with no stages as
 * a spine that simply has no stages.
 */
export function mergeMetaAndList<T = unknown>(
  shards: Shard[],
  listKey: string,
): Record<string, unknown> {
  const meta = shards.find((shard) => shard.name === META_SHARD);
  if (meta === undefined) {
    throw new Error(
      `shard: no '${META_SHARD}' among ${shards.length} shard(s) — ` +
        `the ledger's document-level fields have no home`,
    );
  }
  if (typeof meta.value !== "object" || meta.value === null || Array.isArray(meta.value)) {
    throw new Error(`shard '${META_SHARD}' in '${meta.path}': must be a JSON object`);
  }
  // `__proto__` and friends are refused in `readShards`, on every shard, before
  // anything reaches here. Worth knowing why the spread below is safe anyway:
  // `JSON.parse` defines `__proto__` as an OWN data property rather than
  // invoking the setter, and object spread copies it with CreateDataProperty
  // rather than Set, so neither step can reach `Object.prototype`. That was
  // checked rather than assumed. What an unguarded parse WOULD leave is a
  // literal `__proto__` key on the merged document, dormant until some future
  // consumer folds it with `Object.assign` or a deep merge — both of which go
  // through `[[Set]]` and do hit the setter.
  if (Object.hasOwn(meta.value as object, listKey)) {
    // Otherwise the merge order silently decides whether the meta copy or the
    // shards win, and one of those is always a stale duplicate of the other.
    throw new Error(
      `shard '${META_SHARD}' in '${meta.path}': must not carry '${listKey}' — ` +
        `that array lives in the sibling shards`,
    );
  }
  return {
    ...(meta.value as Record<string, unknown>),
    [listKey]: shards
      .filter((shard) => shard.name !== META_SHARD)
      .map((shard) => shard.value as T),
  };
}

/**
 * The `_meta.json` field recording the monolith's top-level key order.
 *
 * Lives here rather than in `shard-cli` because BOTH sides need it and they
 * must not disagree: `shard-cli --unshard` writes the generated monolith from
 * it, and `loader.ts` reads the same shards into the document the app and every
 * gate actually use. Two definitions of "what these files mean" is precisely the
 * drift `--check` exists to catch, and the cheapest way to never have it is to
 * only ever write it down once.
 */
export const KEY_ORDER_FIELD = "_keys";

/**
 * The id a shard filename may carry. `shard-cli`'s `SAFE_ID`, on the READ side.
 *
 * It exists twice on purpose, and the reason is a bug this file shipped with for
 * one revision. `shard-cli` validates ids on the WRITE path, where they come out
 * of authored JSON. That is not the same trust boundary as the READ path, where
 * the id comes out of a FILENAME ON DISK — which a pull request chooses, and
 * which no amount of write-side validation constrains.
 *
 * Dropping the read-side check was a real regression while refactoring the
 * object-section merge out of `shard-cli`: a shard committed as
 * `0010-__proto__.json` produced `value["__proto__"] = …`, which invokes the
 * setter rather than creating a key. The node's realization vanished, the
 * object's prototype changed, and `Object.hasOwn` could not even see it to
 * report the collision.
 */
export const SHARD_ID_PATTERN = /^[A-Z][A-Z0-9-]*$/;

/**
 * `obj[key] = value`, but as a data property, never through a setter.
 *
 * Every key in this module comes from somewhere a pull request controls — a
 * filename for an object section, a `_keys` entry for the document order — so
 * plain assignment is not safe here even when the key looks ordinary.
 * `defineProperty` uses CreateDataProperty, which cannot reach
 * `Object.prototype` and cannot be intercepted by an inherited setter.
 *
 * `metaOf` in `shard-cli` already made this choice, after a monolith carrying
 * `"__proto__"` silently dropped a key from the emitted `_meta.json`. This is
 * the same fix on the other side of the round trip.
 */
function defineKey(target: Record<string, unknown>, key: string, value: unknown): void {
  Object.defineProperty(target, key, {
    value,
    enumerable: true,
    writable: true,
    configurable: true,
  });
}

/**
 * Refuse a key that would pollute a prototype, wherever it came from.
 *
 * `rejectDangerousKeys` checks parsed VALUES. These three keys arrive as
 * NAMES — out of a filename, or out of a `_keys` array — so they need their own
 * gate on the way in.
 */
function assertSafeKey(key: unknown, where: string): string {
  if (typeof key !== "string" || key.length === 0) {
    throw new Error(`${where}: key must be a non-empty string, got ${JSON.stringify(key)}`);
  }
  if (key === "__proto__" || key === "constructor" || key === "prototype") {
    throw new Error(`${where}: must not name '${key}'`);
  }
  return key;
}

/** One top-level key of a sectioned ledger, and where its shards live. */
export interface MergeSection {
  readonly key: string;
  /** Subdirectory under `X.d/`; omitted means the shards sit directly in it. */
  readonly dir?: string;
  /** `"object"` rebuilds `{id: value}` from the filenames. Default `"array"`. */
  readonly kind?: "array" | "object";
}

/**
 * `<track>/curriculum.d/`'s shape, in the one place both readers agree on.
 *
 * `spine` is the reason this ledger was sharded at all: every content tranche
 * in every track appends to `spine[<node>].segments`, and there are only 33
 * nodes for 23 tracks' worth of authors to collide on.
 */
export const CURRICULUM_SECTIONS: readonly MergeSection[] = [
  { key: "path", dir: "path" },
  { key: "spine", dir: "spine", kind: "object" },
  { key: "extensions", dir: "extensions" },
];

/**
 * Keys in the former grouped book-generation projection.
 *
 * Canonical data now uses chapter/output-owned subdirectories and is read by
 * `book-generation-shards.ts`. These keys and `mergeGroupedShards` remain only
 * so `shard-cli --shard core/book-generation.json` can migrate the immediately
 * preceding flat `<language>.json` layout without losing authored bytes.
 */
export const BOOK_GENERATION_GROUPED_KEYS: readonly string[] = [
  "referenceAppendices",
  "glossaries",
  "answerKeys",
  "indexes",
  "targets",
  "handwritten",
];

/**
 * Fold the legacy `core/book-generation.d/<language>.json` layout back into its
 * six arrays for one-time migration.
 *
 * Each array is rebuilt by walking the language shards in sorted filename order
 * and concatenating each one's slice. That reproduces authored order only
 * because the arrays are contiguous by language and alphabetically ordered;
 * `--check` is what keeps that true, since a hand-edit that interleaved two
 * languages would no longer round-trip and would say so.
 *
 * A language whose shard omits an array contributes nothing to it, which is how
 * `referenceAppendices` (6 languages) and `handwritten` (14) coexist with
 * `targets` (23) in one directory without empty-array noise in 17 files.
 */
export function mergeGroupedShards(
  shards: Shard[],
  groupedKeys: readonly string[],
): Record<string, unknown> {
  const metaShard = shards.find((shard) => shard.name === META_SHARD);
  if (metaShard === undefined) {
    throw new Error(
      `shard: no '${META_SHARD}' among ${shards.length} shard(s) — ` +
        `the ledger's document-level fields have no home`,
    );
  }
  // The same two guards `mergeSectionedShards` and `mergeMetaAndList` apply,
  // which this function was missing. Without the shape check, a `_meta.json`
  // holding `["a","b"]` spreads to `{0:"a",1:"b"}` and those fabricated numeric
  // keys flow into the rebuilt document instead of raising anything; a bare
  // `"abc"` does the same one character at a time.
  if (
    typeof metaShard.value !== "object" ||
    metaShard.value === null ||
    Array.isArray(metaShard.value)
  ) {
    throw new Error(`shard '${META_SHARD}' in '${metaShard.path}': must be a JSON object`);
  }
  const meta = { ...(metaShard.value as Record<string, unknown>) };
  const recorded = meta[KEY_ORDER_FIELD];
  delete meta[KEY_ORDER_FIELD];

  for (const key of groupedKeys) {
    if (Object.hasOwn(meta, key)) {
      // Otherwise the merge order silently decides whether the meta copy or the
      // shards win, and one of those is always a stale duplicate of the other.
      throw new Error(
        `shard '${META_SHARD}' in '${metaShard.path}': must not carry '${key}' — ` +
          `that array lives in the sibling shards`,
      );
    }
  }

  const assembled = new Map<string, unknown[]>();
  for (const key of groupedKeys) assembled.set(key, []);
  for (const shard of shards) {
    if (shard.name === META_SHARD) continue;
    // The legacy grouped layout is FLAT: one `<group>.json` per group, no
    // subdirectories. This also keeps the migration reader from mistaking the
    // new direct-owner projection for the old aggregate shape.
    if (shard.name.includes("/")) {
      throw new Error(
        `shard '${shard.name}' in '${metaShard.path}': a grouped ledger holds one ` +
          `<group>.json per group and no subdirectories`,
      );
    }
    const slice = shard.value as Record<string, unknown>;
    if (typeof slice !== "object" || slice === null || Array.isArray(slice)) {
      throw new Error(`shard '${shard.name}': a language shard must be a JSON object`);
    }
    for (const [key, value] of Object.entries(slice)) {
      const into = assembled.get(key);
      if (into === undefined) {
        // An unknown key would be dropped in silence otherwise, and the rebuild
        // would quietly lose whatever an author had added.
        throw new Error(
          `shard '${shard.name}': carries '${key}', which is not one of this ` +
            `ledger's grouped arrays (${groupedKeys.join(", ")})`,
        );
      }
      if (!Array.isArray(value)) {
        throw new Error(`shard '${shard.name}': '${key}' must be an array`);
      }
      into.push(...value);
    }
  }

  const order = keyOrderFrom(recorded, meta, groupedKeys, metaShard.path);
  const document: Record<string, unknown> = {};
  const emitted = new Set<string>();
  for (const key of order) {
    // `order` may come straight from `_keys` in an attacker-controlled
    // `_meta.json`, so every entry is checked before it names a property.
    assertSafeKey(key, `${KEY_ORDER_FIELD} in '${metaShard.path}'`);
    if (emitted.has(key)) {
      // A repeated key would emit the same property twice; the second write
      // wins and the document silently loses whatever the first described.
      throw new Error(
        `shard '${META_SHARD}' in '${metaShard.path}': ${KEY_ORDER_FIELD} lists ` +
          `'${key}' more than once`,
      );
    }
    emitted.add(key);
    if (assembled.has(key)) defineKey(document, key, assembled.get(key));
    else if (Object.hasOwn(meta, key)) defineKey(document, key, meta[key]);
    else {
      throw new Error(
        `shard '${META_SHARD}' in '${metaShard.path}': records key '${key}' in ` +
          `${KEY_ORDER_FIELD}, but neither the meta shard nor any group supplies it`,
      );
    }
  }
  return document;
}

/**
 * The key order to emit, from `_keys` when present — as a PERMUTATION, not a
 * subset.
 *
 * `_keys` is attacker-controlled JSON that decides which properties the rebuilt
 * document has, and the rebuild emits only the keys it names. A `_meta.json`
 * carrying `"_keys": ["path","spine","extensions"]` would therefore silently
 * drop `version`, `language` and `conceptAliases` from the document every gate
 * reads.
 *
 * Today that is caught downstream by luck rather than by design: every plan is
 * `monolith: "generated"`, so `--check` byte-compares the rebuild against a
 * committed file and a truncation shows up as a diff. A `"removed"` ledger has
 * no such file — `--check` only verifies that the expected shards exist — so
 * the truncation would be invisible. `MonolithDisposition` documents `"removed"`
 * as an intended end state, which makes this a trap laid for the next migration
 * rather than a hypothetical.
 *
 * So `_keys` must name every key exactly once and invent none.
 */
function keyOrderFrom(
  recorded: unknown,
  meta: Record<string, unknown>,
  shardedKeys: readonly string[],
  where: string,
): string[] {
  const natural = [...Object.keys(meta), ...shardedKeys];
  if (!Array.isArray(recorded)) return natural;
  const order = recorded as unknown[];
  const seen = new Set<string>();
  for (const key of order) {
    assertSafeKey(key, `${KEY_ORDER_FIELD} in '${where}'`);
    if (seen.has(key as string)) {
      throw new Error(
        `shard '${META_SHARD}' in '${where}': ${KEY_ORDER_FIELD} lists ` +
          `'${key as string}' more than once`,
      );
    }
    seen.add(key as string);
  }
  for (const key of natural) {
    if (!seen.has(key)) {
      throw new Error(
        `shard '${META_SHARD}' in '${where}': ${KEY_ORDER_FIELD} omits '${key}', ` +
          `which would drop it from the rebuilt ledger`,
      );
    }
  }
  if (seen.size !== natural.length) {
    throw new Error(
      `shard '${META_SHARD}' in '${where}': ${KEY_ORDER_FIELD} names ` +
        `${seen.size} keys but the ledger has ${natural.length}`,
    );
  }
  return order as string[];
}

/** `0010-SPINE-MEET-GREET.json` -> `SPINE-MEET-GREET`; `0007.json` -> undefined. */
function idFromShardName(name: string): string | undefined {
  const base = name.slice(name.lastIndexOf("/") + 1).replace(/\.json$/, "");
  return /^\d+-(.+)$/.exec(base)?.[1];
}

/**
 * Fold `_meta.json` plus several sections' shards back into one document.
 *
 * The general form of `mergeMetaAndList`, for a ledger that is more than one
 * list. `<track>/curriculum.json` is `{version, language, path, spine,
 * extensions}` (Spanish adds `conceptAliases`), so three of its keys are
 * sharded and they sit in the MIDDLE of the document — which is why the key
 * order has to be recorded rather than inferred. See `KEY_ORDER_FIELD`.
 *
 * Shard names arrive already sorted by code unit, and slicing them by directory
 * prefix preserves that order within each section. That is what makes sorted
 * filename order reproduce authored order — and why every section's shards
 * carry a zero-padded ordinal, INCLUDING the object-valued `spine`, whose keys
 * follow the pre-A1 -> C2 ladder in all 23 tracks and are in sorted order in
 * none of them.
 */
export function mergeSectionedShards(
  shards: Shard[],
  sections: readonly MergeSection[],
): Record<string, unknown> {
  const metaShard = shards.find((shard) => shard.name === META_SHARD);
  if (metaShard === undefined) {
    throw new Error(
      `shard: no '${META_SHARD}' among ${shards.length} shard(s) — ` +
        `the ledger's document-level fields have no home`,
    );
  }
  if (
    typeof metaShard.value !== "object" ||
    metaShard.value === null ||
    Array.isArray(metaShard.value)
  ) {
    throw new Error(`shard '${META_SHARD}' in '${metaShard.path}': must be a JSON object`);
  }
  const meta = { ...(metaShard.value as Record<string, unknown>) };
  const recorded = meta[KEY_ORDER_FIELD];
  delete meta[KEY_ORDER_FIELD];

  const assembled = new Map<string, unknown>();
  for (const section of sections) {
    if (Object.hasOwn(meta, section.key)) {
      // Otherwise the merge order silently decides whether the meta copy or the
      // shards win, and one of those is always a stale duplicate of the other.
      throw new Error(
        `shard '${META_SHARD}' in '${metaShard.path}': must not carry '${section.key}' — ` +
          `that section lives in the sibling shards`,
      );
    }
    const prefix = section.dir === undefined ? "" : `${section.dir}/`;
    const mine = shards.filter(
      (shard) =>
        shard.name !== META_SHARD &&
        (section.dir === undefined ? !shard.name.includes("/") : shard.name.startsWith(prefix)),
    );
    if ((section.kind ?? "array") === "object") {
      const value: Record<string, unknown> = {};
      for (const shard of mine) {
        const id = idFromShardName(shard.name);
        // The filename IS the key for an object section, so it is validated
        // here rather than trusted — this is the READ boundary, and the name
        // came off disk where a pull request put it. `SHARD_ID_PATTERN` already
        // excludes the three prototype-polluting names, but `assertSafeKey`
        // states that intent so a future widening of the pattern cannot quietly
        // reopen it.
        if (id === undefined || !SHARD_ID_PATTERN.test(id)) {
          // Refuse rather than invent a key: a guessed one silently relocates a
          // spine node's realization.
          throw new Error(
            `shard '${shard.name}' in '${metaShard.path}': has no usable id in its ` +
              `filename, but '${section.key}' is keyed by id — expected NNNN-<ID>.json ` +
              `with ID matching ${SHARD_ID_PATTERN.source}`,
          );
        }
        assertSafeKey(id, `shard '${shard.name}'`);
        if (Object.hasOwn(value, id)) {
          throw new Error(`shard '${shard.name}': two '${section.key}' shards claim id '${id}'`);
        }
        defineKey(value, id, shard.value);
      }
      assembled.set(section.key, value);
    } else {
      assembled.set(section.key, mine.map((shard) => shard.value));
    }
  }

  // Every shard must have been claimed by exactly one section.
  //
  // Sections take their shards by directory prefix, so a file that matches no
  // prefix — `curriculum.d/stray.json`, or anything under a directory no
  // section names — is read, parsed, and then silently dropped. That silence is
  // what let a poisoned shard reach the loader while `--check` stayed green:
  // the rebuild simply omitted it, so the generated monolith still matched the
  // committed one.
  //
  // `mergeGroupedShards` already refuses an unknown KEY inside a shard for this
  // exact reason. The same reasoning applies to an unknown shard FILE.
  const claimed = new Set<string>();
  for (const section of sections) {
    const prefix = section.dir === undefined ? "" : `${section.dir}/`;
    for (const shard of shards) {
      if (shard.name === META_SHARD) continue;
      if (section.dir === undefined ? !shard.name.includes("/") : shard.name.startsWith(prefix)) {
        claimed.add(shard.name);
      }
    }
  }
  for (const shard of shards) {
    if (shard.name === META_SHARD || claimed.has(shard.name)) continue;
    throw new Error(
      `shard '${shard.name}' in '${metaShard.path}': belongs to no section of this ` +
        `ledger (${sections.map((s) => s.dir ?? s.key).join(", ")}) — it would be ` +
        `read and then silently discarded`,
    );
  }

  const order = keyOrderFrom(
    recorded,
    meta,
    sections.map((section) => section.key),
    metaShard.path,
  );

  const document: Record<string, unknown> = {};
  const emitted = new Set<string>();
  for (const key of order) {
    // `order` may come straight from `_keys` in an attacker-controlled
    // `_meta.json`, so every entry is checked before it names a property.
    assertSafeKey(key, `${KEY_ORDER_FIELD} in '${metaShard.path}'`);
    if (emitted.has(key)) {
      // A repeated key would emit the same property twice; the second write
      // wins and the document silently loses whatever the first described.
      throw new Error(
        `shard '${META_SHARD}' in '${metaShard.path}': ${KEY_ORDER_FIELD} lists ` +
          `'${key}' more than once`,
      );
    }
    emitted.add(key);
    if (assembled.has(key)) defineKey(document, key, assembled.get(key));
    else if (Object.hasOwn(meta, key)) defineKey(document, key, meta[key]);
    else {
      throw new Error(
        `shard '${META_SHARD}' in '${metaShard.path}': records key '${key}' in ` +
          `${KEY_ORDER_FIELD}, but neither the meta shard nor any section supplies it`,
      );
    }
  }
  return document;
}

/** `unknown` from a `catch` reduced to something printable. */
function describe(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/**
 * A parse failure, with the offending file's CONTENTS held back.
 *
 * V8 quotes the bytes it choked on directly into the message — parse a file
 * beginning `AWS_SECRET_ACCESS_KEY=…` and you get `Unexpected token 'A',
 * "AWS_SECRET"... is not valid JSON`. Shards are repo files, and symlinks out of
 * the tree are refused above, so this is defence in depth rather than a live
 * leak. But `--check` runs in CI, CI logs are far more widely readable than the
 * repo, and the quoted snippet adds nothing a reader needs: the filename says
 * where to look and the position says where in it.
 *
 * This is an ALLOWLIST, and it took two wrong turns to get here — both of the
 * same kind, and worth recording so nobody takes a third.
 *
 * The first attempt matched the quotes around the snippet. But V8 splices the
 * bytes in RAW and unescaped, so a file containing a `"` mis-pairs the
 * delimiters: `ab"cd AKIA…` yields `Unexpected token 'a', "ab"cd AKIA"...`, of
 * which a quote-matching regex elides `"ab"` and leaves ` cd AKIA` behind.
 *
 * The second attempt cut from V8's `, "` separator to end of message. But V8
 * has FOUR context templates, not one, and only two of them start that way:
 *
 *     Unexpected token 'x', "CTX"...        <- elided
 *     Unexpected token 'x', ..."CTX"...     <- survived
 *
 * V8 picks the `...`-prefixed forms once the error is more than about eighteen
 * bytes into the file — which is to say, for essentially every real shard. A
 * fuzz over 5,408 malformed inputs leaked bytes in 917 of them.
 *
 * Both failures share a root cause: a sanitiser whose correctness depends on
 * the shape of the thing it is sanitising. So stop pattern-matching the message
 * and allowlist instead. `Unexpected token` is the ONLY V8 JSON family that
 * splices file bytes, and none of its four templates carries a position — so
 * there is nothing in it worth keeping, and the filename printed beside it
 * already says where to look. Every other message is byte-free and passes
 * through intact: "Unterminated string in JSON at position 22", "Expected ','
 * or ']' after array element", "Expected double-quoted property name". Those
 * carry the position, which is the part a reader actually uses.
 *
 * Matching on the family name also catches Node 18's older phrasing
 * (`Unexpected token A in JSON at position 0`), which both earlier attempts
 * missed entirely.
 */
function describeParseFailure(cause: unknown): string {
  const message = describe(cause);
  return /^Unexpected token\b/.test(message)
    ? "unexpected token (contents elided)"
    : message;
}

/**
 * The parse failure as a `cause`, holding back the same bytes the message does.
 *
 * Sanitising the message alone was not enough, and this is the fourth attempt at
 * the same control — which is itself the lesson. `new Error(msg, { cause })`
 * keeps the ORIGINAL `SyntaxError`, V8's spliced bytes and all, and almost
 * nothing prints `.message` on its own:
 *
 *   * Node's default handler for an uncaught throw prints the whole chain,
 *     including `[cause]`. `report-cli`, `gentle-ramp-cli` and `plan-cli` each
 *     wrap only `parseOptions` in a try/catch — the `loadEverything` call that
 *     reaches this code is on the next line, OUTSIDE the handler.
 *   * Vitest prints `Caused by:` under a failing test, and `BUILD` runs Vitest.
 *
 * So both of the CI channels the elision was written to protect printed the
 * bytes anyway, straight past a message that was scrupulously clean.
 *
 * Only the PARSE paths are scrubbed. Read failures keep their cause untouched —
 * an fs error carries a path and an errno, which is exactly what a reader needs
 * and contains nothing the file was hiding.
 */
function scrubbedCause(cause: unknown): unknown {
  return cause instanceof SyntaxError ? new SyntaxError(describeParseFailure(cause)) : cause;
}

/**
 * Refuse the three keys that turn a parsed document into prototype pollution.
 *
 * Applied to EVERY parsed shard and to the monolith, not just to `_meta.json`.
 * The narrower placement was a comment claiming this module was the choke point
 * for untrusted JSON while two of the three paths through it went unchecked —
 * and a guard that is documented as total but is not is worse than none, because
 * the next author reads the comment instead of the code.
 *
 * The walk is recursive because a later consumer may emit or deep-merge any
 * nested author-controlled object. A top-level-only check left values such as
 * `forms.__proto__` dormant until JavaScript object-literal evaluation invoked
 * the legacy prototype setter. Parsed JSON is acyclic, but the exported helper
 * also tolerates a caller-owned cycle rather than recursing forever.
 */
export function rejectDangerousKeys(value: unknown, where: string): void {
  const visited = new Set<object>();
  const visit = (candidate: unknown): void => {
    if (typeof candidate !== "object" || candidate === null || visited.has(candidate)) return;
    visited.add(candidate);
    for (const key of Object.keys(candidate)) {
      if (key === "__proto__" || key === "constructor" || key === "prototype") {
        throw new Error(`${where}: must not carry '${key}'`);
      }
      visit((candidate as Record<string, unknown>)[key]);
    }
  };
  visit(value);
}
