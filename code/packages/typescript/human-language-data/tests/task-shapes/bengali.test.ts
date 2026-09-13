import { expect, it } from "vitest";
import { listTaskShapeInventories, loadLessons, loadTaskShapeInventory } from "../../src/loader.js";
import { measureReadingReach } from "../../src/reading-reach.js";

it("states its own rule rather than borrowing an authority that publishes none", () => {
  const inventory = loadTaskShapeInventory("bengali", "pre-A1");
  expect(inventory.target).toEqual({
    name: "Coding Adventures Bengali pre-A1 Assessment — project-defined equivalent",
    basis: "project-defined",
  });
  expect(inventory.sections.map((section) => section.skill)).toEqual([
    "reading",
    "listening",
    "writing",
    "speaking",
  ]);
  expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);

  // The University of Dhaka's Institute of Modern Languages teaches Bangla as a
  // foreign language -- the ONE language there reserved for foreign students,
  // which makes it the closest institution in the world to this book's purpose.
  // It measures course hours and publishes no per-skill rule, so there is no
  // real pass rule here to depart from or to match. Saying so is the point:
  // an unstated absence reads as an oversight rather than as a finding.
  expect(inventory.passRule.note).toContain("no published Bengali four-skill");
  expect(inventory.sources.map((source) => source.id)).toContain("du-institute-of-modern-languages");
});

it("scores the headline stroke, which is what groups a Bengali word at all", () => {
  const inventory = loadTaskShapeInventory("bengali", "pre-A1");
  const criteria = inventory.sections.flatMap((section) =>
    section.parts.flatMap((part) => part.scoring.criteria));

  // Bengali letters hang from a horizontal line drawn across the top. A hand
  // that omits it or breaks it mid-word has not made a cosmetic error -- it has
  // removed the thing a reader's eye follows to find the word boundary, which
  // is why the reading part scores it too and not only the writing parts.
  expect(criteria).toContain("matra-line word grouping");
  expect(criteria).toContain("matra line drawn");

  // The hasanta cancels the inherent vowel and builds the conjuncts; the corpus
  // writes it 284 times, and dropping it writes a different word.
  expect(criteria).toContain("hasanta placement");
});

it("declares a published stimulus length, so reading reach can measure it rather than skip it", () => {
  const inventories = listTaskShapeInventories().map(({ language, level }) =>
    loadTaskShapeInventory(language, level));
  const report = measureReadingReach(loadLessons(), inventories);
  const row = report.rows.find((r) => r.language === "bengali" && r.level === "pre-A1");
  expect(row?.status).toBe("measurable");
  expect(row?.partsMeasurable).toBe(3);
});
