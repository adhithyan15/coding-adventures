import { expect, it } from "vitest";
import { measureExamCoverage } from "../../../src/exam-inventory.js";
import {
  loadEverything,
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";

it("retrieves the two existing Sanskrit A1 modern-technology nouns twice", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C270-modern-technology-first-retrieval",
    "SA-C270-modern-technology-final-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    270, 270,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    16000, 16010,
  ]);

  const pointAtoms = ["SA-LEX-C105-THINGS105-02", "SA-LEX-C269-MEDIA-02"];
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
    expect(lesson?.frontmatter["introduces.knowledge"]).toEqual([]);
    expect(lesson?.frontmatter["requires.knowledge"]).toEqual(
      expect.arrayContaining(pointAtoms),
    );
    expect(lesson?.frontmatter["practises.knowledge"]).toEqual(
      expect.arrayContaining(pointAtoms),
    );
  }
  for (const atom of pointAtoms) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  expect(learnerCopy.match(/No Sanskrit model is shown before you answer/gu)).toHaveLength(2);
  expect(learnerCopy.match(/संगणकम्/gu)).toHaveLength(2);
  expect(learnerCopy.match(/अन्तर्जालम्/gu)).toHaveLength(2);
  expect(learnerCopy).toContain(
    "not evidence of device use, internet navigation, digital literacy, technological competence, human validation, learner attainment, or exam readiness",
  );

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 270)).toMatchObject({
    chapter: 270,
    payoff: {
      lesson: "SA-C270-modern-technology-final-retrieval",
      kind: "production",
      assesses: pointAtoms,
    },
  });

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-M-13")?.probe).toEqual(
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
  expect(coverage.points.find((point) => point.id === "SA-A1-M-13")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
