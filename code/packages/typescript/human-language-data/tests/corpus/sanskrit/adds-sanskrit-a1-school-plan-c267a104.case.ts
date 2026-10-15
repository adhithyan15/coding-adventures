import { expect, it } from "vitest";
import { measureExamCoverage } from "../../../src/exam-inventory.js";
import {
  loadEverything,
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";

it("turns owned Sanskrit school words into a gentle A1 school-day plan", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C267-school-recall",
    "SA-C267-exam-time",
    "SA-C267-homework-plan",
    "SA-C267-complete-school-plan",
    "SA-C267-mixed-school-practice",
    "SA-C267-school-plan-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    267,
    267,
    267,
    267,
    267,
    267,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    15820,
    15830,
    15840,
    15850,
    15860,
    15870,
  ]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const newAtom = "SA-PATTERN-C267-SCHOOL-PLAN-01";
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"])).toEqual([
    [],
    [newAtom],
    [],
    [],
    [],
    [],
  ]);
  expect(
    lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(newAtom)),
  ).toHaveLength(1);
  const schoolPlanIntroduction = chapterLessons[1]?.blocks.find((block) =>
    block.knowledge?.introduces.includes(newAtom),
  );
  expect(schoolPlanIntroduction?.markdown).toContain("गृहकार्यं करिष्यामि।");
  expect(schoolPlanIntroduction?.markdown).toContain("पाठं पठिष्यामि।");
  expect(schoolPlanIntroduction?.markdown).toContain(
    "do not use it to infer a masculine accusative rule",
  );
  expect(chapterLessons[2]?.body).not.toContain("already-practised object forms");

  const pointAtoms = [
    "SA-LEX-C83-SCHOOL-01",
    "SA-LEX-C83-SCHOOL-02",
    "SA-LEX-C83-SCHOOL-03",
    "SA-LEX-C83-SCHOOL-04",
    "SA-LEX-C83-SCHOOL-05",
    "SA-LEX-C140-READ140-02",
    "SA-LEX-C168-SCH168-01",
    "SA-LEX-C169-SCH169-02",
    "SA-LEX-C255-SCH255-04",
    "SA-LEX-C255-SCH255-05",
    "SA-LEX-C18-ASK-01",
    "SA-LEX-C139-FUT139-02",
    "SA-LEX-C139-FUT139-04",
    "SA-VERB-C264-KARISYAMI-01",
    "SA-VERB-C264-PATHISYAMI-02",
    newAtom,
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
  for (const form of [
    "विद्यालयः",
    "छात्रः",
    "अध्यापकः",
    "पाठः",
    "परीक्षा",
    "समयसारणी",
    "कक्षा",
    "अंकः",
    "गृहकार्यम्",
    "उत्तरम्",
    "कदा",
    "भविष्यति",
    "करिष्यामि",
    "पठिष्यामि",
  ]) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toContain("परीक्षा कदा भविष्यति?");
  expect(learnerCopy).toContain("श्वः परीक्षा भविष्यति।");
  expect(learnerCopy.match(/No Sanskrit model is shown before you answer/gu)).toHaveLength(2);

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 267)).toMatchObject({
    chapter: 267,
    payoff: {
      lesson: "SA-C267-school-plan-retrieval",
      kind: "production",
      assesses: pointAtoms,
    },
  });

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-M-03")?.probe).toEqual(
    pointAtoms,
  );
  const coverage = measureExamCoverage(inventory, loadEverything().lessons);
  expect(coverage).toMatchObject({
    enumerated: 164,
    covered: 153,
    unmapped: 11,
    partial: 0,
  });
  expect(coverage.byCategory[
    "Modern life — fields a spoken revival needs and a classical corpus lacks"
  ]).toEqual({ enumerated: 15, covered: 11 });
  expect(coverage.points.find((point) => point.id === "SA-A1-M-03")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
