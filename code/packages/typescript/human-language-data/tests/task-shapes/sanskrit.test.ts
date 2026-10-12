import { describe, expect, it } from "vitest";
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

describe("Sanskrit A2 task shapes", () => {
  it("defines the project-owned four-skill envelope with independent papers", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "A2");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Sanskrit A2 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 85,
      speakingMinutes: 12,
      speakingPreparationMinutes: 5,
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.map((section) => section.minutes)).toEqual([30, 25, 30, 12]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [8, 8, 8],
      [7, 7, 6],
      [1, 1],
      [6, 1, 1],
    ]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
  });

  it("pins the A2 timing, length, replay, and scoring boundaries", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "A2");
    const [reading, listening, writing] = inventory.sections;

    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(550);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(750);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Sanskrit at 110-130 words per minute") && part.replayCount === 2
    )).toBe(true);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 25, maximum: 35, approximate: false },
      { unit: "words", minimum: 70, maximum: 90, approximate: false },
    ]);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
  });

  it("makes the A2 external-sandhi boundary executable without claiming Vedic scope", () => {
    const inventory = loadTaskShapeInventory("sanskrit", "A2");
    const reading = inventory.sections.find((section) => section.skill === "reading");
    const writing = inventory.sections.find((section) => section.skill === "writing");

    expect(reading?.parts.every((part) =>
      part.promptModes.includes("written-devanagari with external sandhi")
      && part.scoring.criteria.includes("external-sandhi decoding")
    )).toBe(true);
    expect(writing?.parts.every((part) =>
      part.responseModes.some((mode) => mode.includes("Devanagari with external sandhi"))
      && part.scoring.criteria.includes("external sandhi")
      && part.scoring.criteria.includes("word-final visarga or anusvara")
      && part.scoring.criteria.includes("continuous śirorekhā and word-boundary control")
      && part.aids.forbidden.includes("copyable answer model")
      && part.aids.forbidden.includes("romanization")
      && part.aids.forbidden.includes("non-Devanagari response")
    )).toBe(true);
    expect(inventory.passRule.note).toContain("external sandhi");
    expect(inventory.passRule.note).toContain("Vedic accent and morphology remain out of scope");
  });
});
