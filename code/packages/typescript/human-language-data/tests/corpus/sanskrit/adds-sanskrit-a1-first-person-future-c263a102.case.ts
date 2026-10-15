import { expect, it } from "vitest";
import { measureExamCoverage } from "../../../src/exam-inventory.js";
import {
  loadEverything,
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";

it("adds a gentle Sanskrit A1 first-person future ramp", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C263-gamisyami",
    "SA-C263-first-person-future",
    "SA-C263-future-person-contrast",
    "SA-C263-tomorrow-going-recall",
    "SA-C264-karisyami",
    "SA-C264-pathisyami",
    "SA-C264-near-term-plan",
    "SA-C264-mixed-plan-practice",
    "SA-C264-first-person-future-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    263,
    263,
    263,
    263,
    264,
    264,
    264,
    264,
    264,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    15610,
    15620,
    15630,
    15640,
    15650,
    15660,
    15670,
    15680,
    15690,
  ]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const atoms = [
    "SA-VERB-C263-GAMISYAMI-01",
    "SA-GRAMMAR-C263-FUTURE-FIRST-SINGULAR-02",
    "SA-VERB-C264-KARISYAMI-01",
    "SA-VERB-C264-PATHISYAMI-02",
    "SA-PATTERN-C264-NEAR-TERM-PLAN-03",
  ];
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"])).toEqual([
    [atoms[0]],
    [atoms[1]],
    [],
    [],
    [atoms[2]],
    [atoms[3]],
    [atoms[4]],
    [],
    [],
  ]);
  for (const atom of atoms) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }
  for (const finalLesson of chapterLessons.slice(-2)) {
    expect(finalLesson?.frontmatter["requires.knowledge"]).toEqual(expect.arrayContaining(atoms));
    expect(finalLesson?.frontmatter["practises.knowledge"]).toEqual(
      expect.arrayContaining(atoms),
    );
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  for (const form of ["गमिष्यामि", "करिष्यामि", "पठिष्यामि"]) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toContain("first-person singular");
  expect(learnerCopy).toContain("not a complete future paradigm");
  expect(learnerCopy).toContain("Second person, dual, and plural");

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(
    ledger?.chapters.filter((chapter) => chapter.chapter >= 263 && chapter.chapter <= 264),
  ).toMatchObject([
    {
      chapter: 263,
      payoff: {
        lesson: "SA-C263-tomorrow-going-recall",
        kind: "production",
        assesses: atoms.slice(0, 2),
      },
    },
    {
      chapter: 264,
      payoff: {
        lesson: "SA-C264-first-person-future-retrieval",
        kind: "production",
        assesses: atoms,
      },
    },
  ]);

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-F-10")?.probe).toEqual(atoms);
  const coverage = measureExamCoverage(inventory, loadEverything().lessons);
  expect(coverage).toMatchObject({
    enumerated: 164,
    // 146 -> 151 belongs to the later meal-ordering, market, school-plan, and
    // service-location tranches; this test still pins the exact
    // first-person-future probe above.
    covered: 151,
    unmapped: 13,
    partial: 0,
  });
  expect(coverage.byCategory["Likes, wishes and feelings"]).toEqual({
    enumerated: 2,
    covered: 2,
  });
  expect(coverage.points.find((point) => point.id === "SA-A1-F-10")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
