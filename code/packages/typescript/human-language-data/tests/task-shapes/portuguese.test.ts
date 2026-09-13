import { expect, it } from "vitest";
import { listTaskShapeInventories, loadLessons, loadTaskShapeInventory } from "../../src/loader.js";
import { measureReadingReach } from "../../src/reading-reach.js";

it("aims below CAPLE's ACESSO, which is A1 and not pre-A1", () => {
  const inventory = loadTaskShapeInventory("portuguese", "pre-A1");
  expect(inventory.target).toEqual({
    name: "Coding Adventures Portuguese pre-A1 Assessment — project-defined CAPLE ACESSO precursor",
    basis: "project-defined",
  });
  expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
  expect(inventory.sources.map((source) => source.id)).toContain("caple-acesso");
});

it("records WHY the target is European Portuguese, which is a ladder fact and not a preference", () => {
  const inventory = loadTaskShapeInventory("portuguese", "pre-A1");

  // This is the finding most at risk of being "corrected" by someone who reads
  // the choice as a preference between two equal varieties. It is not. Celpe-Bras
  // is the only officially recognised Brazilian Portuguese certificate and its
  // LOWEST band sits at roughly B1, so there is no Brazilian certificate at A2,
  // at A1, or below. A ladder that wants its bottom three rungs certifiable has
  // exactly one option, and that option certifies European Portuguese.
  expect(inventory.passRule.note).toContain("ONLY Portuguese certificates in the");
  expect(inventory.passRule.note).toContain("Celpe-Bras");
  expect(inventory.passRule.note).toContain("EUROPEAN");

  // And where the contract is stricter than its target it says so rather than
  // implying a CAPLE equivalence: CAPLE passes on a single 55% overall.
  expect(inventory.passRule.note).toContain("55%");
});

it("scores the two things Portuguese spelling asks for that its neighbours do not", () => {
  const inventory = loadTaskShapeInventory("portuguese", "pre-A1");
  const criteria = inventory.sections.find((s) => s.skill === "writing")!
    .parts.flatMap((part) => part.scoring.criteria);

  // pão / pao is not a misspelling of one word; it is a different word. The
  // nasal series is written with a tilde or with an m/n that is not itself
  // pronounced as a consonant, and nothing else in the list would catch it.
  expect(criteria.filter((c) => c === "nasal marking")).toHaveLength(3);
  expect(criteria).toContain("graphic accent placement");
});

it("declares a published stimulus length, so reading reach can measure it rather than skip it", () => {
  const inventories = listTaskShapeInventories().map(({ language, level }) =>
    loadTaskShapeInventory(language, level));
  const report = measureReadingReach(loadLessons(), inventories);
  const row = report.rows.find((r) => r.language === "portuguese" && r.level === "pre-A1");
  expect(row?.status).toBe("measurable");
});
