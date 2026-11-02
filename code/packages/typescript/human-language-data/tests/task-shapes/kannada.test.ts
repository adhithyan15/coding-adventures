import { describe, expect, it } from "vitest";
import { listTaskShapeInventories, loadLessons, loadTaskShapeInventory } from "../../src/loader.js";
import { measureReadingReach } from "../../src/reading-reach.js";

it("names a consequential real ladder and still does not borrow it", () => {
  const inventory = loadTaskShapeInventory("kannada", "pre-A1");
  expect(inventory.target).toEqual({
    name: "Coding Adventures Kannada pre-A1 Assessment — project-defined equivalent",
    basis: "project-defined",
  });
  expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);

  // The Kannada Sahitya Parishat's four exams are not a hobbyist certificate:
  // clearing them exempts a state government employee from their department's
  // own Kannada test. The reason they are not this book's target is the
  // AUDIENCE -- people already living in the language who need to certify
  // literacy in it -- not a lack of seriousness, and the note says which.
  expect(inventory.passRule.note).toContain("Kannada Sahitya Parishat");
  expect(inventory.passRule.note).toContain("exempts");
  expect(inventory.passRule.note).toContain("INSIDE Karnataka");
  expect(inventory.sources.map((source) => source.id)).toContain("ksp-kannada-exams");
});

it("scores the subscript conjunct and the curve the palm leaf put there", () => {
  const inventory = loadTaskShapeInventory("kannada", "pre-A1");
  const criteria = inventory.sections.flatMap((section) =>
    section.parts.flatMap((part) => part.scoring.criteria));

  // Kannada hangs a reduced second consonant BELOW the base letter, so a line
  // occupies three tiers; the corpus builds 1,488 of them.
  expect(criteria).toContain("subscript ottakshara recognition");
  expect(criteria).toContain("subscript ottakshara below the base letter");

  // The rounded letterform is not a style. Kannada rounds nearly every letter
  // because the script was cut into palm leaf and a straight stroke splits a
  // leaf along its grain -- and rounded shapes are easier to tell apart at a
  // glance, which is most of what reading a word cold is. A hand that
  // straightens them has made the page harder to read, so it is scored.
  expect(criteria).toContain("rounded letterform");
});

it("declares a published stimulus length, so reading reach can measure it rather than skip it", () => {
  const inventories = listTaskShapeInventories().map(({ language, level }) =>
    loadTaskShapeInventory(language, level));
  const report = measureReadingReach(loadLessons(), inventories);
  const row = report.rows.find((r) => r.language === "kannada" && r.level === "pre-A1");
  expect(row?.status).toBe("measurable");
  expect(row?.partsMeasurable).toBe(3);
});

describe("Kannada B2 task shapes", () => {
  it("defines the project-owned four-skill B2 envelope", () => {
    const inventory = loadTaskShapeInventory("kannada", "B2");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Kannada B2 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 165,
      speakingMinutes: 18,
      speakingPreparationMinutes: 10,
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.map((section) => section.minutes)).toEqual([60, 45, 60, 18]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [8, 8, 8, 8],
      [7, 7, 7, 7],
      [1, 1],
      [1, 1, 5],
    ]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([
      0.6,
      0.6,
      0.6,
      0.6,
    ]);
  });

  it("pins B2 length, single-play listening, regional voices, and scoring", () => {
    const inventory = loadTaskShapeInventory("kannada", "B2");
    const [reading, listening, writing] = inventory.sections;

    expect(reading?.parts.reduce(
      (sum, part) => sum + (part.stimulusLength?.minimum ?? 0),
      0,
    )).toBe(1800);
    expect(reading?.parts.reduce(
      (sum, part) => sum + (part.stimulusLength?.maximum ?? 0),
      0,
    )).toBe(2300);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Kannada at 150-170 words per minute")
      && part.replayCount === 1
      && part.aids.allowed.includes("one unscored orienting preview")
    )).toBe(true);
    expect(listening?.parts.filter((part) =>
      part.promptModes.some((mode) => mode.includes("documented regional Kannada voice"))
    )).toHaveLength(2);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 100, maximum: 130, approximate: false },
      { unit: "words", minimum: 220, maximum: 280, approximate: false },
    ]);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
  });

  it("keeps independent Kannada-script writing and scores its three-tier orthography", () => {
    const inventory = loadTaskShapeInventory("kannada", "B2");
    const writing = inventory.sections.find((section) => section.skill === "writing");

    expect(writing?.parts.every((part) =>
      part.responseModes.some((mode) => mode.includes("Kannada script"))
      && part.aids.forbidden.includes("copyable answer model")
      && part.aids.forbidden.includes("romanization")
      && part.aids.forbidden.includes("non-Kannada-script response")
      && part.aids.forbidden.includes("translator")
    )).toBe(true);
    expect(writing?.parts.every((part) => [
      "rounded letterform",
      "vowel-sign placement",
      "subscript ottakshara below the base letter",
    ].every((criterion) => part.scoring.criteria.includes(criterion)))).toBe(true);
  });

  it("makes the B2 reading target measurable without claiming present attainment", () => {
    const inventories = listTaskShapeInventories().map(({ language, level }) =>
      loadTaskShapeInventory(language, level));
    const report = measureReadingReach(loadLessons(), inventories);
    const row = report.rows.find((entry) =>
      entry.language === "kannada" && entry.level === "B2");

    expect(row?.status).toBe("measurable");
    expect(row?.partsMeasurable).toBe(4);
  }, 30_000);
});
