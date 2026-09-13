import { expect, it } from "vitest";
import { listTaskShapeInventories, loadLessons, loadTaskShapeInventory } from "../../src/loader.js";
import { measureReadingReach } from "../../src/reading-reach.js";

it("names a consequential real ladder and still does not borrow it", () => {
  const inventory = loadTaskShapeInventory("kannada", "pre-A1");
  expect(inventory.target).toEqual({
    name: "Coding Adventures Kannada pre-A1 Assessment — project-defined equivalent",
    basis: "project-defined",
  });
  expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);

  // The Kannada Sahitya Parishat's four exams are not a hobbyist certificate:
  // clearing them exempts a state government employee from their department's
  // own Kannada test. The reason they are not this book's target is the
  // AUDIENCE -- people already living in the language who need to certify
  // literacy in it -- not a lack of seriousness, and the note says which.
  expect(inventory.passRule.note).toContain("Kannada Sahitya Parishat");
  expect(inventory.passRule.note).toContain("exempts");
  expect(inventory.passRule.note).toContain("INSIDE Karnataka");
  expect(inventory.sources.map((source) => source.id)).toContain("ksp-kannada-exams");
});

it("scores the subscript conjunct and the curve the palm leaf put there", () => {
  const inventory = loadTaskShapeInventory("kannada", "pre-A1");
  const criteria = inventory.sections.flatMap((section) =>
    section.parts.flatMap((part) => part.scoring.criteria));

  // Kannada hangs a reduced second consonant BELOW the base letter, so a line
  // occupies three tiers; the corpus builds 1,488 of them.
  expect(criteria).toContain("subscript ottakshara recognition");
  expect(criteria).toContain("subscript ottakshara below the base letter");

  // The rounded letterform is not a style. Kannada rounds nearly every letter
  // because the script was cut into palm leaf and a straight stroke splits a
  // leaf along its grain -- and rounded shapes are easier to tell apart at a
  // glance, which is most of what reading a word cold is. A hand that
  // straightens them has made the page harder to read, so it is scored.
  expect(criteria).toContain("rounded letterform");
});

it("declares a published stimulus length, so reading reach can measure it rather than skip it", () => {
  const inventories = listTaskShapeInventories().map(({ language, level }) =>
    loadTaskShapeInventory(language, level));
  const report = measureReadingReach(loadLessons(), inventories);
  const row = report.rows.find((r) => r.language === "kannada" && r.level === "pre-A1");
  expect(row?.status).toBe("measurable");
  expect(row?.partsMeasurable).toBe(3);
});
