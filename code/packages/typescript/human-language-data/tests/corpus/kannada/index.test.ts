import { existsSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import "./exam-a1.js";
import "./script-closure.js";
import "./track-opening.js";
import "./writing-ramp.js";

const ownerDirectory = dirname(fileURLToPath(import.meta.url));
const ownerNames = ["exam-a1","script-closure","track-opening","writing-ramp"];
const expectedFiles = [
  "exam-a1.ts",
  "script-closure.ts",
  "track-opening.ts",
  "writing-ramp.ts",
];
const actualFiles = readdirSync(ownerDirectory)
  .filter((name) => name.endsWith(".ts") && name !== "index.test.ts")
  .sort();

if (existsSync(join(ownerDirectory, "..", "kannada.test.ts"))) {
  throw new Error("the retired flat kannada corpus-test aggregate must not be restored");
}
if (ownerNames.some((name) => !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(name))) {
  throw new Error("kannada corpus-test owner names must be safe lowercase slugs");
}
if (new Set(ownerNames).size !== ownerNames.length) {
  throw new Error("kannada corpus-test owner names must be unique");
}
if (JSON.stringify(actualFiles) !== JSON.stringify(expectedFiles)) {
  throw new Error(
    "the kannada corpus-test discovery entrypoint is stale: expected "
      + JSON.stringify(expectedFiles)
      + ", received "
      + JSON.stringify(actualFiles),
  );
}
