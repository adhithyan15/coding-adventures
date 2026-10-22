// ---------------------------------------------------------------------------
// lesson-cache.ts — parse the corpus once per test run, not once per test file
// ---------------------------------------------------------------------------
//
// The problem this exists for
// ---------------------------
// Parsing every lesson is the single most expensive thing this package does.
// Measured on main in October 2026, with 38,088 lessons (98 MB of Markdown) in
// 23 tracks, `loadEverything()` took 13.4 s and 9.2 s of that was
// `loadLessons()`:
//
//     reading 38,088 files off disk          0.7 s
//     frontmatter (`splitFrontmatter`)        1.2 s
//     body blocks (`parseBodyBlocks`)         3.5 s
//     `sourceHash` (`canonicalLessonHash`)    3.8 s
//
// None of that is slow for ONE caller. The CLI pays it once and is done. The
// test suite pays it hundreds of times, because Vitest gives every test file a
// fresh module graph — that isolation is the point of a test runner — so the
// 200-odd files that read the real corpus each re-parse all 98 MB of it from
// scratch. On a shared CI runner, three or four of those parses at once are
// what pushed `beforeAll` hooks and corpus-wide cases past their budgets.
// Raising the budgets would only move the next timeout one content PR later
// (see the comment in `vitest.config.ts`); removing the repetition does not.
//
// The idea
// --------
// `parseLesson(source, language, script)` is a PURE function: the same three
// inputs always produce the same `ParsedLesson`. So a track's parsed lessons
// are a function of exactly:
//
//     the bytes of every `<track>/lessons/*.md`, in sorted filename order
//     the track id and its resolved script
//     the code that does the parsing
//
// Hash all of that and you have a name for the answer. If a file with that
// name already exists, the answer has already been computed — by this run's
// global setup, by another worker, or by yesterday's run — and reading it back
// is several times cheaper than computing it again. That is the whole design:
// a CONTENT-ADDRESSED cache. It never asks "is this entry still fresh?",
// because an entry cannot go stale: edit one byte of one lesson and the hash
// changes, so the lookup is for a different name that does not exist yet.
//
//     key = SHA-256( format version, V8 version, package source digest,
//                    track id, script,
//                    for each lesson file: name, byte length, bytes )
//
// Why the bytes and not (path, size, mtime)
// -----------------------------------------
// A stat-based key is cheaper (0.2 s instead of 0.6 s for the whole corpus)
// and is what most build tools use. It is also wrong in exactly the cases this
// package cannot afford. Git calls it the "racy clean" problem: a file edited
// twice within one timestamp tick, keeping its size, looks unchanged. A tool
// that preserves mtimes (`cp -p`, `rsync -t`, some archive extractions) does
// the same on purpose. Either way a stale parse would be served, and the tests
// that compare committed generated files against freshly parsed lessons would
// compare against the OLD lessons and pass. Hashing the content costs 0.4 s
// more per full-corpus read and closes that hole completely, so it is the
// right trade. The files have to be read for parsing on a miss anyway.
//
// Why the package's own source is in the key
// -------------------------------------------
// The parse is a function of the CODE as well as the data. Change
// `classifyBlock` and every cached entry describes a parser that no longer
// exists. Rather than guess which modules `parseLesson` reaches, the key folds
// in a digest of every file in this module's own directory (`src/` under
// Vitest, `dist/` when compiled). Any edit anywhere in the package invalidates
// the whole cache. That costs one re-parse after a code change, which is
// nothing beside serving a parse from yesterday's parser.
//
// The V8 version is in the key because `v8.serialize`'s wire format belongs to
// V8. A newer V8 reads older payloads, but there is no reason to rely on it.
//
// Who uses it: TESTS ONLY
// -----------------------
// The cache is OFF unless something calls `configureLessonCache()`, and only
// the Vitest setup files do (`tests/setup/`). The CLIs and generators —
// `generate:books`, `check:modality` and the rest — never turn it on. They
// parse once per process, so the cache could save them nothing, and a `--check`
// gate is precisely the place where "read what is on disk, every time" should
// be the only code path. Keeping them off it means no generator correctness
// argument ever has to mention this file.
//
// It is also off for every curriculum root other than the configured one. Tests
// build hundreds of tiny fixture corpora in temp directories; caching those
// would gain nothing and would fill the cache with entries nobody reads again.
//
// The security model
// ------------------
// `v8.deserialize` turns bytes into objects. It cannot run code — the format
// carries only structured-clone data, and it defines properties rather than
// assigning them, so a key named `__proto__` is just a key — but it WILL hand
// back whatever data those bytes describe. Someone who can plant a cache file
// can therefore choose what "the corpus" says to every test. So an entry is
// only ever read when ALL of these hold:
//
//   1. The suite's directory is a fixed path computed from this module's own
//      location — `<package>/node_modules/.cache/human-language-data/lessons`.
//      No environment variable, argument or file can redirect it; only code
//      calling `configureLessonCache` names a directory, and the only such
//      code is the setup files and this module's own tests (which use temp
//      directories).
//   2. It is a real directory — not a symlink or junction itself, and its
//      real path is exactly its resolved parent plus its own name — owned by
//      the current user, with no group or other permission bits (mode 0700).
//      Anything else disables the cache for the process.
//   3. The entry's name matches `ENTRY_NAME`, it is not a link (`lstat`, plus
//      O_NOFOLLOW where the platform has it), its real path is exactly
//      `<directory>/<name>`, and the file that is OPENED is the same file
//      (device and inode) that was checked, so a link swapped in between the
//      check and the open is refused.
//   4. The opened file is a regular file owned by the current user with no
//      group or other permission bits (mode 0600) — that is, a file only this
//      user could have created, which is the closest a process can come to
//      "a file I wrote myself".
//   5. It starts with this format's magic bytes and carries a SHA-256 of its
//      payload that matches. A truncated, partially written or bit-flipped
//      file fails here and is treated as a miss, never as data.
//
// A failed check is never an error. The cache is an optimisation, so every
// refusal falls back to parsing from disk, which is always correct.
//
// Windows
// -------
// Windows has no `process.getuid` and no mode bits — permissions are ACLs — so
// checks 2 and 4 cannot be expressed there. Windows CI needs the cache as
// much as Linux does (the same corpus-wide tests time out on `windows-2025`),
// so instead of switching it off, Windows keeps every check that CAN be made
// and narrows where the cache may live:
//
//   * The cache is enabled ONLY for the fixed default directory,
//     `defaultLessonCacheDirectory()`. Any other directory — a temp dir, a
//     configured path, anything a caller passes — leaves the cache off.
//   * The directory and every entry must not be links. Node's `lstat` reports
//     a junction as a symbolic link, and the real-path equalities in checks 2
//     and 3 catch any reparse point `lstat` does not label: a redirected path
//     resolves somewhere other than where it claims to be.
//   * Entry names, the magic bytes and the SHA-256 are checked exactly as on
//     POSIX, and only names matching `ENTRY_NAME` or `TEMP_NAME` inside the
//     directory are ever deleted.
//
// Why dropping ownership is acceptable there: the default directory is inside
// this package's own `node_modules`. Anyone who can write a file into it can
// equally replace `node_modules/vitest` — code the test process runs — so a
// planted cache entry grants nothing that write access did not already give.
// The ownership checks are defence in depth on POSIX, where they are free; on
// Windows they are skipped, and only they are.
//
// The platform is a parameter (`CachePlatform`), not a `process.platform`
// test buried in each function, so the Windows rules are exercised on Linux
// too: `tests/lesson-cache.test.ts` runs the directory restriction, link
// refusal, digest refusal and rename-collision cases under a simulated win32.
//
// Concurrency
// -----------
// Several Vitest workers can miss the same entry at once and all write it.
// Each writes its own temp file — created with O_EXCL, so two writers can
// never share one — and then `rename`s it over the entry's name. `rename` is
// atomic on POSIX: a reader opening the name sees either the old complete file
// or the new complete file, never a mixture. Because the name is a hash of the
// content, every racing writer is writing the same answer, so it does not
// matter which rename lands last. On Windows a rename over an entry another
// process has open can fail (EPERM, EACCES, EBUSY); since that entry already
// holds the same answer, the loser just deletes its temp file and moves on.
//
// Disk usage
// ----------
// One entry per track: about 320 MB for today's whole corpus. Whenever an entry
// is written, the older entries for the same track are deleted, so an edited
// track replaces its entry rather than accumulating them. `npm ci` deletes
// `node_modules`, and the cache with it.
//
// One store, two caches
// ---------------------
// Everything below the key — the private directory, the entry format, the
// guarded read, the atomic write, the sweep — is `openEntryStore`, and it is
// not specific to lessons. `src/narration-cache.ts` opens the same store on a
// sibling directory (`.../human-language-data/narration`) to keep each track's
// narrated chapters, keyed by THIS cache's key for the track's lessons. So
// every rule above protects both, and neither has a file-handling path of its
// own that could drift from the other's.

