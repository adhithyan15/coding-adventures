import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../src/loader.js";

describe("Persian task-shape inventories", () => {
  it("makes the project-defined pre-A1 four-skill envelope executable", () => {
    const inventory = loadTaskShapeInventory("persian", "pre-A1");
    expect(inventory.target).toEqual({
      name: "Coding Adventures Persian pre-A1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual(["reading", "listening", "writing", "speaking"]);
    expect(inventory.administration).toMatchObject({ writtenMinutes: 32, speakingMinutes: 8, speakingPreparationMinutes: 0 });
    expect(inventory.sections.map((section) => section.minutes)).toEqual([10, 10, 12, 8]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [6, 4, 4], [6, 4, 4], [2, 4, 2], [2, 3, 1],
    ]);
    const paperPoints = inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0),
    );
    expect(paperPoints).toEqual([100, 100, 100, 100]);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
  });

  it("keeps scored Persian writing productive and script-aware", () => {
    const inventory = loadTaskShapeInventory("persian", "pre-A1");
    const writing = inventory.sections.find((section) => section.skill === "writing");
    const criteria = writing?.parts.flatMap((part) => part.scoring.criteria).join(" ") ?? "";
    expect(writing?.parts.map((part) => part.id)).toEqual([
      "prea1-writing-delayed-recall", "prea1-writing-dictation", "prea1-writing-bounded-production",
    ]);
    expect(writing?.parts.every((part) => part.aids.forbidden.some((aid) => /model|tracing/i.test(aid)))).toBe(true);
    expect(criteria).toMatch(/right-to-left/);
    expect(criteria).toMatch(/joining and dot control/);
    expect(criteria).toMatch(/ZWNJ/);
    expect(writing?.parts.at(-1)?.aids.allowed.join(" ")).toMatch(/Iranian Persian forms/);
    expect(writing?.parts.at(-1)?.notPublished.join(" ")).toMatch(/short vowels are not required/);
  });
});
