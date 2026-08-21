import { expect, it } from "vitest";
import { loadTaskShapeInventory } from "../../src/loader.js";

it("pins the project-defined Japanese pre-A1 precursor without inventing a JLPT level", () => {
  const inventory = loadTaskShapeInventory("japanese", "pre-A1");
  expect(inventory.target).toEqual({
    name: "Coding Adventures Japanese pre-A1 Assessment — project-defined JLPT/JF Standard precursor",
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
    "prea1-writing-delayed-kana-recall",
    "prea1-writing-dictation",
    "prea1-writing-bounded-production",
  ]);
  expect(inventory.passRule.note).toContain("below the official JLPT");
});
