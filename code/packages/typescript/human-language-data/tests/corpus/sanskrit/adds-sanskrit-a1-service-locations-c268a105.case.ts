import { expect, it } from "vitest";
import { measureExamCoverage } from "../../../src/exam-inventory.js";
import {
  loadEverything,
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";

it("builds one sourced Sanskrit A1 four-service location exchange", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C268-service-recall",
    "SA-C268-vittakosah",
    "SA-C268-post-health-location",
    "SA-C268-bank-police-location",
    "SA-C268-service-location-practice",
    "SA-C268-service-location-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    268,
    268,
    268,
    268,
    268,
    268,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    15880,
    15890,
    15900,
    15910,
    15920,
    15930,
  ]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const bankAtom = "SA-LEX-C268-SERVICE-01";
  const exchangeAtom = "SA-PATTERN-C268-SERVICE-LOCATION-02";
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"])).toEqual([
    [],
    [bankAtom],
    [exchangeAtom],
    [],
    [],
    [],
  ]);
  for (const atom of [bankAtom, exchangeAtom]) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }

  const bankIntroduction = chapterLessons[1]?.blocks.find((block) =>
    block.knowledge?.introduces.includes(bankAtom),
  );
  expect(bankIntroduction?.markdown).toContain("वित्तकोषः");
  expect(bankIntroduction?.markdown).toContain("financial bank");
  expect(bankIntroduction?.markdown).toContain("kosha.sanskrit.today/glossary/places-bank");
  expect(bankIntroduction?.markdown).toContain("sanskritdocuments.org/dict/dictallcheck.pdf");
  expect(bankIntroduction?.markdown).toContain("not the riverbank");

  const pointAtoms = [
    "SA-LEX-C120-THINGS120-02",
    bankAtom,
    "SA-LEX-C84-PLACES-02",
    "SA-LEX-C238-PPL238-04",
    "SA-LEX-C14-DEIXIS-06",
    "SA-LEX-ASTI",
    exchangeAtom,
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
  for (const form of ["पत्रालयः", "वित्तकोषः", "चिकित्सालयः", "आरक्षकः", "कुत्र", "अस्ति"])
    expect(learnerCopy).toContain(form);
  expect(learnerCopy).toContain("पत्रालयः कुत्र अस्ति?");
  expect(learnerCopy).toContain("वित्तकोषः कुत्र अस्ति?");
  expect(learnerCopy).toContain("चिकित्सालयः कुत्र अस्ति?");
  expect(learnerCopy).toContain("आरक्षकः कुत्र अस्ति?");
  expect(learnerCopy.match(/No Sanskrit model is shown before you answer/gu)).toHaveLength(2);

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 268)).toMatchObject({
    chapter: 268,
    payoff: {
      lesson: "SA-C268-service-location-retrieval",
      kind: "production",
      assesses: pointAtoms,
    },
  });

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-M-08")?.probe).toEqual(
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
  expect(coverage.points.find((point) => point.id === "SA-A1-M-08")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
