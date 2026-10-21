// ---------------------------------------------------------------------------
// narration-cache.ts — narrate the corpus once per test run, not once per test
// ---------------------------------------------------------------------------
//
// The problem this exists for
// ---------------------------
// `src/lesson-cache.ts` removed the repeated PARSE from the suite. What it left
// behind was the repeated NARRATION. Measured on main in October 2026, with
// 39,484 lessons in 7,811 chapters, on a 4-core machine:
//
//     narration-cli `--check`, in a test, lesson cache warm      23.0 s
//       loadLessons (a cache hit: read, verify, deserialize)        2.4 s
//       narrationChapters (every lesson narrated, every chapter)   12.8 s
//       JSON, text and fnv1a64 of the 248 MB export                 ~5 s
//       reading the committed files and comparing them              ~1.5 s
//
// and `tests/narration.test.ts` narrates the whole corpus a second time, with
// the same width, at collection time. Two full narrations per run, both of the
// same lessons, in two workers that know nothing of each other — the same shape
// of waste the lesson cache was written for, one layer up. The `--check` case
// is the one that timed out at its 60 s budget on a busy Windows runner.
//
// The idea
// --------
// `narrationChapters(lessons, options)` is a PURE function, so one track's
// chapters are a function of exactly:
//
//     that track's parsed lessons
//     the table width, and that track's chapter titles
//     the code that does the narrating
//
// and this module stores them under a SHA-256 of all of it, exactly as the
// lesson cache stores a parse. Same directory rules, same entry format, same
// atomic write, same refusals — it is the lesson cache's own store, opened on a
// sibling directory. Only the key differs:
//
//     key = SHA-256( format version, V8 version, ICU version, collation locale,
//                    package source digest,
//                    track id, table width, that track's chapter titles,
//                    the LESSON cache's key for that track's lessons )
//
// The last field is what names the lessons, and it is already a SHA-256 of
// every lesson file's bytes, the resolved script and the parser's source (see
// `lessonCacheKey`). A parsed lesson is a pure function of those, so equal
// lesson keys mean equal lessons, and folding the key in is as exact as
// hashing the lessons themselves.
//
// Why not hash the parsed lessons themselves
// ------------------------------------------
// The first version did: `v8.serialize(lessons)` into the key. It was exact,
// and it was useless exactly where it mattered. `v8.serialize` writes how V8
// happens to HOLD a value, not just what the value is: a freshly parsed
// lesson's arrays are dense, and the same arrays read back out of the lesson
// cache are not, so the two serialize to different bytes (German, measured:
// 23,680,372 vs 23,916,753 bytes, `isDeepStrictEqual` true). On a cold cache
// — every CI run — the global setup narrates freshly parsed lessons, while
// every worker narrates lessons deserialized from the lesson cache, so the
// warm-up was keyed under names no worker would ever look up. A key over the
// lessons' SOURCE has no representation to disagree about.
//
// Why that key can be trusted to describe the lessons it is used with
// --------------------------------------------------------------------
// It is reported by the loader (`loadLessons(root, onCacheKey)`) from inside
// the lesson cache, for the very bytes those lessons were parsed from or cached
// under — never recomputed from a second read of the files that an edit could
// slip between. And it is only ever used by `corpusNarration` in
// `narration-cli.ts`, which loads the lessons and narrates them in one call,
// never handing the array to anyone in between. A key that described lessons
// someone had since edited in memory would be wrong; there is no "since".
//
// Who uses it: TESTS ONLY, and only for the real curriculum
// ---------------------------------------------------------
// Off unless `configureNarrationCache()` is called, which only the Vitest setup
// files and this module's tests do. `generate:narration` and
// `check:narration` never turn it on, so the committed export is always
// WRITTEN by the plain, uncached path. Inside the suite, a cached answer is
// only ever compared against those files, never written into them.
//
// That comparison is CI's gate (the package's BUILD runs Vitest, not
// `check:narration`), so the key must name every input the narration reads.
// The parse is covered by the lesson key; the width and titles are fields of
// their own; and the narrator reads nothing else. Its runtime import graph —
// `narration.ts` and the eight `src/` modules it reaches — opens no file and
// reads no environment variable or clock, and every byte of it is in the
// package source digest. `tests/narration-cache.test.ts` walks that graph and
// fails if a file-system, environment or clock dependency ever appears, since
// that would be an input this key cannot see.
//
// The security model is the lesson cache's, verbatim
// --------------------------------------------------
// Every rule in the "security model" and "Windows" sections of
// `src/lesson-cache.ts` applies here unchanged, because this module does not
// open, read, write or delete a single file itself: it hands a directory to
// `openEntryStore`, which is the lesson cache's own validated store. The
// directory is fixed — `<package>/node_modules/.cache/human-language-data/
// narration`, beside the lesson cache's `lessons/` — and is computed from this
// package's location, never from an environment variable, argument or file. On
// Windows, where no ownership check can be made, the store accepts that one
// default directory and nothing else, exactly as it does for lessons.
//
// What a hit saves, and what it does not
// --------------------------------------
// A hit replaces the 12.8 s narration with reading the entry back (about
// 1.5 s for 126 MB); the `--check` case went from 23.0 s to 8.3 s. Everything
// downstream of the chapters still runs on every call: the gate still renders
// every chapter to text and JSON, still hashes both into the ledger, and still
// compares every byte with the committed files. The cache changes how the
// chapters are obtained, never what is compared or how.
//
// Disk usage
// ----------
// One entry per track, about 126 MB for today's corpus, beside the lesson
// cache's ~320 MB. A new entry for a track retires that track's older ones, and
// `npm ci` deletes the lot with `node_modules`.

