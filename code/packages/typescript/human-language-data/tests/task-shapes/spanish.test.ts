import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../src/loader.js";

describe("Spanish task-shape inventories", () => {
  it("loads the official Spanish A1 performance target and its grouped pass rule", () => {
    const inventory = loadTaskShapeInventory("spanish", "A1");
    expect(inventory.target).toEqual({ name: "DELE A1", basis: "external" });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.flatMap((section) => section.parts)).toHaveLength(13);
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 95,
      speakingMinutes: 10,
      speakingPreparationMinutes: 10,
    });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([null, null, null, null]);

    const reading = inventory.sections.find((section) => section.skill === "reading");
    expect(reading?.parts.map((part) => part.items)).toEqual([5, 6, 6, 8]);
    const listening = inventory.sections.find((section) => section.skill === "listening");
    expect(listening?.parts.map((part) => part.replayCount)).toEqual([2, 2, 2, 2]);
    const writing = inventory.sections.find((section) => section.skill === "writing");
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 15, maximum: 25, approximate: false },
      { unit: "words", minimum: 30, maximum: 40, approximate: false },
    ]);
  });
});
