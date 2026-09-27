import { existsSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import "./chapter-007.js";
import "./exam-a1.js";
import "./short-o.js";
import "./track-opening.js";

const ownerDirectory = dirname(fileURLToPath(import.meta.url));
const ownerNames = ["chapter-007","exam-a1","short-o","track-opening"];
const expectedFiles = [
  "chapter-007.ts",
  "exam-a1.ts",
  "short-o.ts",
  "track-opening.ts",
];
const actualFiles = readdirSync(ownerDirectory)
  .filter((name) => name.endsWith(".ts") && name !== "index.test.ts")
  .sort();

if (existsSync(join(ownerDirectory, "..", "tamil.test.ts"))) {
  throw new Error("the retired flat tamil corpus-test aggregate must not be restored");
}
if (ownerNames.some((name) => !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(name))) {
  throw new Error("tamil corpus-test owner names must be safe lowercase slugs");
}
if (new Set(ownerNames).size !== ownerNames.length) {
  throw new Error("tamil corpus-test owner names must be unique");
}
if (JSON.stringify(actualFiles) !== JSON.stringify(expectedFiles)) {
  throw new Error(
    "the tamil corpus-test discovery entrypoint is stale: expected "
      + JSON.stringify(expectedFiles)
      + ", received "
      + JSON.stringify(actualFiles),
  );
}
