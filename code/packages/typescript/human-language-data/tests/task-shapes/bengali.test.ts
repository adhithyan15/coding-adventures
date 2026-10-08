import { describe, expect, it } from "vitest";
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

describe("Bengali A2 task shapes", () => {
  it("defines the project-owned four-skill envelope with independent papers", () => {
    const inventory = loadTaskShapeInventory("bengali", "A2");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Bengali A2 Assessment — project-defined equivalent",
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
    const inventory = loadTaskShapeInventory("bengali", "A2");
    const [reading, listening, writing] = inventory.sections;

    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(550);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(750);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Bengali at 110-130 words per minute") && part.replayCount === 2
    )).toBe(true);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 25, maximum: 35, approximate: false },
      { unit: "words", minimum: 70, maximum: 90, approximate: false },
    ]);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
  });

  it("keeps A2 directions and independent writing in Bengali script", () => {
    const inventory = loadTaskShapeInventory("bengali", "A2");
    const writtenParts = inventory.sections
      .flatMap((section) => section.parts)
      .filter((part) => part.promptModes.some((mode) => mode.startsWith("written-")));
    const writing = inventory.sections.find((section) => section.skill === "writing");

    expect(writtenParts.every((part) =>
      part.promptModes.every((mode) => !mode.startsWith("written-") || mode.includes("bengali"))
    )).toBe(true);
    expect(writing?.parts.every((part) =>
      part.responseModes.some((mode) => mode.includes("Bengali script"))
      && part.scoring.criteria.includes("matra-line, hasanta, and vowel-sign control")
      && part.aids.forbidden.includes("copyable answer model")
      && part.aids.forbidden.includes("romanization")
      && part.aids.forbidden.includes("non-Bengali-script response")
    )).toBe(true);
  });
});

describe("Bengali B1 task shapes", () => {
  it("defines the project-owned four-skill envelope with independent papers", () => {
    const inventory = loadTaskShapeInventory("bengali", "B1");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Bengali B1 Assessment — project-defined equivalent",
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
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
  });

  it("pins B1 reading, listening, writing, and scoring boundaries", () => {
    const inventory = loadTaskShapeInventory("bengali", "B1");
    const [reading, listening, writing] = inventory.sections;

    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(1100);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(1400);
    expect(listening?.parts.map((part) => part.replayCount)).toEqual([2, 2, 1, 1]);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Bengali at 130-150 words per minute")
    )).toBe(true);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 50, maximum: 70, approximate: false },
      { unit: "words", minimum: 130, maximum: 170, approximate: false },
    ]);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
  });

  it("keeps B1 directions and independent writing in Bengali script", () => {
    const inventory = loadTaskShapeInventory("bengali", "B1");
    const writtenParts = inventory.sections
      .flatMap((section) => section.parts)
      .filter((part) => part.promptModes.some((mode) => mode.startsWith("written-")));
    const writing = inventory.sections.find((section) => section.skill === "writing");

    expect(writtenParts.every((part) =>
      part.promptModes.every((mode) => !mode.startsWith("written-") || mode.includes("bengali"))
    )).toBe(true);
    expect(writing?.parts.every((part) =>
      part.responseModes.some((mode) => mode.includes("Bengali script"))
      && part.scoring.criteria.includes("matra-line, hasanta, and vowel-sign control")
      && part.aids.forbidden.includes("copyable answer model")
      && part.aids.forbidden.includes("romanization")
      && part.aids.forbidden.includes("non-Bengali-script response")
    )).toBe(true);
  });
});

