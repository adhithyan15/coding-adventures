import { expect, it } from "vitest";
import { measureExamCoverage } from "../../../src/exam-inventory.js";
import {
  loadEverything,
  loadExamInventory,
  loadTrackChapters,
  loadTrackLessons,
} from "../../../src/loader.js";

it("adds Sanskrit A1 plural possessors on a gentle contrastive ramp", () => {
  const lessons = loadTrackLessons("sanskrit");
  const ids = [
    "SA-C259-asmakam",
    "SA-C259-yusmakam",
    "SA-C259-plural-possessors-practice",
  ];
  const chapterLessons = ids.map((id) =>
    lessons.find((candidate) => candidate.realization.lessonId === id),
  );
  expect(chapterLessons.every(Boolean)).toBe(true);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.chapter))).toEqual([
    259,
    259,
    259,
  ]);
  expect(chapterLessons.map((lesson) => Number(lesson?.frontmatter.sequence))).toEqual([
    15480,
    15490,
    15500,
  ]);
  for (const lesson of chapterLessons) {
    expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(300);
    expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
    expect(lesson?.frontmatter.introduces_senses).toEqual([]);
    expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);
  }

  expect(chapterLessons[0]?.frontmatter["requires.knowledge"]).toContain("SA-LEX-MAMA");
  expect(chapterLessons[0]?.frontmatter["introduces.knowledge"]).toEqual([
    "SA-LEX-C259-ASMAKAM-01",
  ]);
  expect(chapterLessons[1]?.frontmatter["requires.knowledge"]).toContain(
    "SA-LEX-C259-ASMAKAM-01",
  );
  expect(chapterLessons[1]?.frontmatter["introduces.knowledge"]).toEqual([
    "SA-LEX-C259-YUSMAKAM-02",
  ]);
  expect(chapterLessons[2]?.frontmatter["introduces.knowledge"]).toEqual([
    "SA-GRAMMAR-C259-PLURAL-POSSESSORS-03",
  ]);
  expect(chapterLessons[2]?.frontmatter["practises.knowledge"]).toEqual(
    expect.arrayContaining([
      "SA-LEX-MAMA",
      "SA-LEX-C259-ASMAKAM-01",
      "SA-LEX-C259-YUSMAKAM-02",
      "SA-GRAMMAR-C259-PLURAL-POSSESSORS-03",
    ]),
  );

  const learnerCopy = chapterLessons
    .flatMap((lesson) => lesson?.blocks.map((block) => block.markdown) ?? [])
    .join("\n")
    .replace(/\s+/gu, " ");
  expect(learnerCopy).toContain("अस्माकम्");
  expect(learnerCopy).toContain("युष्माकम्");
  expect(learnerCopy).toContain("one possessor");
  expect(learnerCopy).toContain("several possessors");
  expect(learnerCopy).toContain("genitive");
  expect(learnerCopy).toContain("not possessive adjectives");

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 259)).toMatchObject({
    chapter: 259,
    title: "Our and your-plural",
    payoff: {
      lesson: "SA-C259-plural-possessors-practice",
      kind: "production",
      assesses: [
        "SA-LEX-C259-ASMAKAM-01",
        "SA-LEX-C259-YUSMAKAM-02",
        "SA-GRAMMAR-C259-PLURAL-POSSESSORS-03",
      ],
    },
  });

  const inventory = loadExamInventory("sanskrit", "A1");
  expect(inventory.points.find((point) => point.id === "SA-A1-POS-02")?.probe).toEqual([
    "SA-LEX-C259-ASMAKAM-01",
    "SA-LEX-C259-YUSMAKAM-02",
    "SA-GRAMMAR-C259-PLURAL-POSSESSORS-03",
  ]);
  const coverage = measureExamCoverage(inventory, loadEverything().lessons);
  expect(coverage.enumerated).toBe(164);
  expect(coverage.byCategory["Ṣaṣṭhī — possession"]).toEqual({
    enumerated: 3,
    covered: 3,
  });
  expect(coverage.points.find((point) => point.id === "SA-A1-POS-02")).toMatchObject({
    covered: true,
    missingAtoms: [],
    unmapped: false,
  });
}, 120_000);
