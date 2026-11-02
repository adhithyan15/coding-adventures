import { expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../src/loader.js";

it("pins the project-defined Chinese pre-A1 HSK precursor with independent handwriting", () => {
  const inventory = loadTaskShapeInventory("chinese", "pre-A1");
  expect(inventory.target).toEqual({
    name: "Coding Adventures Chinese pre-A1 Assessment — project-defined HSK precursor",
    basis: "project-defined",
  });
  expect(inventory.sections.map((section) => section.skill)).toEqual([
    "reading",
    "listening",
    "writing",
    "speaking",
  ]);
  expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
  expect(inventory.sections.find((section) => section.skill === "writing")?.parts.map((part) => part.id)).toEqual([
    "prea1-writing-delayed-character-recall",
    "prea1-writing-dictation-and-transcription",
    "prea1-writing-bounded-production",
  ]);
  expect(inventory.passRule.note).toContain("not an official HSK level");
});

it("pins the A1 four-skill project equivalent without claiming an official four-skill HSK Level 1", () => {
  const inventory = loadTaskShapeInventory("chinese", "A1");
  expect(inventory.target).toEqual({
    name: "Coding Adventures Chinese A1 Four-Skill Assessment — HSK 3.0 Level 1-aligned project equivalent",
    basis: "project-defined",
  });
  expect(inventory.sections.map((section) => section.skill)).toEqual([
    "reading", "listening", "writing", "speaking",
  ]);
  expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
  expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
  expect(inventory.passRule.note).toContain("not an official HSK 3.0 Level 1 paper");
  expect(inventory.sources.map((source) => source.id)).toContain("cti-hsk-level1-structure");
  const [reading, listening, writing, speaking] = inventory.sections;
  expect(inventory.administration).toMatchObject({
    writtenMinutes: 47, speakingMinutes: 8, speakingPreparationMinutes: 2,
  });
  expect([reading?.minutes, listening?.minutes, writing?.minutes, speaking?.minutes]).toEqual([17, 15, 15, 8]);
  expect(reading?.parts.reduce((sum, part) => sum + (part.items ?? 0), 0)).toBe(20);
  expect(reading?.parts.map((part) => part.stimulusLength)).toEqual([
    { unit: "items", minimum: 2, maximum: 10, approximate: false },
    { unit: "items", minimum: 12, maximum: 30, approximate: false },
  ]);
  expect(reading?.parts.every((part) => part.notPublished.some((note) =>
    note.includes("Han characters per prompt, not questions"),
  ))).toBe(true);
  expect(listening?.parts.reduce((sum, part) => sum + (part.items ?? 0), 0)).toBe(20);
  expect(reading?.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)).toBe(100);
  expect(listening?.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)).toBe(100);
  expect(writing?.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)).toBe(100);
  expect(speaking?.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)).toBe(100);
  expect(writing?.parts.map((part) => part.id)).toEqual([
    "a1-writing-practical-form", "a1-writing-short-message",
  ]);
  expect(speaking?.parts.map((part) => part.id)).toEqual([
    "a1-speaking-personal-exchange", "a1-speaking-rehearsed-role-play",
  ]);
  for (const section of [writing, speaking]) {
    expect(section?.parts.every((part) =>
      part.sourceIds.includes("ca-chinese-assessment") &&
      !part.sourceIds.includes("cti-hsk-level1-structure"),
    )).toBe(true);
  }
  expect(listening?.parts.every((part) =>
    part.notPublished.some((unknown) => /official.*speed/i.test(unknown)),
  )).toBe(true);
});
