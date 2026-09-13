import { expect, it } from "vitest";
import { listTaskShapeInventories, loadLessons, loadTaskShapeInventory } from "../../src/loader.js";
import { measureReadingReach } from "../../src/reading-reach.js";

it("pins the project-defined Tamil pre-A1 target without borrowing the Tamil Virtual Academy's name", () => {
  const inventory = loadTaskShapeInventory("tamil", "pre-A1");
  expect(inventory.target).toEqual({
    name: "Coding Adventures Tamil pre-A1 Assessment — project-defined equivalent",
    basis: "project-defined",
  });
  expect(inventory.sections.map((section) => section.skill)).toEqual([
    "reading",
    "listening",
    "writing",
    "speaking",
  ]);
  expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);

  // The TVA rule is cited BECAUSE this contract departs from it. A note that only
  // said "each skill passes independently" would read as a project default; saying
  // which real rule it refuses is what makes the choice legible to the next author.
  expect(inventory.passRule.note).toContain("Tamil Virtual Academy");
  expect(inventory.passRule.note).toContain("averages");
  expect(inventory.sources.map((source) => source.id)).toContain("tva-certificate-rules");

  // The TVA regulations page carries no publication date. "not stated" is the
  // corpus's existing way of saying so; a fabricated date would be worse than none.
  const tva = inventory.sources.find((source) => source.id === "tva-certificate-rules");
  expect(tva?.published).toBe("not stated");
});

it("scores the two things Tamil orthography actually requires", () => {
  const inventory = loadTaskShapeInventory("tamil", "pre-A1");
  const writing = inventory.sections.find((section) => section.skill === "writing");
  const criteria = writing!.parts.flatMap((part) => part.scoring.criteria);

  // The pulli is not decoration: without it a consonant carries an inherent
  // vowel, so its absence changes the syllable rather than the style.
  expect(criteria).toContain("pulli placement");

  // Tamil writes some vowel signs to the LEFT of their consonant, some to the
  // right, and some on both sides at once. Putting one on the wrong side is the
  // commonest orthographic error a learner makes, and nothing else in the
  // criteria list would catch it.
  expect(criteria).toContain("vowel-sign side");

  expect(inventory.sections.every((section) => section.parts.every((part) =>
    part.promptModes.every((mode) => mode !== "written-devanagari")))).toBe(true);
});


it("declares a published stimulus length, so reading reach can measure it rather than skip it", () => {
  // The corpus-wide reading-reach test asserts one ROW per inventory, so this
  // track is counted the moment the file exists. What that test does not pin is
  // whether the row says anything: an inventory whose reading parts publish no
  // stimulus word count comes back "length-not-published", which is a row that
  // measures nothing. Four of the inventories in the corpus are in exactly that
  // state, honestly, because their awarding bodies do not publish the number.
  //
  // This target is project-defined, so there is no external body to be silent:
  // the number is ours to state, and stating it is what makes the reading rung
  // checkable. Chapter 84's passage landed while this branch was open, so the
  // value is guarded too: core/reading-reach-floor.json pins tamil/pre-A1 at 3,
  // re-derived from the merged tree rather than composed from two branches.
  const inventories = listTaskShapeInventories().map(({ language, level }) =>
    loadTaskShapeInventory(language, level));
  const report = measureReadingReach(loadLessons(), inventories);
  const row = report.rows.find((r) => r.language === "tamil" && r.level === "pre-A1");
  expect(row?.status).toBe("measurable");
  expect(row?.partsMeasurable).toBe(3);
  expect(row?.partsWithinReach).toBe(3);
});