import { createHash } from "node:crypto";
import { dirname, join, resolve } from "node:path";
import {
  type CachePlatform,
  type LessonCacheWrite,
  defaultLessonCacheDirectory,
  hostCachePlatform,
  openEntryStore,
  suiteLessonCacheOptions,
} from "./lesson-cache.js";
// Type-only, for the same reason `lesson-cache.ts` keeps `parse.js` type-only:
// the Vitest setup file imports this module into EVERY test file, and a runtime
// import of the narrator would load its whole module graph into the hundreds
// of files that never narrate anything. The narrating itself — the code that
// calls `narrationChapters` on a miss — lives in `narration-cli.ts`.
import type { ChapterNarration } from "./narration.js";

/**
 * Bump when the entry layout or the meaning of a key changes. It is the first
 * field hashed into every key, so bumping it orphans every existing entry.
 *
 * Distinct from `LESSON_CACHE_FORMAT` on purpose: it is what keeps a narration
 * key and a lesson key apart even if both were ever computed over the same
 * bytes, because the very first field already differs.
 */
export const NARRATION_CACHE_FORMAT = "hl-narration-cache/1";

/** What one track's narration key is computed from. */
export interface NarrationCacheKeyInput {
  /** The track id. */
  language: string;
  /** The width `policyTableWidth` read from `chapter-policy.json`. */
  maxLinearisableTableColumns: number;
  /** Chapter titles keyed `"<language>/<chapter>"`; only this track's are hashed. */
  titles: ReadonlyMap<string, string>;
  /** The lesson cache's key for this track's lessons, as the loader reported it. */
  lessonKey: string;
}

/**
 * The content address for one track's narration.
 *
 * Every variable-length field goes in length-first, as in `lessonCacheKey`, so
 * no two different inputs can produce the same byte stream: a title `"a/b"`
 * followed by `"c"` cannot hash like a title `"a"` followed by `"b/c"`.
 *
 * Only THIS track's titles are hashed, so authoring a title for one track's
 * chapter does not re-narrate the other twenty-two. That is exact, not merely
 * convenient: `narrationChapters` looks titles up by `"<language>/<chapter>"`,
 * and a track id never contains `/`, so another track's entries can never be
 * the ones it reads.
 */
