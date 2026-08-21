import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../src/loader.js";

describe("Latin task-shape inventories", () => {
  it("loads the project-defined Latin A1 target with four independent thresholds", () => {
    const inventory = loadTaskShapeInventory("latin", "A1");
    expect(inventory.target).toEqual({
      name: "Coding Adventures Latin A1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.flatMap((section) => section.parts)).toHaveLength(12);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([25, 25, 25, 25]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 90,
      speakingMinutes: 12,
      speakingPreparationMinutes: 10,
    });
  });
});
