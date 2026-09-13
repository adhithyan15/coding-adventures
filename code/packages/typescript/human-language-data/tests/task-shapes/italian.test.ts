import { expect, it } from "vitest";
import { listTaskShapeInventories, loadLessons, loadTaskShapeInventory } from "../../src/loader.js";
import { measureReadingReach } from "../../src/reading-reach.js";

it("aims below a REAL target rather than at a project invention", () => {
  const inventory = loadTaskShapeInventory("italian", "pre-A1");
  expect(inventory.target).toEqual({
    name: "Coding Adventures Italian pre-A1 Assessment — project-defined CILS A1 precursor",
    basis: "project-defined",
  });
  expect(inventory.sections.map((section) => section.skill)).toEqual([
    "reading",
    "listening",
    "writing",
    "speaking",
  ]);
  expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
});

it("records that CILS already refuses compensation, so this rung's rule is a MATCH and not a departure", () => {
  const inventory = loadTaskShapeInventory("italian", "pre-A1");

  // Every other rung written so far states its independent-skill rule AGAINST
  // the target exam's grain -- DELE groups its skills, the Tamil Virtual Academy
  // averages. CILS does not: 7 of 12 per part at A1 and A2, 11 of 20 from B1 up,
  // and one part below the line fails the exam. That is worth pinning precisely
  // because it is the exception; a future editor who "fixes" this note to match
  // the others would be deleting the finding.
  expect(inventory.passRule.note).toContain("NOT a departure");
  expect(inventory.passRule.note).toContain("7");
  expect(inventory.passRule.note).toContain("11");

  // CILS has a FIFTH part the four-skill model cannot hold. Recording it here is
  // what stops the omission from looking like a decision nobody made.
  expect(inventory.passRule.note).toContain("analisi delle strutture di comunicazione");
  expect(inventory.sources.map((source) => source.id)).toContain("cils-levels");
});

it("scores the two orthographic hazards Italian actually has", () => {
  const inventory = loadTaskShapeInventory("italian", "pre-A1");
  const writing = inventory.sections.find((section) => section.skill === "writing");
  const criteria = writing!.parts.flatMap((part) => part.scoring.criteria);

  // Italian spelling is close to phonemic, which makes this list SHORT rather
  // than absent. nono/nonno and casa/cassa differ in nothing but how long one
  // consonant is held, so a learner who does not hear length writes a different
  // word -- not a misspelling of the right one.
  expect(criteria.filter((c) => c === "double-consonant length")).toHaveLength(3);

  // Italian marks stress only on a final stressed vowel, and only there.
  expect(criteria).toContain("graphic accent on a final stressed vowel");
});

it("declares a published stimulus length, so reading reach can measure it rather than skip it", () => {
  const inventories = listTaskShapeInventories().map(({ language, level }) =>
    loadTaskShapeInventory(language, level));
  const report = measureReadingReach(loadLessons(), inventories);
  const row = report.rows.find((r) => r.language === "italian" && r.level === "pre-A1");
  expect(row?.status).toBe("measurable");
});