export function narrationCacheKey(input: NarrationCacheKeyInput, sourceDigest: string): string {
  const hash = createHash("sha256");
  const field = (value: string): void => {
    const bytes = Buffer.from(value, "utf8");
    hash.update(`${bytes.length}:`);
    hash.update(bytes);
  };
  field(NARRATION_CACHE_FORMAT);
  field(process.versions.v8);
  // Chapters and findings are ordered with `localeCompare`, so — exactly as for
  // the parse — the ICU build and the default collation locale are inputs.
  field(process.versions.icu ?? "no-icu");
  field(new Intl.Collator().resolvedOptions().locale);
  field(sourceDigest);
  field(input.language);
  field(String(input.maxLinearisableTableColumns));
  const prefix = `${input.language}/`;
  const own = [...input.titles]
    .filter(([key]) => key.startsWith(prefix))
    .sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0));
  field(String(own.length));
  for (const [key, title] of own) {
    field(key);
    field(title);
  }
  // Not re-validated here: an ill-formed key only ever makes a different name,
  // which is a miss. `/^[0-9a-f]{64}$/` is what the lesson cache produces.
  field(input.lessonKey);
  return hash.digest("hex");
}

/**
 * A validated, private narration cache. Obtain one from `openNarrationCache`;
 * as with `LessonCache`, there is no way to build one around an unchecked path.
 */
export interface NarrationCache {
  readonly directory: string;
  key(input: NarrationCacheKeyInput): string;
  read(language: string, key: string): ChapterNarration[] | undefined;
  write(language: string, key: string, chapters: readonly ChapterNarration[]): LessonCacheWrite;
}

/** The fixed default, beside the lesson cache: `<package>/node_modules/.cache/human-language-data/narration`. */
export function defaultNarrationCacheDirectory(): string {
  return join(dirname(defaultLessonCacheDirectory()), "narration");
}

/**
 * The platform rules for the narration directory.
 *
 * `hostCachePlatform` is told which ONE directory Windows may use. Passing the
 * narration default here — rather than reusing the lesson platform, whose
 * Windows default is `lessons/` — is what lets Windows cache narration at all,
 * and it is still exactly one fixed directory inside this package's
 * `node_modules`, so the reasoning in `lesson-cache.ts`'s "Windows" section
 * holds word for word.
 */
export function hostNarrationCachePlatform(): CachePlatform | undefined {
  return hostCachePlatform(defaultNarrationCacheDirectory());
}

/**
 * Create (if needed) and validate a private narration cache directory.
 *
 * Every check is `openEntryStore`'s — the lesson cache's own — so the two
 * caches cannot drift apart: `undefined`, cache off, whenever the directory is
 * not provably private.
 */
export function openNarrationCache(
  directory: string,
  platform: CachePlatform | undefined = hostNarrationCachePlatform(),
): NarrationCache | undefined {
  const store = openEntryStore<ChapterNarration>(directory, platform);
  if (store === undefined) return undefined;
  return {
    directory: store.directory,
    key: (input) => narrationCacheKey(input, store.sourceDigest),
    read: store.read,
    write: store.write,
  };
}

/** What the suite's setup files pass to `configureNarrationCache`. */
export function suiteNarrationCacheOptions(): { root: string; directory: string } {
  return { root: suiteLessonCacheOptions().root, directory: defaultNarrationCacheDirectory() };
}

/** The one curriculum root, and the one cache, this process narrates through. */
let configured: { root: string; cache: NarrationCache } | undefined;

/**
 * Turn the narration cache on for one curriculum root, or off (`undefined`).
 * Called by the Vitest setup files and this module's tests — nothing else.
 */
export function configureNarrationCache(
  options: { root: string; directory: string } | undefined,
  platform: CachePlatform | undefined = hostNarrationCachePlatform(),
): boolean {
  if (options === undefined) {
    configured = undefined;
    return false;
  }
  const cache = openNarrationCache(options.directory, platform);
  configured = cache === undefined ? undefined : { root: resolve(options.root), cache };
  return configured !== undefined;
}

/** The narration cache to use for `root`, or `undefined` to narrate as usual. */
export function narrationCacheFor(root: string): NarrationCache | undefined {
  if (configured === undefined) return undefined;
  return resolve(root) === configured.root ? configured.cache : undefined;
}
