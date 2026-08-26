import { lstat, readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { bandChunkName, lessonBand } from "../lesson-bands.mjs";

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

// `lstat`, not `stat`. This walk is unbounded in depth, and `stat` resolves
// links -- so a symlink cycle anywhere under src/ or the corpus recurses until
// it exhausts file descriptors or the stack. A link's own mtime is not
// interesting here anyway; what matters is the files the bundler reads.
// Pre-existing, fixed in this pass because it is the same hazard the new corpus
// walk below is being guarded against and it would be odd to fix only one.
async function newestMtime(target) {
  const info = await lstat(target).catch(() => null);
  if (!info || info.isSymbolicLink()) return 0;
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
// THE REQUEST BUDGET IS DERIVED FROM THE CORPUS, NOT HARDCODED (#12918).
//
// This was a constant, and the constant was the problem. Lesson batches used to
// be grouped by track and split by size, which made the emitted count a function
// of corpus BYTES; the constant therefore had to be raised or the grouping
// re-tuned every few content tranches, and each answer bought exactly one
// tranche of room. Worse, a constant sized for today's corpus silently permits a
// real grouping regression tomorrow: 353 was met on a corpus that only needed
// 281, so 72 batches of drift would have passed unremarked.
//
// Batches are now grouped by a chapter range (see lesson-bands.mjs). One band is
// one track's lesson series over LESSON_BAND_CHAPTERS chapters, so what the
// bundler should have emitted is a property of the CORPUS and is computed here
// rather than remembered.
//
//   * adding lessons inside an existing band changes nothing -> passes;
//   * adding chapters adds a band AND a chunk -> passes;
//   * a grouping regression breaks the correspondence -> FAILS, at any corpus
//     size, which a constant could not do.
// One entry per (track, lesson series, chapter band) the corpus actually holds.
// The parsing is imported, not restated, so this cannot disagree with the
// bundler about which files are lessons or which band they fall in.
//
// EVERY GUARD HERE POINTS THE SAME WAY, AND IT IS WORTH SAYING WHICH WAY. This
// gate fails when the emitted chunks do not match the bands. Anything that
// invents a band makes room for a chunk nobody authored, so it makes the gate
// pass when it should fail -- the only failure mode of a CI check that matters.
//
// `isDirectory()` is false for a symlink when the entry came from
// `withFileTypes`, so a symlinked TRACK is already skipped. The `lessons` hop
// takes a path rather than a dirent, so it is the one that would follow a link:
// `<track>/lessons -> /anywhere` would enumerate a directory outside the
// repository and mint a band per plausible filename found there. Vite's glob
// resolves symlinks, so those modules would not even become lesson chunks --
// budget up, emission flat, gate looser.
const corpusRoot = path.resolve("../../../learning/human-languages");
const bands = new Map();
let lessonFiles = 0;
// A missing corpus root must reach the anti-vacuity guard below carrying a
// diagnosis, not escape as an unhandled ENOENT stack trace.
let tracks = [];
try {
  tracks = await readdir(corpusRoot, { withFileTypes: true });
} catch {
  tracks = [];
}
for (const track of tracks) {
  if (!track.isDirectory() || track.name.startsWith(".")) continue;
  const lessonDir = path.join(corpusRoot, track.name, "lessons");
  // `lstat`, not `stat`: `stat` resolves the link and reports the TARGET as a
  // directory, which is exactly the case being refused.
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
    continue;
  }
  for (const entry of entries) {
    // `isFile()` is likewise false for a symlink, so a link farm inside one
    // track cannot mint bands either.
    if (!entry.isFile() || !entry.name.endsWith(".md")) continue;
    lessonFiles += 1;
    const found = lessonBand(track.name, entry.name);
    if (!found) continue;
    // Rollup appends `-<hash>.js`, and its hashes contain `-` and `_`, so a
    // filename cannot be parsed back into a band unambiguously. Match forwards
    // on the prefix instead: that direction has no ambiguity at all.
    bands.set(`${bandChunkName(found)}-`, 0);
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

// COMPARE THE SETS, NOT THE COUNTS.
//
// An earlier version of this gate compared `lessonBatches.length` against
// `bands.size + slack` and threw the identities away. That passes a regression
// which reshapes the grouping without changing the total -- one band's modules
// scattered into another's chunk while a third splits -- and the data to catch
// it was already in hand. It also sat at zero margin, so the pressure on the
// next legitimate backstop split would have been to bump the slack rather than
// look at the band width.
const unexplained = [];
for (const name of lessonBatches) {
  const prefix = [...bands.keys()].find((candidate) => name.startsWith(candidate));
  if (prefix === undefined) unexplained.push(name);
  else bands.set(prefix, bands.get(prefix) + 1);
}
const missing = [...bands].filter(([, count]) => count === 0).map(([prefix]) => prefix);
const split = [...bands].filter(([, count]) => count > 1);
const extraChunks = split.reduce((sum, [, count]) => sum + count - 1, 0);

if (unexplained.length > 0) {
  failures.push(
    `${unexplained.length} lesson batch(es) match no chapter band in the corpus, ` +
      `e.g. ${unexplained.slice(0, 3).join(", ")}. Grouping and the corpus disagree ` +
      `about which files are lessons — see lesson-bands.mjs.`,
  );
}
if (missing.length > 0) {
  failures.push(
    `${missing.length} chapter band(s) produced no lesson batch, ` +
      `e.g. ${missing.slice(0, 3).join(", ")}. Modules have been absorbed into ` +
      `another band's chunk.`,
  );
}
// BAND_SPLIT_SLACK is the debt number, and it is a small honest one: how many
// EXTRA chunks the 256 kB backstop in vite.config.ts has to carve out of bands
// too dense to ship whole. Measured at 1 (Spanish chapters 5-9), and stable at 1
// across a 35-lesson tranche. It may fall, never grow. If it grows, the backstop
// has started doing the splitter's job again and the BAND WIDTH is what should
// change -- not this number. The message names the band so that is a decision
// rather than a bump.
const BAND_SPLIT_SLACK = 1;
if (extraChunks > BAND_SPLIT_SLACK) {
  failures.push(
    `the size backstop split ${extraChunks} band(s) beyond the ${BAND_SPLIT_SLACK} ` +
      `allowed: ${split.map(([prefix, count]) => `${prefix} into ${count}`).join(", ")}. ` +
      `Reduce LESSON_BAND_CHAPTERS rather than raising this number.`,
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
    `bundle check: ${lessonBatches.length} lesson batches over ${bands.size} chapter bands${extraChunks > 0 ? ` (+${extraChunks} backstop split: ${split.map(([p]) => p).join(", ")})` : ""}, ` +
      `${largestLessonBatch} byte max lesson batch, ` +
      `${largestEagerChunk} byte max eager chunk`,
  );
}
