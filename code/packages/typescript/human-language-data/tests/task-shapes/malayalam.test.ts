import { expect, it } from "vitest";
import { listTaskShapeInventories, loadLessons, loadTaskShapeInventory } from "../../src/loader.js";
import { measureReadingReach } from "../../src/reading-reach.js";

it("pins the project-defined Malayalam pre-A1 target without borrowing the Malayalam Mission's name", () => {
  const inventory = loadTaskShapeInventory("malayalam", "pre-A1");
  expect(inventory.target).toEqual({
    name: "Coding Adventures Malayalam pre-A1 Assessment — project-defined equivalent",
    basis: "project-defined",
  });
  expect(inventory.sections.map((section) => section.skill)).toEqual([
    "reading",
    "listening",
    "writing",
    "speaking",
  ]);
  expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);

  // The Malayalam Mission's four flower-named stages ARE a real graded ladder for
  // exactly this book's reader. The note says why it is still not the target:
  // it certifies a course completed, not four skills measured.
  expect(inventory.passRule.note).toContain("Malayalam Mission");
  expect(inventory.passRule.note).toContain("course completed");
});

it("scores the chandrakkala for both of the jobs it does", () => {
  const inventory = loadTaskShapeInventory("malayalam", "pre-A1");
  const writing = inventory.sections.find((section) => section.skill === "writing");
  const criteria = writing!.parts.flatMap((part) => part.scoring.criteria);

  // Inside a word the chandrakkala joins two consonants; at the end of a word it
  // is the half-u. Only the second is at risk in dictation, because a careless
  // ear loses nothing audible by dropping it -- so it is scored by name there
  // rather than folded into a general "orthographic control".
  expect(criteria).toContain("chandrakkala placement");
  expect(criteria).toContain("word-final chandrakkala");

  // Both the traditional and the reformed orthography are accepted, so a conjunct
  // cannot be marked right or wrong on its own; what is scored is whether one
  // response sticks to one convention.
  expect(criteria).toContain("conjunct consistency");

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
  // checkable. Chapter 69's passage landed while this branch was open, so the
  // value is guarded too: core/reading-reach-floor.json pins malayalam/pre-A1
  // at 3, re-derived from the merged tree rather than composed from branches.
  const inventories = listTaskShapeInventories().map(({ language, level }) =>
    loadTaskShapeInventory(language, level));
  const report = measureReadingReach(loadLessons(), inventories);
  const row = report.rows.find((r) => r.language === "malayalam" && r.level === "pre-A1");
  expect(row?.status).toBe("measurable");
  expect(row?.partsMeasurable).toBe(3);
  expect(row?.partsWithinReach).toBe(3);
});
