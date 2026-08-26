import { readdir, readFile, stat } from "node:fs/promises";
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
// CEILING -- this is debt; it may fall, never grow.
//
// 399 -> 353, LOWERED, in the same commit that raised the grouping `maxSize` in
// vite.config.ts from 49 kB to 56 kB. Regrouping took the measured count from
// 401 to 353, so the ceiling follows it down: a ceiling that may fall should
// fall when it falls, or the slack it keeps becomes room for the next
// regression to hide in. The count moved because grouping changed, not because
// the corpus shrank -- it grew by 35 lessons in the same commit.
//
// Read this and the `maxSize` beside it as a pair before changing either. The
// number below is the debt number. `maxSize` is a bundler GROUPING parameter,
// and raising it is how this number goes DOWN; it is not a second budget to
// spend. The budget that actually protects the browser is the 500 kB eager
// chunk limit below, and the largest lesson batch sits at about 11% of it.
if (lessonBatches.length > 353) {
  failures.push(`${lessonBatches.length} lesson requests exceed the 353-request ceiling`);
}
// Tracks the vite `maxSize` for lesson groups, with a little slack: rolldown
// caps a batch at the size given but a single module can overshoot it. Raised
// with the cap itself from 32 kB, and again to 56 kB -- see the note in
// vite.config.ts. This is not an independent budget; it MIRRORS the grouping
// parameter, so a batch the bundler did not intend to emit still fails here.
if (largestLessonBatch > 56_000) {
  failures.push(`largest lesson batch is ${largestLessonBatch} bytes (limit 56000)`);
}
if (largestEagerChunk > 500_000) {
  failures.push(`largest eager chunk is ${largestEagerChunk} bytes (limit 500000)`);
}

if (failures.length > 0) {
  for (const failure of failures) console.error(`bundle check: ${failure}`);
  process.exitCode = 1;
} else {
  console.log(
    `bundle check: ${lessonBatches.length} lesson batches, ` +
      `${largestLessonBatch} byte max lesson batch, ` +
      `${largestEagerChunk} byte max eager chunk`,
  );
}
