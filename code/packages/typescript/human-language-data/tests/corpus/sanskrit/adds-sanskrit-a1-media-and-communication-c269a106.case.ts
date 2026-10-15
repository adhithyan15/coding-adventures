import { expect, it } from "vitest";
import { measureExamCoverage } from "../../../src/exam-inventory.js";
import {
  loadEverything,
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";

it("builds one sourced Sanskrit A1 six-word media and communication field", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C269-media-recall",
    "SA-C269-akasavani",
    "SA-C269-antarjalam",
    "SA-C269-six-channel-practice",
    "SA-C269-media-first-retrieval",
    "SA-C269-media-final-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    269, 269, 269, 269, 269, 269,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    15940, 15950, 15960, 15970, 15980, 15990,
  ]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const radioAtom = "SA-LEX-C269-MEDIA-01";
  const internetAtom = "SA-LEX-C269-MEDIA-02";
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"])).toEqual([
    [],
    [radioAtom],
    [internetAtom],
    [],
    [],
    [],
  ]);
  for (const atom of [radioAtom, internetAtom]) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }

  const radioIntroduction = chapterLessons[1]?.blocks.find((block) =>
    block.knowledge?.introduces.includes(radioAtom),
  );
  expect(radioIntroduction?.markdown).toContain("आकाशवाणी");
  expect(radioIntroduction?.markdown).toContain("radio");
  expect(radioIntroduction?.markdown).toContain("kosha.sanskrit.today/word/en/radio");
  expect(radioIntroduction?.markdown).toContain("only the modern radio meaning");

  const internetIntroduction = chapterLessons[2]?.blocks.find((block) =>
    block.knowledge?.introduces.includes(internetAtom),
  );
  expect(internetIntroduction?.markdown).toContain("अन्तर्जालम्");
  expect(internetIntroduction?.markdown).toContain("internet");
  expect(internetIntroduction?.markdown).toContain("kosha.sanskrit.today/word/en/Internet");
  expect(internetIntroduction?.markdown).toContain("kosha.sanskrit.today/glossary/computer-internet");

  const pointAtoms = [
    "SA-LEX-C222-THG222-05",
    "SA-LEX-C104-THINGS104-05",
    radioAtom,
    "SA-LEX-C105-THINGS105-01",
    internetAtom,
    "SA-LEX-C120-THINGS120-03",
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
  for (const form of ["पत्रिका", "दूरभाषः", "आकाशवाणी", "दूरदर्शनम्", "अन्तर्जालम्", "लेखः"])
    expect(learnerCopy).toContain(form);
  expect(learnerCopy.match(/No Sanskrit model is shown before you answer/gu)).toHaveLength(2);

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 269)).toMatchObject({
    chapter: 269,
    payoff: {
      lesson: "SA-C269-media-final-retrieval",
      kind: "production",
      assesses: pointAtoms,
    },
  });

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-M-06")?.probe).toEqual(
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
  expect(coverage.points.find((point) => point.id === "SA-A1-M-06")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
