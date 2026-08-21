import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../src/loader.js";

describe("German task-shape inventories", () => {
  it("loads the official German A1 performance target", () => {
    const inventory = loadTaskShapeInventory("german", "A1");
    expect(inventory.target).toEqual({
      name: "Goethe-Zertifikat A1: Start Deutsch 1",
      basis: "external",
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.flatMap((section) => section.parts)).toHaveLength(11);
    expect(inventory.sections.every((section) => section.variants.length === 0)).toBe(true);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 100, passPoints: 60 });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([null, null, null, null]);
  });
});
