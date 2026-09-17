// The exam says how much text the candidate reads. Nothing asked whether the
// curriculum ever produces a text that long.
//
// `task-shapes/<level>.json` carries a `stimulusLength` per reading part --- DELE
// A1 hands over 150--175 words in one part and 175--210 in another --- and
// `task-shapes.ts` checks only that the number parses. So a track could sit at
// full coverage of its content inventory while its longest connected text was a
// third of what the paper puts in front of the candidate, and every gate in the
// repository would stay green. These tests close that.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  defaultCurriculumRoot,
  listTaskShapeInventories,
  loadTaskShapeInventory,
  loadEverything,
} from "../src/loader.js";
import { measureReadingReach, passageLength } from "../src/reading-reach.js";

const root = defaultCurriculumRoot();
const inventoryIds = listTaskShapeInventories();
const inventories = inventoryIds.map(({ language, level }) => loadTaskShapeInventory(language, level));
const report = measureReadingReach(loadEverything().lessons, inventories);

const floors = JSON.parse(
  readFileSync(join(root, "core/reading-reach-floor.json"), "utf8"),
) as { floors: Record<string, number> };

describe("reading reach is measured for every task shape, not a chosen few", () => {
  // The bug this guards against is not a wrong number, it is a MISSING ROW. Four
  // of the twenty-seven inventories publish no stimulus word count, and an
  // earlier draft of this measurement silently skipped them -- which reads
  // exactly like "all inventories are accounted for" while covering 85% of them.
  it("reports one row per inventory the registry enumerates", () => {
    expect(report.rows.length).toBe(inventories.length);
    const seen = report.rows.map((row) => `${row.language}/${row.level}`).sort();
    const expected = inventoryIds.map(({ language, level }) => `${language}/${level}`).sort();
    expect(seen).toEqual(expected);
  });

  it("gives every row exactly one status, so none is quietly dropped", () => {
    for (const row of report.rows) {
      expect(["measurable", "length-not-published", "no-reading-section"]).toContain(row.status);
      if (row.status === "measurable") expect(row.partsMeasurable).toBeGreaterThanOrEqual(1);
      // An excluded row must say WHY, in the board's own words. Exclusion
      // without a reason is indistinguishable from an oversight.
      if (row.status === "length-not-published") {
        expect(row.partsMeasurable).toBe(0);
        expect(row.parts.length).toBeGreaterThanOrEqual(1);
        expect(row.unpublishedReasons.length).toBeGreaterThanOrEqual(1);
      }
    }
  });

  // Without this, deleting every reading lesson in the corpus would leave the
  // suite green: each assertion above holds vacuously over an all-zero table.
  it("is not vacuous — some inventory is comparable and some track has a passage", () => {
    expect(report.rows.filter((row) => row.status === "measurable").length).toBeGreaterThanOrEqual(1);
    const withPassage = Object.entries(report.longestByTrack).filter(([, words]) => words > 0);
    expect(withPassage.length).toBeGreaterThanOrEqual(1);
  });
});

describe("the reach only ratchets upward", () => {
  it("never falls below the committed floor for any track", () => {
    for (const row of report.rows) {
      const key = `${row.language}/${row.level}`;
      const floor = floors.floors[key] ?? 0;
      expect(row.partsWithinReach, `${key} reading parts within reach`).toBeGreaterThanOrEqual(floor);
    }
  });

  // A floor naming a track that no longer exists, or a level whose task shape
  // was deleted, would sit here agreeing with nothing forever.
  it("floors only name inventories that exist", () => {
    const known = new Set(report.rows.map((row) => `${row.language}/${row.level}`));
    for (const key of Object.keys(floors.floors)) expect(known).toContain(key);
  });
});

describe("a passage is the quoted text, not the lesson around it", () => {
  it("counts only blockquote lines", () => {
    const markdown = [
      "Some prose that explains the passage and is not itself the passage.",
      "",
      "> Uno dos tres.",
      "> Cuatro cinco.",
      "",
      "More prose afterwards, also not the passage.",
    ].join("\n");
    expect(passageLength(markdown)).toBe(5);
  });

  // A numeral IS a word to a candidate and to the boards' own word counts; an
  // em-dash used as a bullet is not. Counting letters only would measure our
  // passages by a stricter rule than the exam length they are compared to.
  it("counts numerals but not letterless markers", () => {
    expect(passageLength("> uno --- dos\n> 3 tres")).toBe(4);
  });

  // A spaceless script gets counted by character, because whitespace is not a
  // unit there. This line is sixteen kana and four words; the old count saw ONE
  // token, which is indistinguishable from a single-word passage. The 、 in the
  // middle is U+3001 -- not whitespace, and correctly not counted as a sign.
  it("counts a spaceless script by character", () => {
    expect(passageLength("> もうすこし、ゆっくりいってください")).toBe(16);
  });

  // Chinese's own task shape asks for `items`, and an item is a character.
  it("counts Han characters individually", () => {
    expect(passageLength("> 你好\n> 谢谢")).toBe(4);
  });

  // Mixed: a Han character is one, a run of Latin between spaces is one.
  it("counts a mixed passage in both units", () => {
    expect(passageLength("> コーヒー 100")).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// DECLARING A1 MADE A GAP VISIBLE THAT NOTHING HAD MEASURED.
//
// Eight tracks gained `task-shapes/a1.json` in one tranche. Every one of them
// already had a reading rung and a pre-A1 inventory, and every one of them
// measured 3/3 or 2/2 at pre-A1 -- which is exactly the kind of green that
// hides a bigger number one rung up.
//
// At A1 all eight measure 0/3. The shared A1 envelope asks for a shortest input
// of 70 words, and the longest comprehension passage any of them holds is 53:
//
//     malayalam 53   sanskrit 51   italian 47   portuguese 42
//     bengali   36   kannada  31   tamil    30   telugu     25
//
// That is not a defect introduced here. It is the first time the distance was
// stated as a number rather than left as an absence, and the distance is the
// point: a track is not "nearly at A1 reading" -- it is between seventeen and
// forty-five words short, per track, and now says so.
//
// This test pins the MEASURABILITY rather than the zero. A floor of 0 asserts
// nothing, so what must not regress is that these rows exist and publish a
// stimulus length at all; the moment a passage grows past 70, its floor goes in.
// ---------------------------------------------------------------------------
describe("the A1 rung the eight new inventories opened", () => {
  const NEWLY_MEASURABLE = [
    "bengali", "italian", "kannada", "malayalam",
    "portuguese", "sanskrit", "tamil", "telugu",
  ];

  it("measures every one of them at A1 instead of leaving the rung absent", () => {
    for (const language of NEWLY_MEASURABLE) {
      const row = report.rows.find((r) => r.language === language && r.level === "A1");
      expect(row, `${language} A1 must appear in the reading-reach table`).toBeDefined();
      expect(row!.status, `${language} A1 status`).toBe("measurable");
      expect(row!.partsMeasurable, `${language} A1 measurable parts`).toBe(3);
    }
  });

  it("keeps the pre-A1 rung it already had, so the A1 zero is not a regression", () => {
    for (const language of NEWLY_MEASURABLE) {
      const row = report.rows.find((r) => r.language === language && r.level === "pre-A1");
      expect(row?.partsWithinReach, `${language} pre-A1 parts in reach`).toBeGreaterThan(0);
    }
  });
});
