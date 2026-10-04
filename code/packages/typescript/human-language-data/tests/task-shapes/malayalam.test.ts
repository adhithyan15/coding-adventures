import { describe, expect, it } from "vitest";
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
  // value is guarded too: the malayalam--pre-a1 reading-reach owner pins it
  // at 3, re-derived from the merged tree rather than composed from branches.
  const inventories = listTaskShapeInventories().map(({ language, level }) =>
    loadTaskShapeInventory(language, level));
  const report = measureReadingReach(loadLessons(), inventories);
  const row = report.rows.find((r) => r.language === "malayalam" && r.level === "pre-A1");
  expect(row?.status).toBe("measurable");
  expect(row?.partsMeasurable).toBe(3);
  expect(row?.partsWithinReach).toBe(3);
}, 30_000);

describe("Malayalam task shapes", () => {
  it("defines a project-owned A2 envelope with four independent papers", () => {
    const inventory = loadTaskShapeInventory("malayalam", "A2");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Malayalam A2 Assessment — project-defined equivalent",
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

  it("pins the sourced A2 timing, length, replay, and scoring boundaries", () => {
    const inventory = loadTaskShapeInventory("malayalam", "A2");
    const [reading, listening, writing] = inventory.sections;

    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(550);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(750);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Malayalam at 110-130 words per minute") && part.replayCount === 2
    )).toBe(true);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 25, maximum: 35, approximate: false },
      { unit: "words", minimum: 70, maximum: 90, approximate: false },
    ]);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
  });

  it("requires traditional-script reading and accepts either orthography for independent writing", () => {
    const inventory = loadTaskShapeInventory("malayalam", "A2");
    const reading = inventory.sections.find((section) => section.skill === "reading");
    const writing = inventory.sections.find((section) => section.skill === "writing");

    expect(reading?.parts.some((part) =>
      part.promptModes.includes("written-malayalam-traditional-orthography")
    )).toBe(true);
    expect(writing?.parts.every((part) =>
      part.responseModes.some((mode) => mode.includes("reformed or traditional Malayalam orthography"))
      && part.aids.forbidden.includes("copyable answer model")
      && part.aids.forbidden.includes("romanization")
      && part.aids.forbidden.includes("translator")
    )).toBe(true);
  });
});

describe("Malayalam B1 task shapes", () => {
  it("defines the project-owned four-skill B1 envelope", () => {
    const inventory = loadTaskShapeInventory("malayalam", "B1");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Malayalam B1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 125,
      speakingMinutes: 15,
      speakingPreparationMinutes: 10,
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.map((section) => section.minutes)).toEqual([45, 35, 45, 15]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [7, 7, 7, 7],
      [7, 6, 6, 6],
      [1, 1],
      [5, 1, 1, 4],
    ]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
  });

  it("pins B1 reading, listening, writing, and scoring boundaries", () => {
    const inventory = loadTaskShapeInventory("malayalam", "B1");
    const [reading, listening, writing] = inventory.sections;

    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(1100);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(1400);
    expect(listening?.parts.map((part) => part.replayCount)).toEqual([2, 2, 1, 1]);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Malayalam at 130-150 words per minute")
    )).toBe(true);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 50, maximum: 70, approximate: false },
      { unit: "words", minimum: 130, maximum: 170, approximate: false },
    ]);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
  });

  it("mixes reception orthographies and keeps productive writing independent", () => {
    const inventory = loadTaskShapeInventory("malayalam", "B1");
    const reading = inventory.sections.find((section) => section.skill === "reading");
    const writing = inventory.sections.find((section) => section.skill === "writing");

    expect(reading?.parts.filter((part) =>
      part.promptModes.includes("written-malayalam-traditional-orthography")
    )).toHaveLength(2);
    expect(writing?.parts.every((part) =>
      part.responseModes.some((mode) => mode.includes("reformed or traditional Malayalam orthography"))
      && part.aids.forbidden.includes("copyable answer model")
      && part.aids.forbidden.includes("romanization")
      && part.aids.forbidden.includes("translator")
    )).toBe(true);
  });
});
