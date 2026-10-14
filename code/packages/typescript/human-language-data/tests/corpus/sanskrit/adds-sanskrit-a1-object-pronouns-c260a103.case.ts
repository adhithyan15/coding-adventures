import { expect, it } from "vitest";
import { measureExamCoverage } from "../../../src/exam-inventory.js";
import {
  loadEverything,
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";

it("adds the Sanskrit A1 object-pronoun series one number at a time", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C260-mam",
    "SA-C260-tvam",
    "SA-C260-singular-object-practice",
    "SA-C261-avam",
    "SA-C261-yuvam",
    "SA-C261-dual-object-practice",
    "SA-C262-asman",
    "SA-C262-yusman",
    "SA-C262-object-person-number",
    "SA-C262-object-pronoun-retrieval",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    260,
    260,
    260,
    261,
    261,
    261,
    262,
    262,
    262,
    262,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    15510,
    15520,
    15530,
    15540,
    15550,
    15560,
    15570,
    15580,
    15590,
    15600,
  ]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  const atoms = [
    "SA-PRON-C260-MAM-01",
    "SA-PRON-C260-TVAM-02",
    "SA-GRAMMAR-C260-SINGULAR-OBJECTS-03",
    "SA-PRON-C261-AVAM-01",
    "SA-PRON-C261-YUVAM-02",
    "SA-GRAMMAR-C261-DUAL-OBJECTS-03",
    "SA-PRON-C262-ASMAN-01",
    "SA-PRON-C262-YUSMAN-02",
    "SA-GRAMMAR-C262-OBJECT-PERSON-NUMBER-03",
  ];
  expect(chapterLessons.map((lesson) => lesson?.frontmatter["introduces.knowledge"])).toEqual([
    [atoms[0]],
    [atoms[1]],
    [atoms[2]],
    [atoms[3]],
    [atoms[4]],
    [atoms[5]],
    [atoms[6]],
    [atoms[7]],
    [atoms[8]],
    [],
  ]);
  for (const atom of atoms) {
    expect(
      lessons.filter((lesson) => lesson.frontmatter["introduces.knowledge"].includes(atom)),
    ).toHaveLength(1);
  }
  expect(chapterLessons[9]?.frontmatter["requires.knowledge"]).toEqual(
    expect.arrayContaining(atoms),
  );
  expect(chapterLessons[9]?.frontmatter["practises.knowledge"]).toEqual(
    expect.arrayContaining(atoms),
  );

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  for (const form of ["माम्", "त्वाम्", "आवाम्", "युवाम्", "अस्मान्", "युष्मान्"]) {
    expect(learnerCopy).toContain(form);
  }
  expect(learnerCopy).toContain("one person");
  expect(learnerCopy).toContain("two people");
  expect(learnerCopy).toContain("several people");
  expect(learnerCopy).toContain("independent full forms");
  expect(learnerCopy).toContain("short alternatives");
  expect(learnerCopy).toContain("not the whole Classical Sanskrit pronoun system");

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(
    ledger?.chapters.filter((chapter) => chapter.chapter >= 260 && chapter.chapter <= 262),
  ).toMatchObject([
    {
      chapter: 260,
      payoff: {
        lesson: "SA-C260-singular-object-practice",
        kind: "production",
        assesses: atoms.slice(0, 3),
      },
    },
    {
      chapter: 261,
      payoff: {
        lesson: "SA-C261-dual-object-practice",
        kind: "production",
        assesses: atoms.slice(3, 6),
      },
    },
    {
      chapter: 262,
      payoff: {
        lesson: "SA-C262-object-pronoun-retrieval",
        kind: "production",
        assesses: atoms,
      },
    },
  ]);

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-PRON-03")?.probe).toEqual(atoms);
  const coverage = measureExamCoverage(inventory, loadEverything().lessons);
  expect(coverage).toMatchObject({
    enumerated: 164,
    // 145 -> 146 belongs to the later first-person-future tranche; this test
    // still pins the exact object-pronoun probe above.
    covered: 146,
    unmapped: 18,
    partial: 0,
  });
  expect(coverage.byCategory["Sarvanāma — the pronoun"]).toEqual({
    enumerated: 8,
    covered: 7,
  });
  expect(coverage.points.find((point) => point.id === "SA-A1-PRON-03")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
