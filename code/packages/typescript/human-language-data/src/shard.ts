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
  } catch {
    return false;
  }
  if (stat.isSymbolicLink()) {
    throw new Error(
      `shard: '${dir}' is a symbolic link — a shard directory must be a real ` +
        `directory beside its ledger, so that reads cannot leave the checkout`,
    );
  }
  return stat.isDirectory();
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
  const entries = readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.name.endsWith(".json") && entry.isSymbolicLink()) {
      throw new Error(
        `shard '${entry.name}' in '${dir}': is a symbolic link — ` +
          `a shard must be a real file inside its shard directory`,
      );
    }
  }
  return entries
    .filter((entry) => entry.isFile() && entry.name.endsWith(".json"))
    .map((entry) => entry.name)
    .sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
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
      throw new Error(
        `shard '${name}' in '${dir}': malformed JSON — ${describeParseFailure(cause)}`,
        { cause },
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

  // The monolith gets the same symlink refusal as the shard directory. It would
  // be an odd threat model that blocked `core/spine.d -> ~/.aws` and waved
  // through `core/spine.json -> ~/.aws/credentials`, and the round trip in
  // `shard-cli` reads this file too.
  let stat;
  try {
    stat = lstatSync(monolithPath);
  } catch (cause) {
    throw new Error(`'${monolithPath}': cannot be read — ${describe(cause)}`, { cause });
  }
  if (stat.isSymbolicLink()) {
    throw new Error(
      `'${monolithPath}' is a symbolic link — a ledger must be a real file in the tree`,
    );
  }

  let text: string;
  try {
    text = readFileSync(monolithPath, "utf8");
  } catch (cause) {
    throw new Error(`'${monolithPath}': cannot be read — ${describe(cause)}`, { cause });
  }
  let value: T;
  try {
    value = JSON.parse(text) as T;
  } catch (cause) {
    throw new Error(`'${monolithPath}': malformed JSON — ${describeParseFailure(cause)}`, {
      cause,
    });
  }
  rejectDangerousKeys(value, `'${monolithPath}'`);
  return value;
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
 * The elision does NOT try to match the quotes around the snippet, which is the
 * obvious approach and a broken one: V8 splices the bytes in RAW and unescaped,
 * so a file whose first few bytes contain a `"` mis-pairs the delimiters and
 * walks content straight through the filter. (`ab"cd AKIA…` yields
 * `Unexpected token 'a', "ab"cd AKIA"...`, and a quote-matching regex helpfully
 * elides `"ab"` and leaves ` cd AKIA` behind.) A sanitiser whose correctness
 * depends on the bytes it is sanitising is not one.
 *
 * V8 always introduces the snippet with `, "` and always runs it to the end of
 * the message, so cutting there is independent of the content. The leading
 * `Unexpected token 'A'` quotes one byte too, and is elided for the same reason.
 * Everything that carries no snippet — "Unterminated string in JSON at position
 * 22", "Expected ',' or ']' after array element" — survives untouched, which is
 * the part a reader actually needs.
 */
function describeParseFailure(cause: unknown): string {
  return describe(cause)
    .replace(/, ".*$/s, " (contents elided)")
    .replace(/^Unexpected token '.'/u, "Unexpected token '…'");
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
 * Nothing in this package deep-merges these values today, so this is defence in
 * depth for the consumer that has not been written yet. See `mergeMetaAndList`
 * for why `JSON.parse` plus spread is not itself pollution.
 */
function rejectDangerousKeys(value: unknown, where: string): void {
  if (typeof value !== "object" || value === null) return;
  for (const dangerous of ["__proto__", "constructor", "prototype"]) {
    if (Object.hasOwn(value, dangerous)) {
      throw new Error(`${where}: must not carry '${dangerous}'`);
    }
  }
}