import { createHash, randomBytes } from "node:crypto";
import {
  type BigIntStats,
  closeSync,
  constants as fsConstants,
  existsSync,
  fstatSync,
  lstatSync,
  mkdirSync,
  openSync,
  readFileSync,
  readdirSync,
  realpathSync,
  renameSync,
  unlinkSync,
  writeSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { deserialize, serialize } from "node:v8";
// Type-only, and that is load-bearing: this module must not pull the parser in
// at runtime. The concurrency test runs it in child processes straight from
// source, and a runtime import of `parse.js` would not resolve there.
import type { ParsedLesson } from "./parse.js";

/**
 * Bump when the entry layout or the meaning of a key changes.
 *
 * It is the first thing hashed into every key, so bumping it makes every
 * existing entry unreachable at once — the same mechanism as a lesson edit.
 */
export const LESSON_CACHE_FORMAT = "hl-lesson-cache/1";

/**
 * The first bytes of every entry. Followed by a 32-byte SHA-256 of the payload,
 * then the payload itself (`v8.serialize` of the lesson array):
 *
 *     +-----------------+---------------------+--------------------------+
 *     | MAGIC (8 bytes) | SHA-256 (32 bytes)  | v8.serialize(lessons)    |
 *     +-----------------+---------------------+--------------------------+
 *
 * The magic catches a file that is not an entry at all; the digest catches one
 * that is an entry but not the one that was written.
 */
const MAGIC = Buffer.from("HLLC\u0001\r\n\u0000", "latin1");
const DIGEST_BYTES = 32;
const HEADER_BYTES = MAGIC.length + DIGEST_BYTES;

/** Permission bits for "group or other may do anything". Must all be zero. */
const GROUP_OR_OTHER = 0o077;

/** An entry's basename: `<track>-<64 hex>.v8`. Nothing else is ever opened. */
const ENTRY_NAME = /^([a-z][a-z0-9-]*)-([0-9a-f]{64})\.v8$/;

/** A temp file left behind by a writer that died between create and rename. */
const TEMP_NAME = /^[a-z][a-z0-9-]*-[0-9a-f]{64}\.v8\.[0-9]+\.[0-9a-f]{16}\.tmp$/;

/** How old an orphaned temp file must be before it is swept. */
const ORPHAN_AGE_MS = 60 * 60 * 1000;

/** One lesson file as the key sees it: its basename and its raw bytes. */
export interface LessonSourceFile {
  name: string;
  bytes: Buffer;
}

/** What one track's key is computed from. */
export interface LessonCacheKeyInput {
  language: string;
  script: string | undefined;
  files: readonly LessonSourceFile[];
}

/**
 * A validated, private directory of entries, each one an array of `T`.
 *
 * This is everything below the key: the directory checks, the entry format,
 * the guarded read and the atomic write. `LessonCache` is this plus a key over
 * lesson files; `src/narration-cache.ts` is this plus a key over a track's
 * lesson key, width and titles. Neither can open a file any other way, so
 * every rule in the header above holds for both by construction rather than
 * by two copies agreeing.
 * Obtain one from `openEntryStore`; there is no way to build it around an
 * unchecked path.
 */
export interface EntryStore<T> {
  /** The resolved directory every entry lives in. */
  readonly directory: string;
  /** The package source digest, computed once when the store was opened. */
  readonly sourceDigest: string;
  /** The values stored under `key` for `track`, or `undefined` for any miss. */
  read(track: string, key: string): T[] | undefined;
  /** Store `values` under `key`. Never throws; a failed write is just a miss later. */
  write(track: string, key: string, values: readonly T[]): LessonCacheWrite;
}

/**
 * A validated, private cache directory. Obtain one from `openLessonCache`; there
 * is deliberately no way to construct it around an unchecked path.
 */
export interface LessonCache {
  /** The resolved directory every entry lives in. */
  readonly directory: string;
  /** The content address for one track's lessons. */
  key(input: LessonCacheKeyInput): string;
  /** The parsed lessons stored under `key`, or `undefined` for any miss. */
  read(language: string, key: string): ParsedLesson[] | undefined;
  /** Store `lessons` under `key`. Never throws; a failed write is just a miss later. */
  write(language: string, key: string, lessons: readonly ParsedLesson[]): LessonCacheWrite;
}

/**
 * What a write did. Callers ignore it — every outcome is safe — but the tests
 * use it to tell a Windows rename collision ("another writer already put the
 * same entry there") apart from a write that failed outright.
 */
export type LessonCacheWrite = "written" | "already-written" | "not-written";

// ---------------------------------------------------------------------------
// The package source digest
// ---------------------------------------------------------------------------

/**
 * SHA-256 over every regular file under `directory`, in sorted path order.
 *
 * Path and length go in ahead of each file's bytes so that moving bytes from
 * one file to the next cannot produce the same digest — `a.ts = "xy"` plus
 * `b.ts = ""` must not hash like `a.ts = "x"` plus `b.ts = "y"`.
 */
export function directoryDigest(directory: string): string {
  const hash = createHash("sha256");
  const walk = (relative: string): void => {
    const here = join(directory, relative);
    const entries = readdirSync(here, { withFileTypes: true }).sort((a, b) =>
      a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
    );
    for (const entry of entries) {
      const child = relative === "" ? entry.name : `${relative}/${entry.name}`;
      if (entry.isDirectory()) {
        walk(child);
      } else if (entry.isFile() || entry.isSymbolicLink()) {
        // A linked source file is hashed by what it POINTS AT, because that is
        // the code that runs. Skipping it would let an edit to the target leave
        // the digest — and every cached parse — unchanged. (Linked directories
        // are not followed: `isDirectory()` is false for a link, which also
        // rules out a cycle. `src/` holds neither today.) A dangling link
        // throws, and `openLessonCache`'s caller sees the cache stay off.
        const bytes = readFileSync(join(directory, child));
        hash.update(`${child}\u0000${bytes.length}\u0000`);
        hash.update(bytes);
      }
    }
  };
  walk("");
  return hash.digest("hex");
}

/** Computed once per process: the code cannot change under a running test. */
let ownSourceDigest: string | undefined;

function packageSourceDigest(): string {
  ownSourceDigest ??= directoryDigest(dirname(fileURLToPath(import.meta.url)));
  return ownSourceDigest;
}

/**
 * The content address for one track.
 *
 * Every variable-length field is written with its length first, so no two
 * different inputs can serialise to the same byte stream — without that, a file
 * named `a` holding `bc` and a file named `ab` holding `c` would collide.
 */
export function lessonCacheKey(input: LessonCacheKeyInput, sourceDigest: string): string {
  const hash = createHash("sha256");
  const field = (value: string): void => {
    const bytes = Buffer.from(value, "utf8");
    hash.update(`${bytes.length}:`);
    hash.update(bytes);
  };
  field(LESSON_CACHE_FORMAT);
  field(process.versions.v8);
  // The parse is not locale-free: concept tags and the frontmatter keys inside
  // `sourceHash` are ordered with `localeCompare`, which follows the default ICU
  // locale (under da_DK "aa" sorts after "z"). A parse made under one locale must
  // not be served to a run under another, so the ICU build and the resolved
  // default collation locale are part of the address.
  field(process.versions.icu ?? "no-icu");
  field(new Intl.Collator().resolvedOptions().locale);
  field(sourceDigest);
  field(input.language);
  // `undefined` and `""` are different answers from `trackScript` — "no
  // declaration" and "declared as empty" — so they must not share a key.
  field(input.script === undefined ? "\u0000none" : `=${input.script}`);
  field(String(input.files.length));
  for (const file of input.files) {
    field(file.name);
    hash.update(`${file.bytes.length}:`);
    hash.update(file.bytes);
  }
  return hash.digest("hex");
}

// ---------------------------------------------------------------------------
// The private directory
// ---------------------------------------------------------------------------

/**
 * Is `mode`/`uid` something only the current user could have made?
 *
 * One predicate for both the directory and every entry, so the two checks
 * cannot drift apart the way four hand-written copies of `TRACK_ID` once did
 * in `loader.ts`.
 */
export function privatelyOwned(
  stats: { uid: number | bigint; mode: number | bigint },
  uid: number,
): boolean {
  // `Number()` because entry stats are BigInt (see `sameFile`); a uid and a
  // mode both fit comfortably in a double, so nothing is lost converting them.
  return Number(stats.uid) === uid && (Number(stats.mode) & GROUP_OR_OTHER) === 0;
}

/**
 * The platform rules a cache runs under. Production code takes
 * `hostCachePlatform()`; tests construct the other one to exercise it.
 *
 *     posix   owner + mode checks on the directory and every entry; any
 *             directory a caller names, if it proves private
 *     win32   no owner or mode checks (ACLs, not mode bits); ONLY the fixed
 *             `defaultDirectory`
 *
 * `noFollow` is the O_NOFOLLOW flag where the platform has one and 0 where it
 * does not (Windows); the `lstat` + same-file checks run either way, so a
 * platform without the flag loses no protection.
 *
 * `open`, `fstat` and `rename` are seams. Production passes Node's own; the
 * tests pass wrappers that do what another process could do at the worst
 * moment — swap the entry between the look and the open, report a zero file
 * id, collide on rename — because none of those can be produced on demand
 * any other way.
 */
interface CachePlatformOps {
  noFollow: number;
  open: (path: string, flags: number, mode?: number) => number;
  /** Always BigInt stats: an NTFS file id is 64 bits and a `number` rounds it. */
  fstat: (fd: number) => BigIntStats;
  rename: (from: string, to: string) => void;
}

export type CachePlatform =
  | ({ kind: "posix"; uid: number } & CachePlatformOps)
  | ({ kind: "win32"; defaultDirectory: string } & CachePlatformOps);

/**
 * The platform this process runs on, or `undefined` where neither rule set applies.
 *
 * `defaultDirectory` is the ONE directory Windows will accept. It is a
 * parameter only so that the narration cache can name its own fixed sibling
 * (`defaultNarrationCacheDirectory()`); both defaults are computed from this
 * package's location, and nothing passes anything else outside the tests.
 */
export function hostCachePlatform(
  defaultDirectory: string = defaultLessonCacheDirectory(),
): CachePlatform | undefined {
  // `?? 0`: Node exposes no O_NOFOLLOW on Windows, and OR-ing `undefined` into
  // a flag word would silently become 0 anyway — this just says so.
  const ops: CachePlatformOps = {
    noFollow: fsConstants.O_NOFOLLOW ?? 0,
    open: openSync,
    fstat: (fd) => fstatSync(fd, { bigint: true }),
    rename: renameSync,
  };
  if (typeof process.getuid === "function") {
    return { kind: "posix", uid: process.getuid(), ...ops };
  }
  if (process.platform === "win32") {
    return { kind: "win32", defaultDirectory, ...ops };
  }
  return undefined;
}

/** POSIX: owned by us and private. Windows: nothing to check (see the header). */
function ownedHere(
  stats: { uid: number | bigint; mode: number | bigint },
  platform: CachePlatform,
): boolean {
  return platform.kind === "win32" || privatelyOwned(stats, platform.uid);
}

/**
 * Are these two stats the same file — and can we even tell?
 *
 * BigInt on both sides, always. NTFS file ids are 64-bit, and as a `number`
 * two different files above 2^53 can round to the same id, which would let a
 * swapped-in file pass for the one that was checked.
 *
 * Zero is "unknown", not an id. ReFS, Dev Drive and SMB shares can report a
 * zero file id or volume serial for every file, and `0n === 0n` would then
 * approve any swap at all. A check that cannot distinguish files is no check,
 * so it fails closed: a miss, and the lessons are parsed from disk.
 */
export function sameFile(
  opened: { dev: bigint; ino: bigint },
  checked: { dev: bigint; ino: bigint },
): boolean {
  if (opened.dev === 0n || opened.ino === 0n || checked.dev === 0n || checked.ino === 0n) {
    return false;
  }
  return opened.dev === checked.dev && opened.ino === checked.ino;
}

/**
 * Does `path`'s real location equal `expected`, exactly?
 *
 * `realpathSync.native` asks the OS (`realpath(3)`, or
 * `GetFinalPathNameByHandle` on Windows), which resolves every kind of
 * reparse point, not just the symlinks Node's JS implementation walks. A
 * junction, mount point or symlink anywhere at the leaf makes the real path
 * differ from where the name claims to live.
 */
function realPathIs(path: string, expected: string): boolean {
  return realpathSync.native(path) === expected;
}

/**
 * Create (if needed) and validate a private cache directory.
 *
 * Returns `undefined` — cache off, parse as usual — whenever the directory is
 * not provably private. It never loosens or repairs permissions it finds: a
 * directory that was ever readable or writable by someone else may already hold
 * files they planted, and making it private now would not remove them.
 */
export function openLessonCache(
  directory: string,
  platform: CachePlatform | undefined = hostCachePlatform(),
): LessonCache | undefined {
  const store = openEntryStore<ParsedLesson>(directory, platform);
  if (store === undefined) return undefined;
  return {
    directory: store.directory,
    key: (input) => lessonCacheKey(input, store.sourceDigest),
    read: store.read,
    write: store.write,
  };
}

/**
 * `openLessonCache` without the key: validate `directory` and return a store
 * whose entries hold arrays of `T`. Every check described above runs here and
 * only here.
 *
 * The type parameter is a promise about what the CALLER wrote, not something
 * this function can verify: `decodeEntry` checks that a payload is an array
 * and nothing more. That is sound because an entry is only ever found under a
 * key its own caller computed: each cache has its own fixed directory, and each
 * key starts with a format string no other cache uses (`LESSON_CACHE_FORMAT`,
 * `NARRATION_CACHE_FORMAT`), so even a shared directory could not confuse them.
 */
export function openEntryStore<T>(
  directory: string,
  platform: CachePlatform | undefined,
): EntryStore<T> | undefined {
  if (platform === undefined) return undefined;
  // Windows can make no ownership check, so it may use one directory only:
  // the fixed default inside this package's `node_modules`.
  if (platform.kind === "win32" && resolve(directory) !== resolve(platform.defaultDirectory)) {
    return undefined;
  }
  let resolved: string;
  let sourceDigest: string;
  try {
    // Computed here, eagerly, rather than on the first `key()`: a digest that
    // cannot be computed (an unreadable or dangling source file) must turn the
    // cache OFF, not make `loadTrackLessons` throw halfway through a test.
    sourceDigest = packageSourceDigest();
    // 0700 on every directory this call creates. An existing parent keeps its
    // own mode, which is fine: what must be private is the directory the
    // entries live in, and that is checked below whether or not we made it.
    mkdirSync(directory, { recursive: true, mode: 0o700 });
    // The directory ITSELF must not be a link — `lstat` sees the link, where
    // `stat` would see its target. Its PARENTS may be (a symlinked
    // `node_modules` is ordinary), so they are resolved, and the resolved
    // directory is checked again: a link swapped in after the first look then
    // fails the second.
    const leaf = lstatSync(directory);
    if (!leaf.isDirectory() || leaf.isSymbolicLink()) return undefined;
    // The leaf must resolve to itself: its parent's real path plus its own
    // name. A junction or other reparse point AT the leaf resolves elsewhere.
    resolved = realpathSync.native(directory);
    const parent = realpathSync.native(dirname(resolve(directory)));
    if (resolved !== join(parent, basename(resolve(directory)))) return undefined;
    const stats = lstatSync(resolved);
    if (!stats.isDirectory() || !ownedHere(stats, platform)) return undefined;
  } catch {
    return undefined;
  }
  return {
    directory: resolved,
    sourceDigest,
    read: (track, key) => readEntry<T>(resolved, platform, track, key),
    write: (track, key, values) => writeEntry(resolved, platform, track, key, values),
  };
}

/** This package's root: `src/` (under Vitest) or `dist/` (compiled), one level up. */
function packageRoot(): string {
  return resolve(dirname(fileURLToPath(import.meta.url)), "..");
}

/** The fixed, package-local default: `<package>/node_modules/.cache/...`. */
export function defaultLessonCacheDirectory(): string {
  return join(packageRoot(), "node_modules", ".cache", "human-language-data", "lessons");
}

/**
 * What the test suite's setup files pass to `configureLessonCache`.
 *
 * The root is the same arithmetic as `loader.ts`'s `defaultCurriculumRoot()` —
 * package -> typescript -> packages -> code, then `learning/human-languages` —
 * written out here so the setup files need not import the loader (and its whole
 * module graph) into every test file. `tests/lesson-cache.test.ts` asserts the
 * two agree, because a drift would not fail anything: the cache would just
 * never turn on.
 */
export function suiteLessonCacheOptions(): { root: string; directory: string } {
  return {
    root: join(packageRoot(), "..", "..", "..", "learning", "human-languages"),
    directory: defaultLessonCacheDirectory(),
  };
}

function entryName(language: string, key: string): string | undefined {
  const name = `${language}-${key}.v8`;
  return ENTRY_NAME.test(name) ? name : undefined;
}

// ---------------------------------------------------------------------------
// Reading an entry
// ---------------------------------------------------------------------------

/**
 * Read and verify one entry. Every failure is a miss.
 *
 * A file that fails verification is also deleted when it is one of ours (a
 * regular file we own), so a corrupt entry is replaced by the next write
 * instead of being re-verified and rejected on every load. A file we do NOT own
 * is left exactly where it is: deleting it is not this module's business, and
 * it can never be read anyway.
 */
function readEntry<T>(
  directory: string,
  platform: CachePlatform,
  language: string,
  key: string,
): T[] | undefined {
  const name = entryName(language, key);
  if (name === undefined) return undefined;
  const path = join(directory, name);
  let checked: BigIntStats;
  let fd: number;
  try {
    // Look before opening: a link (or junction) under an entry's name is a
    // miss, and so is anything whose real path is not `<directory>/<name>`.
    // This is the whole defence on Windows, which has no O_NOFOLLOW.
    checked = lstatSync(path, { bigint: true });
    if (!checked.isFile() || checked.isSymbolicLink() || !realPathIs(path, path)) return undefined;
    // O_NOFOLLOW where it exists: if a link was swapped in after the look, the
    // kernel refuses it (ELOOP) rather than following it.
    fd = platform.open(path, fsConstants.O_RDONLY | platform.noFollow);
  } catch {
    return undefined; // ENOENT is the ordinary miss; anything else is one too.
  }
  let bytes: Buffer;
  try {
    // `fstat` on the descriptor, not `stat` on the path: the checks are about
    // the file that is OPEN. It must be the very file `lstat` approved — same
    // device, same inode — so a swap between the look and the open, which
    // O_NOFOLLOW cannot catch where it does not exist, is refused here.
    const stats = platform.fstat(fd);
    if (!sameFile(stats, checked)) return undefined;
    if (!stats.isFile() || !ownedHere(stats, platform)) return undefined;
    bytes = readFileSync(fd);
  } catch {
    return undefined;
  } finally {
    closeSync(fd);
  }
  const values = decodeEntry<T>(bytes);
  if (values === undefined) discardOwnFile(directory, name, platform);
  return values;
}

/** Verify the header and digest, then deserialize. `undefined` on any doubt. */
export function decodeEntry<T = ParsedLesson>(bytes: Buffer): T[] | undefined {
  if (bytes.length < HEADER_BYTES) return undefined;
  if (!bytes.subarray(0, MAGIC.length).equals(MAGIC)) return undefined;
  const expected = bytes.subarray(MAGIC.length, HEADER_BYTES);
  const payload = bytes.subarray(HEADER_BYTES);
  const actual = createHash("sha256").update(payload).digest();
  if (!actual.equals(expected)) return undefined;
  try {
    const value: unknown = deserialize(payload);
    return Array.isArray(value) ? (value as T[]) : undefined;
  } catch {
    return undefined;
  }
}

/** Header + payload for `lessons`. Exported so tests can build exact bytes. */
export function encodeEntry(values: readonly unknown[]): Buffer {
  const payload = serialize(values);
  const digest = createHash("sha256").update(payload).digest();
  return Buffer.concat([MAGIC, digest, payload]);
}

/**
 * Delete one cache file — and only a cache file.
 *
 * Takes the directory and a bare name rather than a path, so it can only ever
 * reach a direct child of the cache directory, and the name must be an entry
 * or a temp file by this module's own grammar. Never a link (unlinking a link
 * is harmless, but there is no reason to touch something this module did not
 * write), and on POSIX never a file owned by someone else.
 */
function discardOwnFile(directory: string, name: string, platform: CachePlatform): void {
  if (!ENTRY_NAME.test(name) && !TEMP_NAME.test(name)) return;
  const path = join(directory, name);
  try {
    const stats = lstatSync(path);
    if (!stats.isFile() || stats.isSymbolicLink()) return;
    if (platform.kind === "posix" && stats.uid !== platform.uid) return;
    unlinkSync(path);
  } catch {
    // Already gone, or not ours to remove. Either way there is nothing to do.
  }
}

/**
 * Rename errors that mean "another writer got there first".
 *
 * Windows refuses to replace a file another process has open — Node opens
 * files with delete sharing, but a concurrent reader's handle can still make
 * `MoveFileEx` fail with one of these. The entry under that name is a hash of
 * the same content, so it already holds exactly what this writer had.
 */
const RENAME_COLLISION = new Set(["EPERM", "EACCES", "EBUSY"]);

// ---------------------------------------------------------------------------
// Writing an entry
// ---------------------------------------------------------------------------

/**
 * Write `values` under `key`, atomically, then retire the track's older entries.
 *
 *     1. create  <entry>.<pid>.<random>.tmp   O_CREAT | O_EXCL | O_NOFOLLOW, 0600
 *     2. write   header + payload             (every byte, or abandon)
 *     3. rename  temp -> <entry>               atomic: readers see old or new
 *     4. sweep   <track>-<other key>.v8        the older versions of this track
 *
 * O_EXCL means the create fails rather than opening a file that already
 * exists, so a temp name cannot be pre-planted, and O_NOFOLLOW refuses a
 * symlink in its place. The pid and 64 random bits keep two writers' temp files
 * apart.
 */
function writeEntry(
  directory: string,
  platform: CachePlatform,
  language: string,
  key: string,
  values: readonly unknown[],
): LessonCacheWrite {
  const name = entryName(language, key);
  if (name === undefined) return "not-written";
  const target = join(directory, name);
  const tempName = `${name}.${process.pid}.${randomBytes(8).toString("hex")}.tmp`;
  const temp = join(directory, tempName);
  let fd: number | undefined;
  // Set only once the temp file is complete and closed. Without it, an EPERM
  // or EACCES from CREATING the temp file would look exactly like a rename
  // collision — both leave `fd` undefined — and be reported as written.
  let renaming = false;
  try {
    const bytes = encodeEntry(values);
    // O_EXCL is the guard that matters on every platform: it maps to
    // CREATE_NEW on Windows, which fails if ANYTHING — file, link, junction —
    // already has the name, so a pre-planted temp name is never opened.
    fd = platform.open(
      temp,
      fsConstants.O_WRONLY | fsConstants.O_CREAT | fsConstants.O_EXCL | platform.noFollow,
      0o600,
    );
    let offset = 0;
    while (offset < bytes.length) offset += writeSync(fd, bytes, offset);
    closeSync(fd);
    fd = undefined;
    renaming = true;
    platform.rename(temp, target);
  } catch (error) {
    // Disk full, read-only checkout, a racing sweep, or (Windows) a rename
    // collision with a writer that got there first — the entry is either
    // already there or simply not written, and the next load parses. Never
    // leave our half-file behind.
    if (fd !== undefined) {
      try {
        closeSync(fd);
      } catch {
        // The descriptor is already unusable; nothing more to release.
      }
    }
    discardOwnFile(directory, tempName, platform);
    // A collision is not a failure, but it is not this writer's entry either,
    // so it does not sweep: the winner already did.
    const code = (error as NodeJS.ErrnoException | undefined)?.code ?? "";
    return renaming && RENAME_COLLISION.has(code) && existsSync(target)
      ? "already-written"
      : "not-written";
  }
  sweep(directory, platform, language, name);
  return "written";
}

/**
 * Delete this track's superseded entries, and any long-orphaned temp files.
 *
 * Deleting an entry another process is reading is safe on POSIX: `unlink`
 * removes the name, and an already-open descriptor keeps reading the file. A
 * process that has not opened it yet gets ENOENT, which is an ordinary miss.
 */
function sweep(directory: string, platform: CachePlatform, language: string, keep: string): void {
  let names: string[];
  try {
    names = readdirSync(directory);
  } catch {
    return;
  }
  const now = Date.now();
  for (const name of names) {
    const entry = ENTRY_NAME.exec(name);
    if (entry && entry[1] === language && name !== keep) {
      discardOwnFile(directory, name, platform);
      continue;
    }
    if (TEMP_NAME.test(name)) {
      try {
        const stats = lstatSync(join(directory, name));
        if (now - stats.mtimeMs > ORPHAN_AGE_MS) discardOwnFile(directory, name, platform);
      } catch {
        // Renamed or swept by another writer in the meantime.
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Process-wide configuration
// ---------------------------------------------------------------------------

/** The one curriculum root, and the one cache, this process is caching for. */
let configured: { root: string; cache: LessonCache } | undefined;

/**
 * Turn the cache on for one curriculum root, or off (`undefined`).
 *
 * Called by the Vitest setup files and by the cache's own tests — nothing else.
 * Returns whether the cache is actually on, since `openLessonCache` declines
 * whenever the directory is not provably private.
 */
export function configureLessonCache(
  options: { root: string; directory: string } | undefined,
  platform: CachePlatform | undefined = hostCachePlatform(),
): boolean {
  if (options === undefined) {
    configured = undefined;
    return false;
  }
  const cache = openLessonCache(options.directory, platform);
  configured = cache === undefined ? undefined : { root: resolve(options.root), cache };
  return configured !== undefined;
}

/** The cache to use for `root`, or `undefined` to parse as usual. */
export function lessonCacheFor(root: string): LessonCache | undefined {
  if (configured === undefined) return undefined;
  return resolve(root) === configured.root ? configured.cache : undefined;
}
