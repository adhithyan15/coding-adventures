import { existsSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import "./chapter-002.js";
import "./chapter-003.js";
import "./form-fields.js";
import "./form-integration.js";
import "./opening-writing.js";
import "./retrieval-r4.js";
import "./session-map.js";
import "./track.js";

const ownerDirectory = dirname(fileURLToPath(import.meta.url));
const ownerNames = ["chapter-002","chapter-003","form-fields","form-integration","opening-writing","retrieval-r4","session-map","track"];
const expectedFiles = [
  "chapter-002.ts",
  "chapter-003.ts",
  "form-fields.ts",
  "form-integration.ts",
  "opening-writing.ts",
  "retrieval-r4.ts",
  "session-map.ts",
  "track.ts",
];
const actualFiles = readdirSync(ownerDirectory)
  .filter((name) => name.endsWith(".ts") && name !== "index.test.ts")
  .sort();

if (existsSync(join(ownerDirectory, "..", "punjabi.test.ts"))) {
  throw new Error("the retired flat punjabi corpus-test aggregate must not be restored");
}
if (ownerNames.some((name) => !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(name))) {
  throw new Error("punjabi corpus-test owner names must be safe lowercase slugs");
}
if (new Set(ownerNames).size !== ownerNames.length) {
  throw new Error("punjabi corpus-test owner names must be unique");
}
if (JSON.stringify(actualFiles) !== JSON.stringify(expectedFiles)) {
  throw new Error(
    "the punjabi corpus-test discovery entrypoint is stale: expected "
      + JSON.stringify(expectedFiles)
      + ", received "
      + JSON.stringify(actualFiles),
  );
}
