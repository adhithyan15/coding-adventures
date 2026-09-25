import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity, REINFORCEMENT_WINDOWS } from "../../../src/continuity.js";
import { defaultCurriculumRoot, loadTrackChapters, loadTrackLessons } from "../../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Gujarati's meaning-first opening script spine", () => {
  const ordered = loadTrackLessons("gujarati").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const opening = ordered.slice(0, 11);
  expect(opening.map((lesson) => lesson.realization.lessonId)).toEqual([
    "GU-C01-namaste",
    "GU-W01-ha",
    "GU-W01-aa-matra",
    "GU-W01-aa",
    "GU-W01-na",
    "GU-W01-ma",
    "GU-W01-sa",
    "GU-W01-ta",
    "GU-W01-e-matra",
    "GU-W01-virama",
    "GU-W01-namaste-read",
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);

  const meaningFirst = opening[0]!;
  expect(meaningFirst.realization.romanization).toBe("namaste");
  expect(meaningFirst.frontmatter.skills).toEqual(["listening", "speaking"]);
  expect(meaningFirst.body).not.toMatch(/\p{Script=Gujarati}/u);

  const courtesy = ordered.slice(11, 26);
  expect(courtesy.map((lesson) => lesson.realization.lessonId)).toEqual([
    "GU-W02-ra",
    "GU-W02-da",
    "GU-W02-va",
    "GU-W02-ya",
    "GU-W02-bha",
    "GU-W02-dha",
    "GU-W02-vocalic-r",
    "GU-C01-aabhaar",
    "GU-C01-aavjo",
    "GU-C01-haa-naa",
    "GU-W01-haa-guided-copy",
    "GU-W01-haa-delayed-copy",
    "GU-W01-haa-dictation",
    "GU-C01-saarun",
    "GU-C01-practice",
  ]);
  expect(courtesy.every((lesson) => lesson.frontmatter.chapter === "2")).toBe(true);

  const chapterSizes = new Map<string, number>();
  for (const lesson of ordered) {
    const chapter = lesson.frontmatter.chapter;
    chapterSizes.set(chapter, (chapterSizes.get(chapter) ?? 0) + 1);
  }
  // Chapters 35-41 are the joining tranche: seven chapters of exactly five,
  // one new item per lesson with the writing lesson third in every one.
  expect([...chapterSizes.entries()]).toEqual([
    ["1", 11],
    ["2", 15],
    ["3", 10],
    ["4", 5],
    ["5", 5],
    ["6", 5],
    ["7", 5],
    ["8", 10],
    ["9", 6],
    ["10", 5],
    ["11", 5],
    ["12", 2],
    ["13", 6],
    ["14", 4],
    ["15", 4],
    ["16", 4],
    ["17", 5],
    ["18", 4],
    ["19", 6],
    ["20", 6],
    ["21", 6],
    ["22", 5],
    ["23", 1],
    ["24", 8],
    ["25", 7],
    ["26", 7],
    ["27", 8],
    // HL-C271: chapter 28, "The Day and Its Times". Fourteen lessons, thirteen
    // of them ear-first: the eight new headwords are heard and said before any
    // of them is shown, and only two ever reach the page in this chapter.
    ["28", 14],
    // HL-C286. Chapter 29 writes six words already known by ear, so it teaches
    // no new headword at all. Chapter 30 introduces nothing: nine zero-new-atom
    // lessons returning the numbers and the fifteen core verbs at R4. Chapters
    // 31-34 are the vocabulary tranche -- eight lessons each, five of them one
    // new headword apiece, then an oral checkpoint, one word to the page, and
    // an R1 return that also carries a named distant band.
    ["29", 8],
    ["30", 9],
    ["31", 8],
    ["32", 8],
    ["33", 8],
    ["34", 8],
    // 5 -> 6. HL-C432 adds one `review` lesson to chapter 35. This map counts
    // LESSONS, not atoms, and that lesson introduces none, so the script spine
    // this test is named for is unchanged.
    ["35", 6],
    ["36", 5],
    ["37", 5],
    ["38", 5],
    ["39", 5],
    ["40", 5],
    ["41", 5],
    // HL-C359: chapter 42, "Which One in the Line". Five ordinals plus a
    // payoff. It opens on FIFTH because Wiktionary's -mun entry names the five
    // numbers the suffix does not build on, and the track's count stops at
    // paanch -- so four of the five reachable ordinals are exceptions and one
    // is the rule they are exceptions to.
    ["42", 6],
    // HL-C361: chapter 43, "Past Five, and the Balance Turns Over". Eight
    // lessons in COUNTING order, because for six to ten counting order IS the
    // construction order: Wiktionary's -mun entry ends its exception list at
    // chha, so the reader crosses the boundary between exception and rule
    // exactly once, between the first lesson and the second. The writing lesson
    // is third, where chapters 35-41 already put theirs.
    ["43", 8],
    // chapter 44 -- the reading rung: words, lines, and a whole meeting
    ["44", 3],
    // chapter 45 -- HL-C443: gha and the ai sign, each from its word, and two reviews
    ["45", 4],
    // Chapters 46-91: five word lessons each; 68 and 91 also close their
    // half of the vocabulary tranche with two reviews.
    ["46", 5],
    ["47", 5],
    ["48", 5],
    ["49", 5],
    ["50", 5],
    ["51", 5],
    ["52", 5],
    ["53", 5],
    ["54", 5],
    ["55", 5],
    ["56", 5],
    ["57", 5],
    ["58", 5],
    ["59", 5],
    ["60", 5],
    ["61", 5],
    ["62", 5],
    ["63", 5],
    ["64", 5],
    ["65", 5],
    ["66", 5],
    ["67", 5],
    ["68", 7],
    ["69", 5],
    ["70", 5],
    ["71", 5],
    ["72", 5],
    ["73", 5],
    ["74", 5],
    ["75", 5],
    ["76", 5],
    ["77", 5],
    ["78", 5],
    ["79", 5],
    ["80", 5],
    ["81", 5],
    ["82", 5],
    ["83", 5],
    ["84", 5],
    ["85", 5],
    ["86", 5],
    ["87", 5],
    ["88", 5],
    ["89", 5],
    ["90", 5],
    ["91", 7],
  ]);
});
