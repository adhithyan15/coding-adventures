import { expect, it } from "vitest";
import { listTaskShapeInventories, loadLessons, loadTaskShapeInventory } from "../../src/loader.js";
import { measureReadingReach } from "../../src/reading-reach.js";

it("gives a classical language all four skills, and records why that is not a category error", () => {
  const inventory = loadTaskShapeInventory("sanskrit", "pre-A1");
  expect(inventory.target).toEqual({
    name: "Coding Adventures Sanskrit pre-A1 Assessment — project-defined equivalent",
    basis: "project-defined",
  });
  expect(inventory.sections.map((section) => section.skill)).toEqual([
    "reading",
    "listening",
    "writing",
    "speaking",
  ]);

  // The obvious objection -- Sanskrit is classical, so test reading and writing
  // and stop -- is wrong about the world, and this is the assertion that keeps
  // the answer attached to the decision. Samskrita Bharati teaches Sanskrit
  // THROUGH conversation, and its four-year certificate is named "from
  // conversation to the shastras": speech is where the training starts, not
  // what is bolted on at the end.
  expect(inventory.passRule.note).toContain("SPEAKING paper for a classical language");
  expect(inventory.passRule.note).toContain("sambhashanatah shastraparyantam");
  expect(inventory.passRule.note).toContain("rather than a category error");
  expect(inventory.sources.map((source) => source.id)).toContain("samskrita-bharati-courses");
});

it("scores the word ending, because in Sanskrit that is where the grammar lives", () => {
  const inventory = loadTaskShapeInventory("sanskrit", "pre-A1");
  const criteria = inventory.sections.flatMap((section) =>
    section.parts.flatMap((part) => part.scoring.criteria));

  // Sanskrit's noun families are told apart by how a word ENDS, and the visarga
  // and the anusvara carry most of that. The corpus writes 1,011 visargas
  // against 103 anusvaras, so confusing them is not a spelling slip: it puts
  // the word in the wrong family and everything agreeing with it goes wrong.
  // Scored by name in every writing part AND in reading, because the reader has
  // to see the difference before the writer can make it.
  expect(criteria).toContain("word-final visarga or anusvara discrimination");
  expect(criteria.filter((c) => c === "word-final visarga or anusvara")).toHaveLength(3);

  // At pre-A1 the words are kept apart. Requiring external sandhi here would be
  // requiring a grammar the learner has not met, so the dictation part scores
  // the boundary rather than the join.
  expect(criteria).toContain("word boundary without external sandhi");
});

it("declares a published stimulus length, so reading reach can measure it rather than skip it", () => {
  const inventories = listTaskShapeInventories().map(({ language, level }) =>
    loadTaskShapeInventory(language, level));
  const report = measureReadingReach(loadLessons(), inventories);
  const row = report.rows.find((r) => r.language === "sanskrit" && r.level === "pre-A1");
  expect(row?.status).toBe("measurable");
  expect(row?.partsMeasurable).toBe(3);
});
