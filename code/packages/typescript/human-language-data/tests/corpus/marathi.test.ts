import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import { loadTrackLessons } from "../../src/loader.js";
import { readingOrder } from "../../src/ramp.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";

it("pins Marathi continuity", () => expectLanguageContinuity("marathi"));
it("pins Marathi modality", () => expectLanguageModality("marathi"));
it("pins Marathi lesson-content budgets", () =>
  expectLanguageLessonBudgets("marathi", {
    // 142 -> 179: the second Devanagari runway (the new chapters 5-8) adds
    // twenty-four sign lessons and four retrieval payoffs, and nine ear-only
    // retrieval lessons carry those twenty-four atoms through R2, R3 and R4.
    // Not one of the thirty-seven declares an idiom, a sense or a culture claim
    // -- a letter lesson has no business declaring any -- so only the
    // reviewed-lesson count moves and the three content counters below are
    // unchanged. That is the evidence the runway declared nothing new against
    // those budgets rather than that nobody looked.
    //
    // 179 -> 188: chapter 9's nine schema-v1 lessons migrated to v2, the last
    // thing standing between ch09-introductions and the generator. Not one is
    // a new lesson -- every one was already written and already in the book;
    // declaring their atoms is what made them MEASURABLE, so this budget can
    // see them. Re-measured against the tree, never derived. Idioms, senses
    // and culture claims are unchanged at 5 / 4 / 7: the migration typed the
    // knowledge that was already on the page and authored no new vocabulary.
    //
    // 188 -> 194: chapter 10's six schema-v1 lessons migrated to v2. Same shape
    // as chapter 9 -- every one was already written and already in the book, so
    // this counts lessons that became MEASURABLE, not lessons that were added.
    // Idioms, senses and culture claims stay at 5 / 4 / 7.
    //
    // 194 -> 205: chapters 11 and 12's eleven schema-v1 lessons migrated to v2,
    // which retires the last of Marathi's hand-written chapters. Same shape
    // again -- all eleven were already written and already in the book, so
    // this counts lessons that became MEASURABLE. With 9, 10, 11 and 12 done,
    // marathi/lessons holds NO schema-v1 lesson at all: 179 -> 205 is the
    // whole v1 island, and idioms, senses and culture claims never moved off
    // 5 / 4 / 7 across any of it.
    lessons: 205,
    idioms: 5,
    senses: 4,
    cultureClaims: 7,
    unitPrefix: "MR",
  }));

