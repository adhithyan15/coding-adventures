import { lstat, readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";

const assetsDir = path.resolve("dist/assets");
const names = await readdir(assetsDir);
const javascript = names.filter((name) => name.endsWith(".js"));
const lessonBatches = javascript.filter((name) => name.startsWith("lessons-"));
// Which chunks are eager is READ FROM THE BUILD, not listed here.
//
// This used to be a hardcoded name pattern, and it went stale the moment a
// chunk was made lazy: `book-ledgers` was moved behind a dynamic import and
// the gate went on counting it against the eager ceiling, reporting a
// 500 kB failure for bytes the browser no longer downloads on first paint.
// A name list cannot know what the bundler decided.
//
// The browser's own definition is in `dist/index.html`: the entry `<script
// type="module">` plus every `<link rel="modulepreload">` is exactly what is
// fetched before the app paints. Anything else arrives on demand.
// Refuse to report on a stale build.
//
// This script reads `dist/`, so running it without building first measures the
// LAST build, not the current source. That is not a hypothetical: the number
// 487,797 was reported as passing verification on five consecutive pull
// requests while forty lessons were added, because `dist/` was never rebuilt
// between them. A measurement that cannot move is worse than no measurement,
// because it reads as evidence.
//
// So: if any source or corpus file is newer than the built entry HTML, stop.
const indexPath = path.resolve("dist/index.html");
const builtAt = (await stat(indexPath)).mtimeMs;
const watched = [
  path.resolve("src"),
  path.resolve("../../../learning/human-languages"),
];
// Only files the bundler could actually pull in count as sources. The corpus
// tree also holds the LaTeX books, and every local book build rewrites a .log,
// a .aux and a .pdf under it — none of which the app imports. Counting those
// left the guard permanently tripped after a `build-books-locally.sh` run,
// which teaches the exact habit it exists to prevent: ignoring it.
const BUNDLED_EXTENSIONS = new Set([
  ".md", ".json", ".svg", ".ttf", ".woff2",
  ".ts", ".tsx", ".js", ".mjs", ".css", ".html",
]);

async function newestMtime(target) {
  const info = await stat(target).catch(() => null);
  if (!info) return 0;
  if (!info.isDirectory()) {
    return BUNDLED_EXTENSIONS.has(path.extname(target)) ? info.mtimeMs : 0;
  }
  const entries = await readdir(target, { withFileTypes: true });
  const times = await Promise.all(
    entries
      .filter((entry) => entry.name !== "node_modules" && !entry.name.startsWith("."))
      .map((entry) => newestMtime(path.join(target, entry.name))),
  );
  return Math.max(0, ...times);
}
const newestSource = Math.max(...(await Promise.all(watched.map(newestMtime))));
if (newestSource > builtAt) {
  console.error(
    "bundle check: dist/ is older than the sources it was built from. " +
      "Run `npm run build` first — this check reads the build, not the source.",
  );
  process.exit(1);
}

const indexHtml = await readFile(indexPath, "utf8");
const preloaded = new Set(
  [...indexHtml.matchAll(/assets\/([^"']+\.js)/g)].map((match) => match[1]),
);
const eager = javascript.filter((name) => preloaded.has(name));
if (eager.length === 0) {
  console.error("bundle check: found no eager chunks in dist/index.html — is the build stale?");
  process.exit(1);
}
const handwritingChunks = javascript.filter((name) =>
  name.startsWith("handwriting-tools-"),
);

async function largestBytes(files) {
  const sizes = await Promise.all(
    files.map(async (name) => (await stat(path.join(assetsDir, name))).size),
  );
  return Math.max(0, ...sizes);
}

const largestLessonBatch = await largestBytes(lessonBatches);
const largestEagerChunk = await largestBytes(eager);
const failures = [];

if (lessonBatches.length === 0) failures.push("no lazy lesson batches were emitted");
if (handwritingChunks.length !== 1) {
  failures.push(
    `expected one handwriting-tools chunk, found ${handwritingChunks.length}`,
  );
}
// THE REQUEST BUDGET IS DERIVED, NOT HARDCODED (#12918).
//
// This was a constant, and the constant was the problem. Lesson batches used to
// be grouped by track and split by size, which made the emitted count a function
// of corpus BYTES; the constant therefore had to be raised or the grouping
// re-tuned every few content tranches, and each answer bought exactly one
// tranche of room. Worse, a constant sized for today's corpus silently permits a
// real grouping regression tomorrow: 353 was met on a corpus that only needed
// 281, so 72 batches of drift would have passed unremarked.
//
// Batches are now grouped by a chapter range (see vite.config.ts). One band is
// one track's lesson series over LESSON_BAND_CHAPTERS chapters, so the number of
// bands is a property of the CORPUS and can be counted here directly. The gate
// compares what the bundler emitted against what the grouping says it should
// have emitted.
//
// What that buys, and why it ends the recurrence:
//
//   * adding lessons inside an existing band moves neither side -> passes;
//   * adding chapters moves both sides together -> passes;
//   * a grouping regression moves only the emitted count -> FAILS, at any
//     corpus size, which a constant could not do.
//
// BAND_SPLIT_SLACK is the debt number, and it is small and meaningful: it counts
// how many bands are dense enough that the 256 kB backstop in vite.config.ts has
// to split them. Measured at 1, and stable at 1 across a 35-lesson tranche. It
// may fall, never grow. If it grows, the backstop is doing the splitter's job
// again and the band width is what should change, not this number.
const BAND_SPLIT_SLACK = 1;

// Read the band width from the bundler config rather than restating it, so the
// two cannot drift.
const viteConfig = await readFile(path.resolve("vite.config.ts"), "utf8");
const bandWidth = Number(
  /export const LESSON_BAND_CHAPTERS = (\d+);/.exec(viteConfig)?.[1] ?? 0,
);
if (!Number.isInteger(bandWidth) || bandWidth <= 0) {
  console.error(
    "bundle check: could not read LESSON_BAND_CHAPTERS from vite.config.ts — " +
      "the request budget is derived from it and cannot be computed without it.",
  );
  process.exit(1);
}

// One entry per (track, lesson series, chapter band) the corpus actually holds.
//
// The series letter is part of the key on purpose. 599 of the corpus's lesson
// files are not `XX-C<digits>` — writing lessons (`AR-W00-…`) and review
// lessons (`ES-R02-…`) among them — and a pattern matching only `-C` drops
// every one of them, undercounting both sides of this comparison.
const corpusRoot = path.resolve("../../../learning/human-languages");
const bands = new Set();
let lessonFiles = 0;
// A missing corpus root must reach the anti-vacuity guard below carrying a
// diagnosis, not escape as an unhandled ENOENT stack trace. The exit code was
// never in doubt either way, but "no such file or directory, scandir" sends the
// reader to the filesystem when the answer is that this gate cannot derive a
// budget without a corpus to derive it from.
let tracks = [];
try {
  tracks = await readdir(corpusRoot, { withFileTypes: true });
} catch {
  tracks = [];
}
// EVERY GUARD BELOW POINTS THE SAME WAY, AND IT IS WORTH SAYING WHICH WAY.
//
// This gate fails when `batches > bands + slack`. So anything that inflates the
// BAND count raises the budget and makes the gate pass when it should fail —
// which is the only failure mode of a CI check that matters. Nothing here is
// defending the build; it is defending the check's own trustworthiness.
//
// `isDirectory()` is false for a symlink when the entry came from
// `withFileTypes`, so a symlinked track is already skipped above. The inner
// `readdir` is the one that would follow a link, because it takes a path rather
// than a dirent — the same trap the `lstatSync` guards elsewhere in this file
// were written for. A symlinked `lessons/` pointing back at another track would
// duplicate every one of its bands under a second track name and silently buy
// the budget a few dozen batches of slack.
const MAX_CHAPTER_DIGITS = 6;
for (const track of tracks) {
  if (!track.isDirectory() || track.name.startsWith(".")) continue;
  const lessonDir = path.join(corpusRoot, track.name, "lessons");
  // `lstat`, not `stat`: `stat` resolves the link and would report the TARGET
  // as a directory, which is exactly the case being refused.
  let lessonDirInfo;
  try {
    lessonDirInfo = await lstat(lessonDir);
  } catch {
    continue; // a track with no lessons/ directory yet
  }
  if (!lessonDirInfo.isDirectory()) continue;
  let entries;
  try {
    entries = await readdir(lessonDir, { withFileTypes: true });
  } catch {
    continue; // a track with no readable lessons/ directory yet
  }
  for (const entry of entries) {
    // `isFile()` is likewise false for a symlink, so a link farm inside one
    // track cannot mint bands either.
    if (!entry.isFile() || !entry.name.endsWith(".md")) continue;
    lessonFiles += 1;
    // The digit run is bounded so `Number()` cannot reach Infinity or leave the
    // safe-integer range: `Math.floor(Infinity / bandWidth) * bandWidth` is
    // `Infinity`, which is a perfectly good Set key and would add a band nobody
    // authored. A chapter number needing seven digits is not a chapter number.
    const match = new RegExp(`^[A-Za-z]{2}-([A-Za-z])(\\d{1,${MAX_CHAPTER_DIGITS}})(?!\\d)`).exec(
      entry.name,
    );
    if (!match) continue;
    const band = Math.floor(Number(match[2]) / bandWidth) * bandWidth;
    bands.add(`${track.name}|${match[1].toUpperCase()}|${band}`);
  }
}
// Anti-vacuity: an empty or unreadable corpus would make the budget zero and
// this gate would pass on a build with no lesson batches at all.
if (lessonFiles === 0 || bands.size === 0) {
  console.error(
    "bundle check: found no lesson files under the curriculum tree — " +
      "refusing to derive a request budget from an empty corpus.",
  );
  process.exit(1);
}

const requestBudget = bands.size + BAND_SPLIT_SLACK;
if (lessonBatches.length > requestBudget) {
  failures.push(
    `${lessonBatches.length} lesson batches exceed the derived budget of ` +
      `${requestBudget} (${bands.size} chapter bands + ${BAND_SPLIT_SLACK} split). ` +
      `Grouping has drifted back toward splitting by size — see vite.config.ts.`,
  );
}
// Mirrors the `maxSize` BACKSTOP in vite.config.ts, so a batch the bundler did
// not intend to emit still fails here. It is not an independent budget, and it
// is not the constraint that protects the browser — that is the 500 kB eager
// limit below, which lesson batches never touch because they are lazy.
//
// 56_000 -> 262_144 with the move to chapter-range grouping (#12918). The number
// went UP because it stopped being the splitter: under the old shape it decided
// where every batch ended, and under this one it only catches a band dense
// enough to be worth splitting. Largest measured batch is 200,124 B.
if (largestLessonBatch > 262_144) {
  failures.push(`largest lesson batch is ${largestLessonBatch} bytes (limit 262144)`);
}
if (largestEagerChunk > 500_000) {
  failures.push(`largest eager chunk is ${largestEagerChunk} bytes (limit 500000)`);
}

if (failures.length > 0) {
  for (const failure of failures) console.error(`bundle check: ${failure}`);
  process.exitCode = 1;
} else {
  console.log(
    `bundle check: ${lessonBatches.length} lesson batches (budget ${requestBudget} = ${bands.size} bands + ${BAND_SPLIT_SLACK}), ` +
      `${largestLessonBatch} byte max lesson batch, ` +
      `${largestEagerChunk} byte max eager chunk`,
  );
}
