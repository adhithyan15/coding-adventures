// How lazy lesson batches are grouped, defined ONCE for both sides that need it.
//
// The bundler (vite.config.ts) uses this to name a chunk. The CI gate
// (scripts/check-bundle.mjs) uses it to derive how many chunks there should be
// and what they should be called. Those two must agree exactly, and the way to
// guarantee that is for there to be one definition rather than two that look
// alike.
//
// It lives in a plain .mjs so that both a TypeScript Vite config and a bare
// `node scripts/*.mjs` can import it without a build step. `lesson-bands.d.mts`
// carries the types.
//
// ---------------------------------------------------------------------------
// Why this is a module and not a constant read out of vite.config.ts
// ---------------------------------------------------------------------------
//
// The first version of the gate read the band width back out of vite.config.ts
// with `/export const LESSON_BAND_CHAPTERS = (\d+);/.exec(...)`. `exec` returns
// the FIRST match anywhere in the file, comments included — and vite.config.ts
// carries a long prose note that discusses this constant by name. A line as
// innocent as
//
//     // historical note: this was `export const LESSON_BAND_CHAPTERS = 1;`
//
// placed above the real declaration hands the checker a band width of 1 while
// the bundler keeps using 5. Smaller bands mean MORE bands, so the derived
// budget inflates — measured at 1,158 against a true 281, roughly 4x — and a
// grouping regression all the way back to byte-linear splitting passes
// unremarked. That is the same silent permissiveness that got the hardcoded 353
// removed in the first place.
//
// An import cannot be shadowed by a comment.

/** How many chapters share one lazy lesson batch. */
export const LESSON_BAND_CHAPTERS = 5;

/**
 * Longest chapter number accepted.
 *
 * Unbounded `\d+` reaches `Infinity` on a long enough digit run, and
 * `Math.floor(Infinity / 5) * 5` is `Infinity` — a perfectly good object key and
 * a perfectly absurd chunk name (`lessons-spanish-CInfinity`). Chapter numbers
 * beyond 2^53 also collapse distinct lessons into one band. Four digits is far
 * past any real corpus and keeps every value an exact integer.
 */
export const MAX_CHAPTER_DIGITS = 4;

/**
 * Track directory names this grouping will accept.
 *
 * `[^/]+` in the old module-id pattern excluded path separators and nothing
 * else, which is enough to keep a chunk inside `dist/assets` but not enough to
 * keep it fetchable: Rollup's filename sanitiser strips NUL, `?` and `*` but
 * leaves `#` and `%`, so a track directory named `es#1` produces an asset whose
 * URL the browser truncates at the fragment and whose lazy import 404s at
 * runtime. Whitelist instead of blacklist.
 */
const TRACK_NAME = /^[a-z0-9_-]+$/i;

/**
 * Lesson filename -> series letter and chapter number.
 *
 * The SERIES letter is captured, not assumed to be `C`. 599 of the corpus's
 * 4,154 lesson files are not `XX-C<digits>` — writing lessons (`AR-W00-…`) and
 * review lessons (`ES-R02-…`) among them. A chapter-only pattern returns null
 * for every one of them, which drops them out of the group on the bundler side
 * and out of the budget on the checker side, and the two errors do not cancel.
 */
const LESSON_ID = new RegExp(`^[A-Za-z]{2}-([A-Za-z])(\\d{1,${MAX_CHAPTER_DIGITS}})(?!\\d)`);

/**
 * The band a lesson file belongs to, or `null` if it is not a bandable lesson.
 *
 * @param {string} track     track directory name
 * @param {string} filename  lesson file basename
 */
export function lessonBand(track, filename) {
  if (!TRACK_NAME.test(track)) return null;
  const match = LESSON_ID.exec(filename);
  if (!match) return null;
  const chapter = Number(match[2]);
  if (!Number.isSafeInteger(chapter)) return null;
  return {
    track,
    series: match[1].toUpperCase(),
    band: Math.floor(chapter / LESSON_BAND_CHAPTERS) * LESSON_BAND_CHAPTERS,
  };
}

/** The chunk name a band gets. Rollup appends `-<hash>.js`. */
export function bandChunkName({ track, series, band }) {
  return `lessons-${track}-${series}${band}`;
}

/**
 * A bundler module id -> chunk name, or `null` to leave the module ungrouped.
 *
 * Ids arrive with the host's separators, so they are normalised before matching.
 */
export function bandChunkNameForModuleId(moduleId) {
  const normalized = String(moduleId).replaceAll("\\", "/");
  const match = /human-languages\/([^/]+)\/lessons\/([^/]+)$/.exec(normalized);
  if (!match) return null;
  const found = lessonBand(match[1], match[2]);
  return found ? bandChunkName(found) : null;
}