it("pins Marathi's complete pre-A1 writing ramp", () => {
  const marathi = languageWritingStages("marathi");
  expect(marathi.defects).toEqual([]);
  expect(marathi.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
});

it("keeps Marathi's opening script runways below the chapter atom budget", () => {
  const ordered = loadTrackLessons("marathi").sort(readingOrder);
  const opening = ordered.slice(0, 14);
  expect(opening.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-C01-namaskar",
    "MR-W01-ha",
    "MR-W01-o-matra",
    "MR-W01-ho-delayed-copy",
    "MR-W01-ho-dictation",
    "MR-W01-na",
    "MR-W01-aa-matra",
    "MR-W01-ii-matra",
    "MR-W01-ma",
    "MR-W01-sa",
    "MR-W01-ka",
    "MR-W01-virama",
    "MR-W01-ra",
    "MR-W01-namaskar-read",
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);

  const firstDoorway = ordered.slice(14, 21);
  expect(firstDoorway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-C01-dhanyavad",
    "MR-W02-visarga",
    "MR-W02-aa-independent",
    "MR-W02-bha",
    "MR-W02-e-matra",
    "MR-W02-anusvara",
    "MR-W02-ta",
  ]);
  expect(firstDoorway.every((lesson) => lesson.frontmatter.chapter === "2")).toBe(true);

  const secondDoorway = ordered.slice(21, 28);
  expect(secondDoorway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-W03-da",
    "MR-W03-dha",
    "MR-W03-ba",
    "MR-W03-ya",
    "MR-W03-lla",
    "MR-W03-va",
    "MR-W03-dhanyavad-write",
  ]);
  expect(secondDoorway.every((lesson) => lesson.frontmatter.chapter === "3")).toBe(true);

  // The SECOND runway, chapters 5-8. It exists because closure is measured in
  // READING ORDER: twenty-three of the twenty-four signs below were already
  // somewhere in the corpus, but the earliest lesson that could be said to
  // teach them sat at reading position 112, and every lesson from chapter 9
  // onward that used them therefore asked the reader to decode something they
  // had not been shown. Teaching more letters later moved nothing; teaching
  // these letters HERE retired all forty-four violations at once.
  //
  // Pinned as an ordered list, not a count, because the order is the argument:
  // marks first (they block the most lessons), then two consonant rows that
  // teach the voice/breath pattern, then the retroflex row, then the leftovers.
  const secondRunway = ordered.slice(33, 61);
  expect(secondRunway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-W05-i-matra",
    "MR-W05-u-matra",
    "MR-W05-uu-matra",
    "MR-W05-ru-matra",
    "MR-W05-candrabindu",
    "MR-W05-a-independent",
    "MR-R05-marks-recall",
    "MR-W06-kha",
    "MR-W06-ga",
    "MR-W06-gha",
    "MR-W06-ca",
    "MR-W06-cha",
    "MR-W06-ja",
    "MR-W06-jha",
    "MR-R06-two-rows-recall",
    "MR-W07-tta",
    "MR-W07-ttha",
    "MR-W07-dda",
    "MR-W07-nna",
    "MR-W07-pa",
    "MR-R07-retroflex-recall",
    "MR-W08-la",
    "MR-W08-sha",
    "MR-W08-ssa",
    "MR-W08-u-independent",
    "MR-W08-uu-independent",
    "MR-W08-e-independent",
    "MR-R08-runway-recall",
  ]);
  expect(
    secondRunway.every((lesson) => ["5", "6", "7", "8"].includes(lesson.frontmatter.chapter)),
  ).toBe(true);
  // Every sign lesson teaches exactly one letter, and each chapter closes with a
  // retrieval payoff that adds none.
  expect(secondRunway.filter((lesson) => lesson.realization.type === "writing")).toHaveLength(24);
  expect(secondRunway.filter((lesson) => lesson.realization.type === "review")).toHaveLength(4);

  const chapterSizes = new Map<string, number>();
  for (const lesson of ordered) {
    const chapter = lesson.frontmatter.chapter;
    chapterSizes.set(chapter, (chapterSizes.get(chapter) ?? 0) + 1);
  }
  expect([...chapterSizes.entries()]).toEqual([
    ["1", 14],
    ["2", 7],
    ["3", 7],
    ["4", 5],
    // Chapters 5-8 are the second script runway. Everything from here down used
    // to sit four numbers lower; inserting four chapters rather than stretching
    // an existing one is what keeps every chapter under the twelve-atom budget
    // while still putting all twenty-four signs BEFORE the lessons that need
    // them. Length is never a cost in this corpus, so splitting was free.
    ["5", 7],
    ["6", 8],
    ["7", 6],
    ["8", 7],
    // Chapter 9 gains one ear-only reach-back and chapter 13 four more: the
    // twenty-four new atoms need R2 and R3 retrieval, and those windows fall
    // inside chapters that already existed.
    ["9", 10],
    ["10", 6],
    ["11", 6],
    ["12", 5],
    ["13", 10],
    ["14", 6],
    ["15", 4],
    ["16", 4],
    ["17", 4],
    ["18", 6],
    ["19", 4],
    ["20", 1],
    ["21", 12],
    // Chapter 22 is where R4 lands -- eighty lessons or more after each sign
    // was taught, which is the whole point of the window.
    ["22", 9],
    ["23", 10],
    ["24", 18],
    ["25", 10],
    // Chapters 26-29 are the pre-A1 verb tranche. They sit AFTER the A1 writing
    // runways in book order while realizing pre-A1 spine nodes, which is the
    // shape the other Indic tracks already use: a node's level is a property of
    // the node, not of where the chapter falls in the book.
    ["26", 5],
    ["27", 5],
    ["28", 5],
    ["29", 4],
  ]);
});

it("keeps Marathi's A1 form-label runway gentle and pre-compositional", () => {
  const ordered = loadTrackLessons("marathi").sort(readingOrder);
  const ids = ordered.filter((lesson) => lesson.realization.chapter === 23);
  expect(ids.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-A1F01-naav",
    "MR-A1F01-shahar",
    "MR-A1F01-bhasha",
    "MR-A1F01-avdate-pey",
    "MR-A1F01-avadti-kruti",
    "MR-A1F01-mitrache-naav",
    "MR-A1F02-first-three-copy",
    "MR-A1F02-last-three-copy",
    "MR-A1F03-first-three-delayed",
    "MR-A1F03-last-three-delayed",
  ]);
  expect(ids.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 210)).toBe(true);
  expect(ids.every((lesson) => lesson.realization.type === "writing")).toBe(true);
  expect(ids.flatMap((lesson) => lesson.blocks.map((block) => block.writingStage)).filter(Boolean)).toEqual([
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "delayed-copy",
  ]);
  expect(ids.some((lesson) => lesson.body.includes("controlled-composition"))).toBe(false);
  expect(ids.some((lesson) => lesson.body.includes("timed-assessment-production"))).toBe(false);
});

it("gives the five Chapter 14 family atoms genuine R1, R2, and R3 retrieval", () => {
  const report = measureContinuity(loadTrackLessons("marathi"));
  const repairedAtoms = new Set([
    "MR-LEX-MITRA",
    "MR-ETYMON-MITRA-BIND",
    "MR-GRAMMAR-TATSAMA-BORROWING",
    "MR-LEX-KUTUMB",
    "MR-GRAMMAR-KUTUMB-NEUTER",
  ]);
  expect(report.reinforcement.filter((defect) => repairedAtoms.has(defect.atom))).toEqual([]);
});
