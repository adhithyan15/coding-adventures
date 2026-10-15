import { expect, it } from "vitest";
import { measureExamCoverage } from "../../../src/exam-inventory.js";
import {
  loadEverything,
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";

it("turns owned Sanskrit meal words into a gentle A1 order", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C265-meal-and-restaurant-recall",
    "SA-C265-dadatu",
    "SA-C265-give-me-an-item",
    "SA-C265-polite-order-contrast",
    "SA-C265-mixed-order-practice",
    "SA-C265-restaurant-order-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    265,
    265,
    265,
    265,
    265,
    265,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    15700,
    15710,
    15720,
    15730,
    15740,
    15750,
  ]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const newAtoms = [
    "SA-GRAMMAR-C265-DADATU-01",
    "SA-PATTERN-C265-POLITE-ORDER-02",
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

  const pointAtoms = [
    "SA-LEX-C97-LOC97-02",
    "SA-LEX-C160-THG160-01",
    "SA-LEX-KHADATI-EAT",
    "SA-LEX-C29-COURTESY-01",
    "SA-PRON-DATIVE-ME-01",
    "SA-LEX-C72-HOUSEWORK-05",
    "SA-LEX-ANNAM-FOOD",
    "SA-LEX-PANIYAM-WATER",
    "SA-LEX-KSHIRAM-MILK",
    ...newAtoms,
  ];
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
  for (const form of ["ददाति", "ददातु", "कृपया", "मह्यम्", "भोजनम्", "भोजनालयः"]) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toContain("third-person singular imperative");
  expect(learnerCopy).toContain("not a complete imperative paradigm");
  expect(learnerCopy).toContain("No Sanskrit model is shown before you answer");

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 265)).toMatchObject({
    chapter: 265,
    payoff: {
      lesson: "SA-C265-restaurant-order-retrieval",
      kind: "production",
      assesses: pointAtoms,
    },
  });

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-M-02")?.probe).toEqual(
    pointAtoms,
  );
  const coverage = measureExamCoverage(inventory, loadEverything().lessons);
  expect(coverage).toMatchObject({
    enumerated: 164,
    covered: 156,
    unmapped: 8,
    partial: 0,
  });
  expect(coverage.byCategory[
    "Modern life — fields a spoken revival needs and a classical corpus lacks"
  ]).toEqual({ enumerated: 15, covered: 11 });
  expect(coverage.points.find((point) => point.id === "SA-A1-M-02")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
