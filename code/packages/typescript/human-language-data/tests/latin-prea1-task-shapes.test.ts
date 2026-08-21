import { describe, expect, it } from "vitest";
import { loadTaskShapeInventory } from "../src/loader.js";

describe("Latin pre-A1 task-shape inventory", () => {
  it("makes the project-defined four-skill bridge executable", () => {
    const inventory = loadTaskShapeInventory("latin", "pre-A1");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Latin pre-A1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 35,
      speakingMinutes: 8,
      speakingPreparationMinutes: 0,
    });
    expect(inventory.sections.map((section) => section.minutes)).toEqual([15, 10, 10, 8]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [8, 6],
      [8],
      [2, 3, 2],
      [2, 3, 1],
    ]);

    const paperPoints = inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0),
    );
    expect(paperPoints).toEqual([100, 100, 100, 100]);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([
      0.6,
      0.6,
      0.6,
      0.6,
    ]);
  });

  it("scores productive writing while preserving Latin convention choices", () => {
    const inventory = loadTaskShapeInventory("latin", "pre-A1");
    const writing = inventory.sections.find((section) => section.skill === "writing");
    const speaking = inventory.sections.find((section) => section.skill === "speaking");

    expect(writing?.parts.map((part) => part.id)).toEqual([
      "prea1-writing-delayed-recall",
      "prea1-writing-dictation",
      "prea1-writing-bounded-production",
    ]);
    expect(writing?.parts.every((part) =>
      part.aids.forbidden.some((aid) => /model|tracing/i.test(aid)),
    )).toBe(true);
    expect(writing?.parts.flatMap((part) => part.scoring.criteria).join(" ")).toMatch(/u\/v and i\/j/);
    expect(writing?.parts.flatMap((part) => part.scoring.criteria).join(" ")).toMatch(/macrons only when explicitly tested/);
    expect(speaking?.parts.flatMap((part) => part.scoring.criteria).join(" ")).toMatch(/pronunciation model/);
  });
});