describe("Bengali B2 task shapes", () => {
  it("defines the project-owned four-skill envelope with independent papers", () => {
    const inventory = loadTaskShapeInventory("bengali", "B2");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Bengali B2 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 165,
      speakingMinutes: 18,
      speakingPreparationMinutes: 10,
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.map((section) => section.minutes)).toEqual([60, 45, 60, 18]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [8, 8, 8, 8],
      [7, 7, 7, 7],
      [1, 1],
      [1, 1, 5],
    ]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
  });

  it("pins B2 lengths, single-play listening, regional voices, and scoring", () => {
    const inventory = loadTaskShapeInventory("bengali", "B2");
    const [reading, listening, writing] = inventory.sections;

    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(1800);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(2300);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Bengali at 150-170 words per minute")
      && part.replayCount === 1
    )).toBe(true);
    expect(listening?.parts.filter((part) =>
      part.promptModes.some((mode) => mode.includes("documented regional Bengali voice"))
    )).toHaveLength(2);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 100, maximum: 130, approximate: false },
      { unit: "words", minimum: 220, maximum: 280, approximate: false },
    ]);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
  });

  it("keeps B2 directions and independent writing in Bengali script", () => {
    const inventory = loadTaskShapeInventory("bengali", "B2");
    const writtenParts = inventory.sections
      .flatMap((section) => section.parts)
      .filter((part) => part.promptModes.some((mode) => mode.startsWith("written-")));
    const writing = inventory.sections.find((section) => section.skill === "writing");

    expect(writtenParts.every((part) =>
      part.promptModes.every((mode) => !mode.startsWith("written-") || mode.includes("bengali"))
    )).toBe(true);
    expect(writing?.parts.every((part) =>
      part.responseModes.some((mode) => mode.includes("Bengali script"))
      && part.scoring.criteria.includes("matra-line, hasanta, and vowel-sign control")
      && part.aids.forbidden.includes("copyable answer model")
      && part.aids.forbidden.includes("romanization")
      && part.aids.forbidden.includes("non-Bengali-script response")
    )).toBe(true);
  });
});

describe("Bengali C1 task shapes", () => {
  it("defines the project-owned four-skill envelope with independent papers", () => {
    const inventory = loadTaskShapeInventory("bengali", "C1");

    expect(inventory.target).toEqual({
      name: "Coding Adventures Bengali C1 Assessment — project-defined equivalent",
      basis: "project-defined",
    });
    expect(inventory.administration).toMatchObject({
      writtenMinutes: 200,
      speakingMinutes: 22,
      speakingPreparationMinutes: 15,
    });
    expect(inventory.sections.map((section) => section.skill)).toEqual([
      "reading",
      "listening",
      "writing",
      "speaking",
    ]);
    expect(inventory.sections.map((section) => section.minutes)).toEqual([75, 50, 75, 22]);
    expect(inventory.sections.map((section) => section.parts.map((part) => part.items))).toEqual([
      [9, 9, 9, 9],
      [7, 7, 7, 7],
      [1, 1],
      [1, 1, 6],
    ]);
    expect(Object.values(inventory.passRule.independentSkillThresholds)).toEqual([0.6, 0.6, 0.6, 0.6]);
    expect(inventory.passRule).toMatchObject({ maximumPoints: 400, passPoints: 240 });
  });

  it("pins C1 lengths, listening, regional voices, and scoring", () => {
    const inventory = loadTaskShapeInventory("bengali", "C1");
    const [reading, listening, writing] = inventory.sections;

    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.minimum ?? 0), 0)).toBe(2800);
    expect(reading?.parts.reduce((sum, part) => sum + (part.stimulusLength?.maximum ?? 0), 0)).toBe(3500);
    expect(listening?.parts.every((part) =>
      part.promptModes.includes("recorded Bengali at 160-185 words per minute with natural variation")
      && part.replayCount === 1
    )).toBe(true);
    expect(listening?.parts.filter((part) =>
      part.promptModes.some((mode) => mode.includes("documented regional Bengali voice"))
    )).toHaveLength(2);
    expect(writing?.parts.map((part) => part.responseLength)).toEqual([
      { unit: "words", minimum: 180, maximum: 220, approximate: false },
      { unit: "words", minimum: 300, maximum: 380, approximate: false },
    ]);
    expect(inventory.sections.map((section) =>
      section.parts.reduce((sum, part) => sum + (part.scoring.maxRawPoints ?? 0), 0)
    )).toEqual([100, 100, 100, 100]);
  });

  it("keeps C1 directions and independent writing in Bengali script", () => {
    const inventory = loadTaskShapeInventory("bengali", "C1");
    const writtenParts = inventory.sections
      .flatMap((section) => section.parts)
      .filter((part) => part.promptModes.some((mode) => mode.startsWith("written-")));
    const writing = inventory.sections.find((section) => section.skill === "writing");

    expect(writtenParts.every((part) =>
      part.promptModes.every((mode) => !mode.startsWith("written-") || mode.includes("bengali"))
    )).toBe(true);
    expect(writing?.parts.every((part) =>
      part.responseModes.some((mode) => mode.includes("Bengali script"))
      && part.scoring.criteria.includes("matra-line, hasanta, and vowel-sign control")
      && part.aids.forbidden.includes("copyable answer model")
      && part.aids.forbidden.includes("romanization")
      && part.aids.forbidden.includes("non-Bengali-script response")
    )).toBe(true);
  });
});
