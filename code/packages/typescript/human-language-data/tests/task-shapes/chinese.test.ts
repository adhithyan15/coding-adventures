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
