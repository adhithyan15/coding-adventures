import { expect, it } from "vitest";
import { listTaskShapeInventories, loadLessons, loadTaskShapeInventory } from "../../src/loader.js";
import { measureReadingReach } from "../../src/reading-reach.js";

it("states its own rule beside the richest real Indic ladder rather than borrowing it", () => {
  const inventory = loadTaskShapeInventory("telugu", "pre-A1");
  expect(inventory.target).toEqual({
    name: "Coding Adventures Telugu pre-A1 Assessment — project-defined equivalent",
    basis: "project-defined",
  });
  expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);

  // SiliconAndhra ManaBadi's eight-level ladder, examined by Potti Sreeramulu
  // Telugu University, tests FIVE components -- reading, writing, listening,
  // speaking and comprehension -- which is more than this contract has. The
  // reason it is still not the target is the one thing a contract cannot
  // borrow: it publishes no per-skill pass rule. Pinning that keeps the note
  // from being flattened into the generic "no external exam exists", which
  // would be false here.
  expect(inventory.passRule.note).toContain("Potti Sreeramulu Telugu University");
  expect(inventory.passRule.note).toContain("more");
  expect(inventory.passRule.note).toContain("no published per-skill pass rule");
  expect(inventory.sources.map((source) => source.id)).toContain("manabadi-pstu");
});

it("scores the subscript consonant, which is the thing a Latin-script hand has no practice at", () => {
  const inventory = loadTaskShapeInventory("telugu", "pre-A1");
  const criteria = inventory.sections.flatMap((section) =>
    section.parts.flatMap((part) => part.scoring.criteria));

  // Telugu builds a conjunct by hanging a reduced second consonant BELOW the
  // base letter rather than beside it, so a line occupies three tiers and the
  // reader's eye has to look down as well as along. Writing the vattu beside
  // the base is an error with no Latin-script analogue, so the reading part
  // scores recognising one and the dictation part scores placing one.
  expect(criteria).toContain("subscript vattu recognition");
  expect(criteria).toContain("subscript vattu below the base letter");

  // The talakattu is what makes a line of Telugu read as a line at all.
  expect(criteria).toContain("talakattu drawn");
});

it("declares a published stimulus length, so reading reach can measure it rather than skip it", () => {
  const inventories = listTaskShapeInventories().map(({ language, level }) =>
    loadTaskShapeInventory(language, level));
  const report = measureReadingReach(loadLessons(), inventories);
  const row = report.rows.find((r) => r.language === "telugu" && r.level === "pre-A1");
  expect(row?.status).toBe("measurable");
  expect(row?.partsMeasurable).toBe(3);
});
