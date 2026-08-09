import { readdir, stat } from "node:fs/promises";
import path from "node:path";

const assetsDir = path.resolve("dist/assets");
const names = await readdir(assetsDir);
const javascript = names.filter((name) => name.endsWith(".js"));
const lessonBatches = javascript.filter((name) => name.startsWith("lessons-"));
const eager = javascript.filter((name) =>
  /^(?:index|script-data|curriculum-plans|book-ledgers|handwriting-tools)-/.test(name),
);
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
if (lessonBatches.length >= 400) {
  failures.push(`${lessonBatches.length} lesson requests exceed the 399-request ceiling`);
}
if (largestLessonBatch > 33_000) {
  failures.push(`largest lesson batch is ${largestLessonBatch} bytes (limit 33000)`);
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
