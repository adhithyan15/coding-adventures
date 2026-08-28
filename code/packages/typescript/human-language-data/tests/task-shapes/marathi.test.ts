import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../src/loader.js";

describe("Marathi task-shape inventories", () => {
  it("makes the project-defined pre-A1 four-skill envelope executable", () => {
    const inventory = loadTaskShapeInventory("marathi", "pre-A1");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Marathi pre-A1 Assessment — project-defined equivalent",
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
    expect(inventory.sections.map((section) => section.minutes)).toEqual([10, 10, 12, 8]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [6, 4, 4],
      [6, 4, 4],
      [2, 4, 2],
      [2, 3, 1],
    ]);

    const paperPoints = inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0),
    );
    expect(paperPoints).toEqual([100, 100, 100, 100]);
    expect(paperPoints.reduce((sum, points) => sum + points, 0)).toBe(
      inventory.passRule.maximumPoints,
    );
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([
      0.6,
      0.6,
      0.6,
      0.6,
    ]);
  });

  it("keeps scored writing productive and removes copyable supports", () => {
    const inventory = loadTaskShapeInventory("marathi", "pre-A1");
    const writing = inventory.sections.find((section) => section.skill === "writing");

    expect(writing?.parts.map((part) => part.id)).toEqual([
      "prea1-writing-delayed-recall",
      "prea1-writing-dictation",
      "prea1-writing-bounded-production",
    ]);
    expect(writing?.parts.map((part) => part.responseModes)).toEqual([
      ["delayed handwritten recall"],
      ["handwritten dictation", "Devanagari transcription"],
      ["independent handwritten word or memorized chunk"],
    ]);
    expect(writing?.parts.every((part) =>
      part.aids.forbidden.some((aid) => /model|tracing/i.test(aid)),
    )).toBe(true);
  });

  it("makes the project-defined A1 destination an exact four-paper contract", () => {
    const inventory = loadTaskShapeInventory("marathi", "A1");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Marathi A1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 58,
      speakingMinutes: 10,
      speakingPreparationMinutes: 5,
    });
    expect(inventory.sections.map((section) => section.minutes)).toEqual([20, 18, 20, 10]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.id))).toEqual([
      ["a1-reading-signs-and-forms", "a1-reading-short-messages", "a1-reading-personal-descriptions"],
      ["a1-listening-announcements", "a1-listening-short-exchanges", "a1-listening-personal-account"],
      ["a1-writing-practical-form", "a1-writing-reader-purpose-message"],
      ["a1-speaking-personal-interview", "a1-speaking-prepared-description", "a1-speaking-transactional-role-play"],
    ]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [7, 7, 6],
      [6, 6, 5],
      [1, 1],
      [5, 1, 1],
    ]);

    const [reading, listening, writing] = inventory.sections;
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(250);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(350);
    expect(reading?.parts.every((part) =>
      part.promptGenres.every((genre) => genre.includes("no source over 90 words")),
    )).toBe(true);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Marathi at 90-110 words per minute") && part.replayCount === 2,
    )).toBe(true);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "items", minimum: 6, maximum: 8, approximate: false },
      { unit: "words", minimum: 30, maximum: 40, approximate: false },
    ]);
    expect(writing?.parts.every((part) =>
      ["copyable answer model", "romanization", "dictionary", "translator", "spell-checker"]
        .every((aid) => part.aids.forbidden.includes(aid)),
    )).toBe(true);

    const paperPoints = inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0),
    );
    expect(paperPoints).toEqual([100, 100, 100, 100]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
  });
});
