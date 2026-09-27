import { existsSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import "./objective-activities.js";
import "./opening-writing.js";
import "./retrieval-r3.js";
import "./retrieval-r4.js";
import "./runway-b.js";
import "./session-map.js";
import "./track.js";

const ownerDirectory = dirname(fileURLToPath(import.meta.url));
const ownerNames = ["objective-activities","opening-writing","retrieval-r3","retrieval-r4","runway-b","session-map","track"];
const expectedFiles = [
  "objective-activities.ts",
  "opening-writing.ts",
  "retrieval-r3.ts",
  "retrieval-r4.ts",
  "runway-b.ts",
  "session-map.ts",
  "track.ts",
];
const actualFiles = readdirSync(ownerDirectory)
  .filter((name) => name.endsWith(".ts") && name !== "index.test.ts")
  .sort();

if (existsSync(join(ownerDirectory, "..", "gujarati.test.ts"))) {
  throw new Error("the retired flat gujarati corpus-test aggregate must not be restored");
}
if (ownerNames.some((name) => !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(name))) {
  throw new Error("gujarati corpus-test owner names must be safe lowercase slugs");
}
if (new Set(ownerNames).size !== ownerNames.length) {
  throw new Error("gujarati corpus-test owner names must be unique");
}
if (JSON.stringify(actualFiles) !== JSON.stringify(expectedFiles)) {
  throw new Error(
    "the gujarati corpus-test discovery entrypoint is stale: expected "
      + JSON.stringify(expectedFiles)
      + ", received "
      + JSON.stringify(actualFiles),
  );
}
