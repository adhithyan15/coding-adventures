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
//   2. It is a real directory (not a symlink, after resolving its parents),
//      owned by the current user, with no group or other permission bits
//      (mode 0700). Anything else disables the cache for the process.
//   3. The entry is opened with O_NOFOLLOW, so a symlink planted under an
//      entry's name is refused by the kernel rather than followed.
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
// On Windows (`process.getuid` is absent) the ownership and mode checks above
// have no faithful equivalent — permissions are ACLs, not mode bits — so the
// cache stays off and the tests parse, exactly as they did before.
//
// Concurrency
// -----------
// Several Vitest workers can miss the same entry at once and all write it.
// Each writes its own temp file — created with O_EXCL, so two writers can
// never share one — and then `rename`s it over the entry's name. `rename` is
// atomic on POSIX: a reader opening the name sees either the old complete file
// or the new complete file, never a mixture. Because the name is a hash of the
// content, every racing writer is writing the same answer, so it does not
// matter which rename lands last.
//
// Disk usage
// ----------
// One entry per track: about 320 MB for today's whole corpus. Whenever an entry
// is written, the older entries for the same track are deleted, so an edited
// track replaces its entry rather than accumulating them. `npm ci` deletes
// `node_modules`, and the cache with it.

import { createHash, randomBytes } from "node:crypto";
import {
  closeSync,
  constants as fsConstants,
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
import { dirname, join, resolve } from "node:path";
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
  write(language: string, key: string, lessons: readonly ParsedLesson[]): void;
}

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
export function privatelyOwned(stats: { uid: number; mode: number }, uid: number): boolean {
  return stats.uid === uid && (stats.mode & GROUP_OR_OTHER) === 0;
}

/** The current user's id, or `undefined` where POSIX ownership does not exist. */
function currentUid(): number | undefined {
  return typeof process.getuid === "function" ? process.getuid() : undefined;
}

/**
 * Create (if needed) and validate a private cache directory.
 *
 * Returns `undefined` — cache off, parse as usual — whenever the directory is
 * not provably private. It never loosens or repairs permissions it finds: a
 * directory that was ever readable or writable by someone else may already hold
 * files they planted, and making it private now would not remove them.
 */
