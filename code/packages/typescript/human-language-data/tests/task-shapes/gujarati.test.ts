import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../src/loader.js";

describe("Gujarati task-shape inventories", () => {
  it("loads the project-defined Gujarati pre-A1 floor with four separate 100-point papers", () => {
    const inventory = loadTaskShapeInventory("gujarati", "pre-A1");
    expect(inventory.target).toEqual({
      name: "Coding Adventures Gujarati pre-A1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 32,
      speakingMinutes: 8,
      speakingPreparationMinutes: 0,
    });
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    expect(inventory.sections.find((section) => section.skill === "writing")?.parts.map((part) => part.id)).toEqual([
      "prea1-writing-delayed-recall",
      "prea1-writing-dictation",
      "prea1-writing-bounded-production",
    ]);
  });

  it("loads the project-defined Gujarati A1 envelope without widening its published boundaries", () => {
    const inventory = loadTaskShapeInventory("gujarati", "A1");
    expect(inventory.target).toEqual({
      name: "Coding Adventures Gujarati A1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 58,
      speakingMinutes: 10,
      speakingPreparationMinutes: 5,
    });
    expect(inventory.sections.map((section) => [section.skill, section.minutes])).toEqual([
      ["reading", 20],
      ["listening", 18],
      ["writing", 20],
      ["speaking", 10],
    ]);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);

    const reading = inventory.sections.find((section) => section.skill === "reading");
    expect(reading?.parts.reduce((sum, part) => sum + part.items, 0)).toBe(20);
    expect(Math.max(...(reading?.parts.map((part) => part.stimulusLength?.maximum ?? 0) ?? []))).toBe(140);
    expect(reading?.parts.every((part) => part.promptGenres.every((genre) => genre.includes("no source over 90 words")))).toBe(true);

    const listening = inventory.sections.find((section) => section.skill === "listening");
    expect(listening?.parts.reduce((sum, part) => sum + part.items, 0)).toBe(17);
    expect(listening?.parts.every((part) => part.replayCount === 2)).toBe(true);
    expect(listening?.parts.every((part) => part.promptModes.includes("recorded Gujarati at 90-110 words per minute"))).toBe(true);

    const writing = inventory.sections.find((section) => section.skill === "writing");
    expect(writing?.parts.map((part) => part.id)).toEqual([
      "a1-writing-practical-form",
      "a1-writing-reader-purpose-message",
    ]);
    expect(writing?.parts[1]?.responseLength).toMatchObject({ unit: "words", minimum: 30, maximum: 40 });

    const speaking = inventory.sections.find((section) => section.skill === "speaking");
    expect(speaking?.parts.map((part) => part.id)).toEqual([
      "a1-speaking-personal-interview",
      "a1-speaking-prepared-description",
      "a1-speaking-transactional-role-play",
    ]);
    expect(speaking?.parts[1]?.responseLength).toMatchObject({ unit: "seconds", minimum: 60, maximum: 60 });
  });
});
