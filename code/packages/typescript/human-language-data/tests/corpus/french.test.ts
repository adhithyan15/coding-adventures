import { expect, it } from "vitest";
import { compileLessonActivities } from "../../src/activity.js";
import { loadTrackLessons } from "../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";

it("pins French continuity", () => expectLanguageContinuity("french"));
it("pins French modality", () => expectLanguageModality("french"));
// 58 -> 78 lessons: retiring the handwritten chapters 3, 4 and 5 migrated their
// twenty lessons to schema v2, which is what makes a lesson measurable at all.
// 10 -> 13 culture claims: the three `culture` blocks those chapters carried only
// in LaTeX (merci's three metaphors for gratitude, the comme ci comme ça shrug,
// and travailler's Spanish twin) are now typed claims owned by a lesson.
it("pins French lesson-content budgets", () =>
  expectLanguageLessonBudgets("french", {
    lessons: 78,
    idioms: 3,
    senses: 7,
    cultureClaims: 13,
    unitPrefix: "FR",
  }));

it("pins French's complete pre-A1 writing runway", () => {
  const french = languageWritingStages("french");
  expect(french.defects).toEqual([]);
  expect(french.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  expect(french.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]);
});

it("pins French-owned objective activities without extending a global ledger", () => {
  const ids = loadTrackLessons("french")
    .flatMap((lesson) => compileLessonActivities(lesson.blocks))
    .map((activity) => activity.id)
    .sort();
  expect(ids).toEqual([
    "FR-C18-oui-negative",
    "FR-W01-salut-delayed-copy-check",
    "FR-W01-salut-dictation-answer",
    "FR-W01-salut-guided-copy-check",
    "FR-W01-salut-observe-final",
  ]);
});