export function openLessonCache(directory: string): LessonCache | undefined {
  const uid = currentUid();
  if (uid === undefined) return undefined;
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
    if (!lstatSync(directory).isDirectory()) return undefined;
    resolved = realpathSync(directory);
    const stats = lstatSync(resolved);
    if (!stats.isDirectory() || !privatelyOwned(stats, uid)) return undefined;
  } catch {
    return undefined;
  }
  return {
    directory: resolved,
    key: (input) => lessonCacheKey(input, sourceDigest),
    read: (language, key) => readEntry(resolved, uid, language, key),
    write: (language, key, lessons) => writeEntry(resolved, uid, language, key, lessons),
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
function readEntry(
  directory: string,
  uid: number,
  language: string,
  key: string,
): ParsedLesson[] | undefined {
  const name = entryName(language, key);
  if (name === undefined) return undefined;
  const path = join(directory, name);
  let fd: number;
  try {
    // O_NOFOLLOW: if the final component is a symlink the open fails (ELOOP)
    // instead of reading the link's target.
    fd = openSync(path, fsConstants.O_RDONLY | fsConstants.O_NOFOLLOW);
  } catch {
    return undefined; // ENOENT is the ordinary miss; anything else is one too.
  }
  let bytes: Buffer;
  try {
    // `fstat` on the descriptor, not `stat` on the path: the checks are about
    // the file that is OPEN, so a rename between the check and the read cannot
    // swap in a different one.
    const stats = fstatSync(fd);
    if (!stats.isFile() || !privatelyOwned(stats, uid)) return undefined;
    bytes = readFileSync(fd);
  } catch {
    return undefined;
  } finally {
    closeSync(fd);
  }
  const lessons = decodeEntry(bytes);
  if (lessons === undefined) discardOwnFile(path, uid);
  return lessons;
}

/** Verify the header and digest, then deserialize. `undefined` on any doubt. */
export function decodeEntry(bytes: Buffer): ParsedLesson[] | undefined {
  if (bytes.length < HEADER_BYTES) return undefined;
  if (!bytes.subarray(0, MAGIC.length).equals(MAGIC)) return undefined;
  const expected = bytes.subarray(MAGIC.length, HEADER_BYTES);
  const payload = bytes.subarray(HEADER_BYTES);
  const actual = createHash("sha256").update(payload).digest();
  if (!actual.equals(expected)) return undefined;
  try {
    const value: unknown = deserialize(payload);
    return Array.isArray(value) ? (value as ParsedLesson[]) : undefined;
  } catch {
    return undefined;
  }
}

/** Header + payload for `lessons`. Exported so tests can build exact bytes. */
export function encodeEntry(lessons: readonly ParsedLesson[]): Buffer {
  const payload = serialize(lessons);
  const digest = createHash("sha256").update(payload).digest();
  return Buffer.concat([MAGIC, digest, payload]);
}

function discardOwnFile(path: string, uid: number): void {
  try {
    const stats = lstatSync(path);
    if (stats.isFile() && stats.uid === uid) unlinkSync(path);
  } catch {
    // Already gone, or not ours to remove. Either way there is nothing to do.
  }
}

// ---------------------------------------------------------------------------
// Writing an entry
// ---------------------------------------------------------------------------

/**
 * Write `lessons` under `key`, atomically, then retire the track's older entries.
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
  uid: number,
  language: string,
  key: string,
  lessons: readonly ParsedLesson[],
): void {
  const name = entryName(language, key);
  if (name === undefined) return;
  const target = join(directory, name);
  const temp = `${target}.${process.pid}.${randomBytes(8).toString("hex")}.tmp`;
  let fd: number | undefined;
  try {
    const bytes = encodeEntry(lessons);
    fd = openSync(
      temp,
      fsConstants.O_WRONLY | fsConstants.O_CREAT | fsConstants.O_EXCL | fsConstants.O_NOFOLLOW,
      0o600,
    );
    let offset = 0;
    while (offset < bytes.length) offset += writeSync(fd, bytes, offset);
    closeSync(fd);
    fd = undefined;
    renameSync(temp, target);
  } catch {
    // Disk full, read-only checkout, a racing sweep — the entry simply is not
    // written, and the next load parses. Never leave our half-file behind.
    if (fd !== undefined) {
      try {
        closeSync(fd);
      } catch {
        // The descriptor is already unusable; nothing more to release.
      }
    }
    discardOwnFile(temp, uid);
    return;
  }
  sweep(directory, uid, language, name);
}

/**
 * Delete this track's superseded entries, and any long-orphaned temp files.
 *
 * Deleting an entry another process is reading is safe on POSIX: `unlink`
 * removes the name, and an already-open descriptor keeps reading the file. A
 * process that has not opened it yet gets ENOENT, which is an ordinary miss.
 */
function sweep(directory: string, uid: number, language: string, keep: string): void {
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
      discardOwnFile(join(directory, name), uid);
      continue;
    }
    if (TEMP_NAME.test(name)) {
      try {
        const stats = lstatSync(join(directory, name));
        if (now - stats.mtimeMs > ORPHAN_AGE_MS) discardOwnFile(join(directory, name), uid);
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
): boolean {
  if (options === undefined) {
    configured = undefined;
    return false;
  }
  const cache = openLessonCache(options.directory);
  configured = cache === undefined ? undefined : { root: resolve(options.root), cache };
  return configured !== undefined;
}

/** The cache to use for `root`, or `undefined` to parse as usual. */
export function lessonCacheFor(root: string): LessonCache | undefined {
  if (configured === undefined) return undefined;
  return resolve(root) === configured.root ? configured.cache : undefined;
}
