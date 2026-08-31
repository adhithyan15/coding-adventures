import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../src/loader.js";

const paperPoints = (level: "pre-A1" | "A1" | "A2" | "B1" | "B2" | "C1" | "C2") =>
  loadTaskShapeInventory("hindi", level).sections.map((section) =>
    section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0),
  );

describe("Hindi task-shape inventories", () => {
  it("keeps every published rung independently four-skill and non-compensatory", () => {
    for (const level of ["pre-A1", "A1", "A2", "B1", "B2", "C1", "C2"] as const) {
      const inventory = loadTaskShapeInventory("hindi", level);
      expect(inventory.language).toBe("hindi");
      expect(inventory.level).toBe(level);
      expect(inventory.target.basis).toBe("project-defined");
      expect(inventory.sections.map((section) => section.skill)).toEqual([
        "reading",
        "listening",
        "writing",
        "speaking",
      ]);
      expect(paperPoints(level)).toEqual([100, 100, 100, 100]);
      expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
      expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([
        0.6,
        0.6,
        0.6,
        0.6,
      ]);
    }
  });

  it("pins the gentle pre-A1 and A1 destinations", () => {
    const preA1 = loadTaskShapeInventory("hindi", "pre-A1");
    expect(preA1.administration).toMatchObject({
      writtenMinutes: 32,
      speakingMinutes: 8,
      speakingPreparationMinutes: 0,
    });
    expect(preA1.sections.map((section) => section.minutes)).toEqual([10, 10, 12, 8]);

    const a1 = loadTaskShapeInventory("hindi", "A1");
    expect(a1.administration).toMatchObject({
      writtenMinutes: 58,
      speakingMinutes: 10,
      speakingPreparationMinutes: 5,
    });
    expect(a1.sections.map((section) => section.minutes)).toEqual([20, 18, 20, 10]);
    expect(a1.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [7, 7, 6],
      [6, 6, 5],
      [1, 1],
      [5, 1, 1],
    ]);
  });

  it("makes A2 an exact 30/25/30/12-minute four-paper contract", () => {
    const inventory = loadTaskShapeInventory("hindi", "A2");
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 85,
      speakingMinutes: 12,
      speakingPreparationMinutes: 5,
    });
    expect(inventory.sections.map((section) => section.minutes)).toEqual([30, 25, 30, 12]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [8, 8, 8],
      [7, 7, 6],
      [1, 1],
      [6, 1, 1],
    ]);

    const [reading, listening, writing, speaking] = inventory.sections;
    expect(reading?.parts.reduce(
      (sum, part) => sum + (part.stimulusLength?.minimum ?? 0),
      0,
    )).toBe(550);
    expect(reading?.parts.reduce(
      (sum, part) => sum + (part.stimulusLength?.maximum ?? 0),
      0,
    )).toBe(750);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Hindi at 110-130 words per minute") &&
      part.replayCount === 2
    )).toBe(true);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 25, maximum: 35, approximate: false },
      { unit: "words", minimum: 70, maximum: 90, approximate: false },
    ]);
    expect(writing?.parts.every((part) =>
      ["copyable answer model", "romanization", "dictionary", "translator", "spell-checker"]
        .every((aid) => part.aids.forbidden.includes(aid))
    )).toBe(true);
    expect(speaking?.parts[1]?.responseLength).toEqual({
      unit: "seconds",
      minimum: 90,
      maximum: 120,
      approximate: false,
    });
  });

  it("makes B1 an exact 45/35/45/15-minute four-paper contract", () => {
    const inventory = loadTaskShapeInventory("hindi", "B1");
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 125,
      speakingMinutes: 15,
      speakingPreparationMinutes: 10,
    });
    expect(inventory.sections.map((section) => section.minutes)).toEqual([45, 35, 45, 15]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [7, 7, 7, 7],
      [7, 6, 6, 6],
      [1, 1],
      [5, 1, 1, 4],
    ]);
    const [reading, listening, writing, speaking] = inventory.sections;
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(1100);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(1400);
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

  it("makes B2 an exact 60/45/60/18-minute four-paper contract", () => {
    const inventory = loadTaskShapeInventory("hindi", "B2");
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 165,
      speakingMinutes: 18,
      speakingPreparationMinutes: 10,
    });
    expect(inventory.sections.map((section) => section.minutes)).toEqual([60, 45, 60, 18]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [8, 8, 8, 8],
      [7, 7, 7, 7],
      [1, 1],
      [1, 1, 5],
    ]);
    const [reading, listening, writing, speaking] = inventory.sections;
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(1800);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(2300);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Hindi at 150-170 words per minute") &&
      part.replayCount === 1 &&
      part.aids.allowed.includes("one unscored orienting preview")
    )).toBe(true);
    expect(listening?.parts.filter((part) =>
      part.promptModes.some((mode) => mode.includes("regional Hindi voice"))
    )).toHaveLength(2);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 100, maximum: 130, approximate: false },
      { unit: "words", minimum: 220, maximum: 280, approximate: false },
    ]);
    expect(speaking?.parts[0]?.responseLength).toEqual({
      unit: "seconds",
      minimum: 210,
      maximum: 240,
      approximate: false,
    });
  });

  it("makes C1 an exact 75/50/75/22-minute four-paper contract", () => {
    const inventory = loadTaskShapeInventory("hindi", "C1");
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 200,
      speakingMinutes: 22,
      speakingPreparationMinutes: 15,
    });
    expect(inventory.sections.map((section) => section.minutes)).toEqual([75, 50, 75, 22]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [9, 9, 9, 9],
      [7, 7, 7, 7],
      [1, 1],
      [1, 1, 6],
    ]);
    const [reading, listening, writing, speaking] = inventory.sections;
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(2800);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(3500);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Hindi at 160-185 words per minute with natural variation") &&
      part.replayCount === 1
    )).toBe(true);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 180, maximum: 220, approximate: false },
      { unit: "words", minimum: 300, maximum: 380, approximate: false },
    ]);
    expect(speaking?.parts[0]?.responseLength).toEqual({
      unit: "seconds",
      minimum: 270,
      maximum: 300,
      approximate: false,
    });
    expect(speaking?.parts[0]?.aids.allowed).toContain(
      "paper notes made during fifteen-minute preparation",
    );
  });

  it("makes C2 an exact 90/60/90/25-minute four-paper contract", () => {
    const inventory = loadTaskShapeInventory("hindi", "C2");
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 240,
      speakingMinutes: 25,
      speakingPreparationMinutes: 15,
    });
    expect(inventory.sections.map((section) => section.minutes)).toEqual([90, 60, 90, 25]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [10, 10, 10, 10],
      [8, 8, 8, 8],
      [1, 1],
      [1, 1, 6],
    ]);
    const [reading, listening, writing, speaking] = inventory.sections;
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(4000);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(5000);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Hindi at a natural 165-200 words per minute with variation") &&
      part.replayCount === 1
    )).toBe(true);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 220, maximum: 280, approximate: false },
      { unit: "words", minimum: 420, maximum: 520, approximate: false },
    ]);
    expect(speaking?.parts[0]?.responseLength).toEqual({
      unit: "seconds",
      minimum: 330,
      maximum: 360,
      approximate: false,
    });
    expect(speaking?.parts[2]?.responseModes).toContain(
      "spoken defence, qualification, mediation, and audience-shift reformulation",
    );
  });
});
