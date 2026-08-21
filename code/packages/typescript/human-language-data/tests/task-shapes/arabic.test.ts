import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../src/loader.js";

describe("Arabic task-shape inventories", () => {
  it("loads the external Arabic STAMP 4S target with a four-skill project A1 floor", () => {
    const inventory = loadTaskShapeInventory("arabic", "A1");
    expect(inventory.target).toEqual({
      name: "Avant STAMP 4S Arabic (Modern Standard) — Level 3 / Novice-High project A1 floor",
      basis: "external",
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.flatMap((section) => section.parts)).toHaveLength(4);
    expect(inventory.administration).toMatchObject({
      writtenMinutes: { minimum: 90, maximum: 105 },
      speakingMinutes: { minimum: 20, maximum: 25 },
      speakingPreparationMinutes: null,
    });
    expect(inventory.sections.map((section) => section.parts[0]?.items)).toEqual([30, 30, 3, 3]);
    expect(inventory.sections.map((section) => section.parts[0]?.scoring.maxRawPoints)).toEqual([null, null, null, null]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([
      1 / 3,
      1 / 3,
      3 / 8,
      3 / 8,
    ]);
  });
});
