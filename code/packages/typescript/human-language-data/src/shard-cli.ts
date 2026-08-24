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
 * How one ledger splits.
 *
 * A table rather than a `switch`, because HL21 migrates several more ledgers
 * after this one — `<track>/chapters.json`, `<track>/curriculum.json`'s four
 * top-level keys, `core/book-generation.json` — and each is the same three
 * facts: which file, which array, and what to call each element's shard.
 */
export interface ShardPlan {
  /** Ledger path relative to the curriculum root, POSIX-separated. */
  readonly path: string;
  /** The top-level array key that becomes one file per element. */
  readonly listKey: string;
  /**
   * The stable part of an element's filename, without ordinal or extension.
   *
   * Returns a string that must be a safe filename. It is validated, not
   * trusted: `idOf` reads a field out of authored JSON, and an id of `../../..`
   * or `con` would otherwise decide where this tool writes.
   */
  readonly idOf: (element: unknown, index: number) => string;
}

/** Ledgers HL21 has migrated so far. Grows one entry per follow-on PR. */
export const SHARD_PLANS: readonly ShardPlan[] = [
  {
    path: "core/spine.json",
    listKey: "nodes",
    idOf: (element) => (element as { id?: unknown }).id as string,
  },
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
  const ordinal = (index + 1) * ORDINAL_STRIDE;
  const padded = String(ordinal).padStart(ORDINAL_WIDTH, "0");
  if (padded.length > ORDINAL_WIDTH) {
    throw new Error(
      `shard-cli: ordinal ${ordinal} does not fit ${ORDINAL_WIDTH} digits — ` +
        `this ledger has outgrown the shard numbering. Widen ORDINAL_WIDTH and ` +
        `re-run --shard for every plan, in one commit, when no branch is in flight.`,
    );
  }
  return `${padded}-${id}.json`;
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
  // Absolute rejected up front, because on Windows `path.relative` returns the
  // TARGET unchanged when the two paths sit on different roots —
  // `relative('C:\\a', 'D:\\b')` is `'D:\\b'`, which is neither `".."` nor
  // `"../"`-prefixed and so passes the containment test below. Not reachable
  // today (`relative` always comes from the hardcoded `SHARD_PLANS`), but the
  // docstring promises a defence against a command-line path, and a comment
  // claiming a guard the code does not have is exactly what produced the last
  // two findings in this file.
  if (isAbsolute(relative)) {
    throw new Error(`shard-cli: ledger path must be relative to the curriculum root, got '${relative}'`);
  }
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
    const id = assertSafeId(plan.idOf(element, index), plan, index);
    if (seen.has(id)) {
      // Two elements with one id would produce one file, and the second would
      // overwrite the first — a silent loss of a node, discovered later by
      // whoever notices the count is wrong.
      throw new Error(`${plan.path}: duplicate ${plan.listKey} id '${id}'`);
    }
    seen.add(id);
    out.set(shardFilename(index, id), serialize(element));
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
 * `assertRealFile` before the write, because `open(2)` with `O_WRONLY|O_TRUNC`
 * follows symlinks: with `core/spine.json` committed as a link, this would
 * truncate and overwrite the link's target. The file is expected to exist
 * already — it is a generated artifact under version control — so a missing one
 * is a broken checkout worth reporting rather than silently creating.
 */
export function unshardLedger(root: string, plan: ShardPlan): string {
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
      // Same failure, same fix, one message — as `modality-cli` puts it.
      process.stderr.write(
        `${plan.path}: generated monolith is missing or stale. ` +
          `Run 'npm run unshard' and commit the result.\n`,
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
