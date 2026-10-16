import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";

const caseDirectory = dirname(fileURLToPath(import.meta.url));
const thisCase = fileURLToPath(import.meta.url).split(/[\\/]/u).at(-1);
const censusOwner =
  "pins-sanskrit-a1-exam-coverage-which-nothing-read-before-c2d225a1.case.ts";

it("keeps Sanskrit A1 aggregate coverage pins in one census owner", () => {
  const aggregateMarkers = [
    "coverage." + "enumerated",
    "coverage." + "covered",
    "coverage." + "unmapped",
    "coverage." + "partial",
    "coverage." + "byCategory",
    "formatExamCoverage(" + "coverage)",
    "expect(" + "coverage).toMatchObject",
  ];
  const owners = readdirSync(caseDirectory)
    .filter((name) => name.endsWith(".case.ts") && name !== thisCase)
    .filter((name) => {
      const source = readFileSync(join(caseDirectory, name), "utf8");
      return aggregateMarkers.some((marker) => source.includes(marker));
    })
    .sort();

  expect(owners).toEqual([censusOwner]);
});
