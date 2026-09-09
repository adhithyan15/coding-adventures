import { expect, it } from "vitest";
import { loadEverything, loadExamInventory, loadTrackLessons } from "../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";
import { readingOrder } from "../../src/ramp.js";
import { measureScriptClosure } from "../../src/script-closure.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Hindi continuity", () => expectLanguageContinuity("hindi"));
it("pins Hindi modality", () => expectLanguageModality("hindi"));
it("pins Hindi lesson-content budgets", () =>
  expectLanguageLessonBudgets("hindi", {
    // 310 -> 343: this vocabulary tranche adds thirty-three lessons in seven
    // chapters (75-81), one new word each. Thirty-five were authored; kab and
    // kyon were cut, because #14113's joining tranche teaches both and a
    // headword introduced twice is a hard error. Re-measured against the tree.
    //
    // 343 -> 355: the ordinal tranche adds twelve lessons in three chapters
    // (82-84), one new item each, closing HI-A1-NUM-04.
    //
    // 355 -> 358: chapter 85, the reading rung — six signs, six notices, and a
    // 78-word paragraph. It adds NO new word: every token in all three was
    // checked to occur in a lesson with a lower sequence number, which is what
    // lets the lessons claim nothing in them is new. The count moves because
    // reading is its own skill, not because the track learned more Hindi.
    lessons: 358,
    idioms: 21,
    // +2: HI-C70-song declares gana's singing sense and HI-C73-drink declares
    // khana's eating sense, which is what covers HI-A1-V-26.
    senses: 24,
    cultureClaims: 27,
    unitPrefix: "HI",
  }));

it("teaches independent ऋ before ऋतु becomes load-bearing", () => {
  const ordered = loadTrackLessons("hindi").sort(readingOrder);
  const scriptLessonIndex = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "HI-S125-letter-vocalic-r",
  );
  const seasonLessonIndex = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "HI-C14-ritu",
  );

  expect(scriptLessonIndex).toBeGreaterThanOrEqual(0);
  expect(seasonLessonIndex).toBeGreaterThan(scriptLessonIndex);
  expect(ordered[scriptLessonIndex]?.body).toContain("ऋ");
  expect(ordered[seasonLessonIndex]?.realization.headword).toContain("ऋतु");
  expect(
    measureScriptClosure(ordered).violations.filter(
      (violation) => violation.lessonId === "HI-C14-ritu",
    ),
  ).toEqual([]);
});

it("pins Hindi's first cumulative pre-A1 writing-stage runway", () => {
  const hindi = languageWritingStages("hindi");
  expect(hindi.defects).toEqual([]);
  expect(hindi.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
});

it("removes support gently from a visible glyph trace to one heard known word", () => {
  const ordered = loadTrackLessons("hindi").sort(readingOrder);
  const stageLessons = ordered.filter((lesson) =>
    [
      "HI-W01-shirorekha-na-ma",
      "HI-W01-na-ma",
      "HI-W05-namaste-delayed-copy",
      "HI-W05-namaste-dictation",
    ].includes(lesson.realization.lessonId),
  );

  expect(stageLessons.map((lesson) => lesson.realization.lessonId)).toEqual([
    "HI-W01-shirorekha-na-ma",
    "HI-W01-na-ma",
    "HI-W05-namaste-delayed-copy",
    "HI-W05-namaste-dictation",
  ]);
  expect(stageLessons.map((lesson) => Number(lesson.frontmatter["duration.max_seconds"]))).toEqual([
    257, 186, 120, 120,
  ]);
  expect(stageLessons[2]?.frontmatter.prerequisites).toContain("HI-W05-write-namaste");
  expect(stageLessons[3]?.frontmatter.prerequisites).toContain("HI-W05-namaste-delayed-copy");
});

it("removes support gently from a known phrase to a no-model two-sentence purpose", () => {
  const ordered = loadTrackLessons("hindi").sort(readingOrder);
  const ids = [
    "HI-W06-name-sentence-frame",
    "HI-W06-name-sentence-stop",
    "HI-W06-name-sentence-delayed",
    "HI-W06-name-sentence-dictation",
    "HI-W06-two-sentence-card",
    "HI-W06-two-sentence-no-model",
  ];
  const lessons = ordered.filter((lesson) => ids.includes(lesson.realization.lessonId));

  expect(lessons.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(lessons.map((lesson) => Number(lesson.frontmatter["duration.max_seconds"]))).toEqual([
    150, 120, 150, 150, 180, 180,
  ]);
  for (let index = 1; index < lessons.length; index += 1) {
    expect(lessons[index]?.frontmatter.prerequisites).toContain(ids[index - 1]);
  }

  const markdown = lessons.map((lesson) =>
    lesson.blocks.map((block) => block.markdown).join("\n"),
  );
  expect(markdown[0]).toContain("four visible word groups");
  expect(markdown[1]).toContain("changes only the sentence boundary");
  expect(markdown[2]).toContain("no visible answer and no romanization");
  expect(markdown[3]).toContain("from sound alone");
  expect(markdown[4]).toContain("new classmate");
  expect(markdown[5]).toContain("There is no Devanagari model and no romanized answer");
  expect(markdown[5]).toContain("two meanings in the requested order");
  expect(markdown[5]).toContain("one **।** after each sentence");
});

// ---------------------------------------------------------------------------
// THE HINDI A1 INVENTORY HAD NO COVERAGE ASSERTION, which is the failure mode
// HL-C350's repairs were told to avoid: land the atoms, wire the probes, and
// let a number nothing reads stay whatever it was. A stale pin that agrees
// merges silently. Both tests below were falsified before being kept -- a
// fabricated atom id fails the first, and nulling HI-A1-NUM-04's probe fails
// the second.
// ---------------------------------------------------------------------------
it("probes only Hindi atoms that EXIST, so a guessed id cannot under-report", () => {
  const { lessons } = loadEverything();
  const taught = trackIntroducedAtoms(lessons, "hindi");
  const unknown: string[] = [];
  for (const point of loadExamInventory("hindi", "A1").points) {
    for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
  }
  expect(unknown).toEqual([]);
}, 60_000);

it("pins Hindi A1 coverage, and the numeral column the ordinal tranche moved", () => {
  const { lessons } = loadEverything();
  const coverage = measureExamCoverage(loadExamInventory("hindi", "A1"), lessons);
  expect(coverage.enumerated).toBe(282);
  expect(coverage.covered).toBe(193);
  expect(coverage.unmapped).toBe(89);
  expect(coverage.partial).toBe(0);
  // HL-C350 measured ordinals as the weakest single column in the corpus --
  // twenty tracks enumerate an ordinal point and eighteen left it uncovered.
  // HI-A1-NUM-04 is the one that moved here, and it is a COMPOUND point: the
  // ordinals to tenth AND the distributive ek … dusra. Both halves are taught,
  // and the second needed no new word, because dusra was taught with both its
  // senses. The three that remain in this category are a different problem and
  // stay named: NUM-03 (numerals above twenty), NUM-05 (the Devanagari digit
  // shapes) and NUM-06 (measures).
  expect(coverage.byCategory["Sankhya-vachak (quantifiers and numerals)"]!).toEqual({
    enumerated: 7,
    covered: 4,
  });
  expect(formatExamCoverage(coverage)).toContain(
    "hindi A1 (partial inventory): 193/282 points covered (68%)",
  );
}, 60_000);
