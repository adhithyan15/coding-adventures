import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../src/loader.js";

describe("French task-shape inventories", () => {
  it("loads the official French A1 performance target without flattening its forms", () => {
    const inventory = loadTaskShapeInventory("french", "A1");
    expect(inventory.target).toEqual({
      name: "DELF A1 tout public",
      basis: "external",
    });
    expect(inventory.sections.flatMap((section) => section.parts)).toHaveLength(14);
    expect(inventory.administration.speakingMinutes).toEqual({ minimum: 5, maximum: 7 });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.2, 0.2, 0.2, 0.2]);

    const listening = inventory.sections.find((section) => section.skill === "listening");
    expect(listening?.variants.map((variant) => variant.partIds.length)).toEqual([4, 5]);

    const writing = inventory.sections.find((section) => section.skill === "writing");
    expect(writing?.parts[1]?.responseLength).toEqual({
      unit: "words",
      minimum: 40,
      maximum: null,
      approximate: false,
    });

    const speaking = inventory.sections.find((section) => section.skill === "speaking");
    expect(speaking?.parts.map((part) => part.scoring.maxRawPoints)).toEqual([4, 4, 4]);
  });

  it("loads DILF as French's sourced pre-A1 target without inventing per-skill floors", () => {
    const inventory = loadTaskShapeInventory("french", "pre-A1");
    expect(inventory.target).toEqual({ name: "DILF A1.1", basis: "external" });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.map((section) => section.minutes)).toEqual([25, 25, 15, 10]);
    expect(inventory.sections.flatMap((section) => section.parts)).toHaveLength(17);
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 65,
      speakingMinutes: 10,
      speakingGroupMaximum: 1,
      speakingPreparationMinutes: null,
    });
    expect(inventory.passRule).toMatchObject({
      maximumPoints: 100,
      passPoints: 50,
      requiresEverySectionAttempted: false,
    });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([null, null, null, null]);
    expect(inventory.sections.find((section) => section.skill === "speaking")?.parts.map((part) => part.id)).toEqual([
      "speaking-price-transaction",
      "speaking-present-person-or-place",
      "speaking-express-need-or-request-information",
      "speaking-describe-health-problem",
    ]);
  });
});
