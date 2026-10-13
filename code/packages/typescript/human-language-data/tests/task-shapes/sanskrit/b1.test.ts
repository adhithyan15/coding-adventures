import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../../src/loader.js";

describe("Sanskrit B1 task shapes", () => {
  it("defines the project-owned four-skill envelope with independent papers", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "B1");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Sanskrit B1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 125,
      speakingMinutes: 15,
      speakingGroupMaximum: 2,
      speakingPreparationMinutes: 10,
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.map((section) => section.minutes)).toEqual([45, 35, 45, 15]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [7, 7, 7, 7],
      [7, 6, 6, 6],
      [1, 1],
      [5, 1, 1, 4],
    ]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
  });

  it("pins the B1 length, pace, replay, and production boundaries", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "B1");
    const [reading, listening, writing, speaking] = inventory.sections;

    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(1100);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(1400);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Sanskrit at 130-150 words per minute")
    )).toBe(true);
    expect(listening?.parts.map((part) => part.replayCount)).toEqual([2, 2, 1, 1]);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 50, maximum: 70, approximate: false },
      { unit: "words", minimum: 130, maximum: 170, approximate: false },
    ]);
    expect(speaking?.parts[1]?.responseLength).toEqual({
      unit: "seconds",
      minimum: 150,
      maximum: 180,
      approximate: false,
    });
  });

  it("keeps Sanskrit-specific script, sandhi, pronunciation, and scope boundaries explicit", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "B1");
    const reading = inventory.sections.find((section) => section.skill === "reading");
    const writing = inventory.sections.find((section) => section.skill === "writing");
    const speaking = inventory.sections.find((section) => section.skill === "speaking");

    expect(reading?.parts.every((part) =>
      part.promptModes.includes("written-devanagari with external sandhi")
      && part.scoring.criteria.includes("external-sandhi decoding")
    )).toBe(true);
    expect(writing?.parts.every((part) =>
      part.responseModes.some((mode) => mode.includes("Devanagari with external sandhi"))
      && part.scoring.criteria.includes("external sandhi")
      && part.scoring.criteria.includes("word-final visarga or anusvara")
      && part.scoring.criteria.includes("continuous śirorekhā and word-boundary control")
      && part.aids.forbidden.includes("copyable answer model")
      && part.aids.forbidden.includes("romanization")
      && part.aids.forbidden.includes("non-Devanagari response")
    )).toBe(true);
    expect(speaking?.parts.every((part) =>
      part.scoring.criteria.includes("pronunciation and fluency in a documented tradition")
    )).toBe(true);
    expect(inventory.passRule.note).toContain("Vedic accent and morphology remain out of scope");
    expect(inventory.passRule.note).toContain("project-defined assessment, not an external qualification");
  });
});
