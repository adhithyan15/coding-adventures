import { existsSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import "./chapter-014-family.js";
import "./form-label.js";
import "./track.js";
import "./writing-ramp.js";

const ownerDirectory = dirname(fileURLToPath(import.meta.url));
const ownerNames = ["chapter-014-family","form-label","track","writing-ramp"];
const expectedFiles = [
  "chapter-014-family.ts",
  "form-label.ts",
  "track.ts",
  "writing-ramp.ts",
];
const actualFiles = readdirSync(ownerDirectory)
  .filter((name) => name.endsWith(".ts") && name !== "index.test.ts")
  .sort();

if (existsSync(join(ownerDirectory, "..", "marathi.test.ts"))) {
  throw new Error("the retired flat marathi corpus-test aggregate must not be restored");
}
if (ownerNames.some((name) => !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(name))) {
  throw new Error("marathi corpus-test owner names must be safe lowercase slugs");
}
if (new Set(ownerNames).size !== ownerNames.length) {
  throw new Error("marathi corpus-test owner names must be unique");
}
if (JSON.stringify(actualFiles) !== JSON.stringify(expectedFiles)) {
  throw new Error(
    "the marathi corpus-test discovery entrypoint is stale: expected "
      + JSON.stringify(expectedFiles)
      + ", received "
      + JSON.stringify(actualFiles),
  );
}
