import { existsSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import "./track.js";

const ownerDirectory = dirname(fileURLToPath(import.meta.url));
const ownerNames = ["track"];
const expectedFiles = [
  "track.ts",
];
const actualFiles = readdirSync(ownerDirectory)
  .filter((name) => name.endsWith(".ts") && name !== "index.test.ts")
  .sort();

if (existsSync(join(ownerDirectory, "..", "urdu.test.ts"))) {
  throw new Error("the retired flat urdu corpus-test aggregate must not be restored");
}
if (ownerNames.some((name) => !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(name))) {
  throw new Error("urdu corpus-test owner names must be safe lowercase slugs");
}
if (new Set(ownerNames).size !== ownerNames.length) {
  throw new Error("urdu corpus-test owner names must be unique");
}
if (JSON.stringify(actualFiles) !== JSON.stringify(expectedFiles)) {
  throw new Error(
    "the urdu corpus-test discovery entrypoint is stale: expected "
      + JSON.stringify(expectedFiles)
      + ", received "
      + JSON.stringify(actualFiles),
  );
}
