import { describe, expect, it } from "vitest";
import {
  listTaskShapeInventories,
  loadLessons,
  loadTaskShapeInventory,
} from "../../../src/loader.js";
import { measureReadingReach } from "../../../src/reading-reach.js";

describe("Telugu B2 task shapes", () => {
  it("makes the project-owned four-skill envelope executable", () => {
    const inventory = loadTaskShapeInventory("telugu", "B2");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Telugu B2 Assessment — project-defined equivalent",
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
    expect(inventory.passRule.note).toContain("project-defined");
    expect(inventory.passRule.note).toContain("not an external qualification");
    expect(inventory.passRule.note).toContain("does not establish mock readiness");
  });

  it("labels precise project choices instead of laundering them into external measurements", () => {
    const inventory = loadTaskShapeInventory("telugu", "B2");
    const parts = inventory.sections.flatMap((section) => section.parts);

    expect(inventory.sources.map((source) => source.id)).toEqual([
      "ca-hl16-assessment-policy",
      "ca-hl18-task-shapes",
      "ca-telugu-assessment",
      "coe-cefr-companion-2020",
      "manabadi-pstu",
    ]);
    expect(parts.every((part) =>
      part.notPublished.some((note) =>
        note.includes("checked external sources do not publish")
        && note.includes("project-defined")
      )
    )).toBe(true);
    expect(parts.every((part) =>
      part.aids.notPublished.some((note) =>
        note.includes("checked external sources do not publish")
        && note.includes("aid policy")
      )
    )).toBe(true);
  });

  it("preserves the project B2 length, listening, regional-voice, and scoring contract", () => {
    const inventory = loadTaskShapeInventory("telugu", "B2");
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
      part.promptModes.includes("project-defined recorded Telugu at 150-170 words per minute")
      && part.replayCount === 1
      && part.aids.allowed.includes("one unscored orienting preview")
    )).toBe(true);
    expect(listening?.parts.filter((part) =>
      part.promptModes.some((mode) => mode.includes("project-selected documented regional Telugu voice"))
    )).toHaveLength(2);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 100, maximum: 130, approximate: false },
      { unit: "words", minimum: 220, maximum: 280, approximate: false },
    ]);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
  });

  it("scores independent Telugu-script writing without importing Kannada claims", () => {
    const inventory = loadTaskShapeInventory("telugu", "B2");
    const writing = inventory.sections.find((section) => section.skill === "writing");

    expect(writing?.parts.every((part) =>
      part.responseModes.some((mode) => mode.includes("Telugu script"))
      && part.aids.forbidden.includes("copyable answer model")
      && part.aids.forbidden.includes("romanization")
      && part.aids.forbidden.includes("non-Telugu-script response")
      && part.aids.forbidden.includes("translator")
    )).toBe(true);
    expect(writing?.parts.every((part) => [
      "talakattu drawn",
      "vowel-sign placement",
      "subscript vattu below the base letter",
    ].every((criterion) => part.scoring.criteria.includes(criterion)))).toBe(true);
    expect(writing?.parts.every((part) =>
      !part.scoring.criteria.some((criterion) =>
        criterion.includes("ottakshara") || criterion.includes("rounded letterform")
      )
    )).toBe(true);
  });

  it("makes B2 reading measurable without claiming present attainment", () => {
    const inventories = listTaskShapeInventories().map(({ language, level }) =>
      loadTaskShapeInventory(language, level));
    const report = measureReadingReach(loadLessons(), inventories);
    const row = report.rows.find((entry) =>
      entry.language === "telugu" && entry.level === "B2");

    expect(row?.status).toBe("measurable");
    expect(row?.partsMeasurable).toBe(4);
  }, 30_000);
});
