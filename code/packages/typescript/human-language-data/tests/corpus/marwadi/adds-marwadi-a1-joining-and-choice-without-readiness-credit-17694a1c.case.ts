import { expect, it } from "vitest";
import {
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";
import { formatExamCoverage, measureExamCoverage } from "../../../src/exam-inventory.js";

it("adds one bounded Marwadi A1 joining-and-choice tranche without readiness credit", () => {
  const lessons = loadTrackLessons("marwadi");
  const ids = [
    "MW-C151-ar-together",
    "MW-C151-ar-choice",
    "MW-C151-pan-contrast",
    "MW-C151-kyunkai-reason",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );

  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    151,
    151,
    151,
    151,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    9430,
    9440,
    9450,
    9460,
  ]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const ar = "MW-CONJ-AR-01";
  const andFrame = "MW-GRAMMAR-COORDINATE-AND-01";
  const choiceFrame = "MW-GRAMMAR-ALTERNATIVE-CHOICE-01";
  const preferenceQuestion = "MW-FUNCTION-ASK-PREFERENCE-ALTERNATIVES-01";
  const pan = "MW-CONJ-PAN-01";
  const contrastFrame = "MW-GRAMMAR-ADVERSATIVE-PAN-01";
  const reasonFrame = "MW-GRAMMAR-CAUSAL-KYUNKAI-01";
  const introduced = [ar, andFrame, choiceFrame, preferenceQuestion, pan, contrastFrame, reasonFrame];
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"])).toEqual([
    [ar, andFrame],
    [choiceFrame, preferenceQuestion],
    [pan, contrastFrame],
    [reasonFrame],
  ]);
  for (const atom of introduced) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  for (const form of ["अर", "पण", "क्यूंकै", "चाय", "पाणी", "गरम", "ठंडो"]) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toContain("कुत्तो अर बिल्ली");
  expect(learnerCopy).toContain("चाय अर पाणी?");
  expect(learnerCopy).toContain("चाय गरम है, पण पाणी ठंडो है");
  expect(learnerCopy).toContain("बाटी गरम है क्यूंकै तावो गरम है");
  expect(learnerCopy).toMatch(/not an A1 mock|not A1 readiness/i);
  expect(learnerCopy).toContain("Linguistic Survey of India");

  const ledger = loadTrackChapters().find((track) => track.language === "marwadi");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 151)).toMatchObject({
    chapter: 151,
    payoff: {
      lesson: "MW-C151-kyunkai-reason",
      kind: "production",
      assesses: introduced,
    },
  });

  const inventory = loadExamInventory("marwadi", "A1");
  expect(inventory.points.find((point) => point.id === "MW-A1-CJ-01")?.probe).toEqual([
    ar,
    andFrame,
  ]);
  expect(inventory.points.find((point) => point.id === "MW-A1-CJ-03")?.probe).toEqual([
    ar,
    choiceFrame,
  ]);
  expect(inventory.points.find((point) => point.id === "MW-A1-CJ-04")?.probe).toEqual([
    pan,
    contrastFrame,
  ]);
  expect(inventory.points.find((point) => point.id === "MW-A1-CJ-06")?.probe).toEqual([
    "MW-LEX-C117-WHY117-05",
    reasonFrame,
  ]);
  expect(inventory.points.find((point) => point.id === "MW-A1-F3-02")?.probe).toEqual([
    choiceFrame,
    preferenceQuestion,
  ]);

  const coverage = measureExamCoverage(inventory, lessons);
  expect(coverage.enumerated).toBe(197);
  expect(coverage.covered).toBe(103);
  expect(coverage.unmapped).toBe(94);
  expect(coverage.partial).toBe(0);
  expect(coverage.byCategory["Yojak (joining and subordination)"]).toEqual({
    enumerated: 11,
    covered: 4,
  });
  for (const id of [
    "MW-A1-CJ-01",
    "MW-A1-CJ-03",
    "MW-A1-CJ-04",
    "MW-A1-CJ-06",
    "MW-A1-F3-02",
  ]) {
    expect(coverage.points.find((point) => point.id === id)).toMatchObject({
      covered: true,
      missingAtoms: [],
      unmapped: false,
    });
  }
  expect(formatExamCoverage(coverage)).toContain(
    "marwadi A1 (partial inventory): 103/197 points covered (52%)",
  );
}, 120_000);

