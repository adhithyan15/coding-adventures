import { expect, it } from "vitest";
import { measureExamCoverage } from "../../../src/exam-inventory.js";
import {
  loadEverything,
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";

it("turns owned Sanskrit market words into a gentle A1 price exchange", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C266-market-recall",
    "SA-C266-krinami",
    "SA-C266-ask-and-hear-price",
    "SA-C266-complete-transaction",
    "SA-C266-mixed-market-practice",
    "SA-C266-market-transaction-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    266,
    266,
    266,
    266,
    266,
    266,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    15760,
    15770,
    15780,
    15790,
    15800,
    15810,
  ]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const newAtoms = [
    "SA-VERB-C266-KRINAMI-01",
    "SA-PATTERN-C266-MARKET-EXCHANGE-02",
  ];
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"])).toEqual([
    [],
    [newAtoms[0]],
    [newAtoms[1]],
    [],
    [],
    [],
  ]);
  for (const atom of newAtoms) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }

  const evaluativeAtoms = [
    "SA-LEX-C74-MARKET-01",
    "SA-LEX-C74-MARKET-02",
    "SA-LEX-C74-MARKET-03",
    "SA-LEX-C87-NUMBERS-03",
    "SA-LEX-C138-ACT138-01",
    "SA-LEX-C185-WHY185-01",
    "SA-LEX-C215-QUAL215-05",
    "SA-LEX-C248-THG248-03",
    ...newAtoms,
  ];
  const shoppingAtoms = [
    "SA-LEX-C74-MARKET-01",
    "SA-LEX-C74-MARKET-03",
    "SA-LEX-C74-MARKET-04",
    "SA-LEX-C74-MARKET-05",
    "SA-LEX-C24-ASK-03",
    "SA-LEX-C87-NUMBERS-03",
    "SA-LEX-C185-WHY185-01",
    "SA-LEX-C222-THG222-03",
    "SA-LEX-C222-THG222-04",
    "SA-LEX-C248-THG248-03",
    "SA-LEX-DHANYAVADA",
    "SA-GRAMMAR-C265-DADATU-01",
    "SA-PATTERN-C265-POLITE-ORDER-02",
    ...newAtoms,
  ];
  const pointAtoms = [...new Set([...evaluativeAtoms, ...shoppingAtoms])];
  for (const finalLesson of chapterLessons.slice(-2)) {
    expect(finalLesson?.frontmatter["requires.knowledge"]).toEqual(
      expect.arrayContaining(pointAtoms),
    );
    expect(finalLesson?.frontmatter["practises.knowledge"]).toEqual(
      expect.arrayContaining(pointAtoms),
    );
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  for (const form of [
    "क्रीणाति",
    "क्रीणामि",
    "मूल्यं",
    "रूप्यकाणि",
    "आपणः",
    "आपणिकः",
    "धन्यवादः",
  ]) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toContain("first-person singular present");
  expect(learnerCopy).toContain("not a complete present paradigm");
  expect(learnerCopy).toContain("No Sanskrit model is shown before you answer");

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 266)).toMatchObject({
    chapter: 266,
    payoff: {
      lesson: "SA-C266-market-transaction-retrieval",
      kind: "production",
      assesses: pointAtoms,
    },
  });

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-EV-02")?.probe).toEqual(
    evaluativeAtoms,
  );
  expect(inventory.points.find((point) => point.id === "SA-A1-M-09")?.probe).toEqual(
    shoppingAtoms,
  );
  const coverage = measureExamCoverage(inventory, loadEverything().lessons);
  expect(coverage).toMatchObject({
    enumerated: 164,
    covered: 156,
    unmapped: 8,
    partial: 0,
  });
  expect(coverage.byCategory["Evaluative notions"]).toEqual({ enumerated: 4, covered: 4 });
  expect(coverage.byCategory[
    "Modern life — fields a spoken revival needs and a classical corpus lacks"
  ]).toEqual({ enumerated: 15, covered: 11 });
  for (const id of ["SA-A1-EV-02", "SA-A1-M-09"]) {
    expect(coverage.points.find((point) => point.id === id)).toMatchObject({
      covered: true,
      missingAtoms: [],
      unmapped: false,
    });
  }
}, 120_000);
