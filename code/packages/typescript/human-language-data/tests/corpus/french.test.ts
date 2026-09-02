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
// 90 -> 99 lessons: retiring handwritten chapter 7. Its two schema-v1 lessons
// taught five day-names and then two more, behind two wide reveal tables; they
// are replaced by nine schema-v2 lessons -- la lune, then one day per lesson,
// then the chapter practice. Net +7 (9 new, 2 retired). `la lune` earns a lesson
// because the .tex taught it inside a parenthesis in a table cell and no lesson
// owned it, which is the shape the block-gap measure cannot see.
// 78 -> 90 lessons: retiring handwritten chapter 6. Its two schema-v1 lessons
// each taught five numbers at once behind a wide reveal table; they are replaced
// by twelve schema-v2 lessons -- one number per lesson, then the calendar
// synthesis, then the chapter practice -- so the whole chapter is measurable and
// the numbers arrive one at a time. Net +12 (14 new, 2 retired).
// 13 -> 14 culture claims: the Roman-calendar claim (septembre to decembre still
// count seven to ten because the old year began in March) lived only in the
// hand-written grammarlens and is now owned by FR-C06-mois-romains.
// 14 -> 16 culture claims: interpretatio germanica (the Germanic peoples swapped
// their own gods into the Roman week, role for role) and the samedi/Saturday
// swap (French kept the Roman gods on the weekdays and took the Sabbath for the
// weekend; English did the reverse) both lived only in hand-written cousinweb
// blocks and are now owned by FR-C07-mardi and FR-C07-samedi.
// 99 -> 108 lessons: retiring handwritten chapter 8. Its two schema-v1 lessons
// owned three words between them (heure, midi, minuit) for a chapter whose .tex
// also taught `il est ... heures`, the une heure / deux heures agreement, and
// named `et quart`, `et demie` and `moins le quart` while deferring them. None
// of those was owned by any lesson, and none of them cost a prose block -- which
// is the shape the block-gap measure cannot see. Nine lessons now own all of it,
// the deferred three included: a reader who cannot say "half past" cannot tell
// the time. Net +8 (9 new, 1 retired).
// 16 -> 17 culture claims: English `noon` is Latin nona hora, "the ninth hour",
// which drifted from mid-afternoon to midday; it lived only in a hand-written
// culture block and is now owned by FR-C08-midi.
// 108 -> 128 lessons: retiring handwritten chapter 9, which is the first FRENCH
// CHAPTER SPLIT. Its two schema-v1 lessons owned `les mois` and `les saisons` --
// two headwords for TWELVE months and FOUR seasons. Sixteen words plus the
// au/en rule cannot fit `maxNewAtomsPerChapter` at one atom per word, and length
// is never a cost here, so chapter 9 became three chapters: the months to juin,
// the months from juillet, and the seasons. Twenty lessons replace two, and
// every later French chapter renumbered by +2 (old 10-33 -> 12-35).
// 17 -> 19 culture claims: the Februa, the purification festival February is
// named for; and the two men who put themselves in a calendar of gods. Both
// lived only in hand-written blocks and are now owned by FR-C09-fevrier and
// FR-C09-aout.
// 128 -> 135 lessons: retiring handwritten chapter 12, Family. Its two
// schema-v1 lessons held FOUR headwords between them -- `le pere, la mere` in
// one and `le frere, la soeur` in the other -- so no word in the chapter had a
// lesson of its own. Seven schema-v2 lessons replace them: one per word, one
// for Grimm's law (which the .tex hid inside a `grammarlens` and no lesson
// owned), and one for the oe ligature, which is a LETTER and pays forward to
// l'oeuf and l'oeil. Net +7 measured rather than +5, because the two retired
// lessons were schema-v1 and never counted toward this budget.
// 135 -> 140 lessons: retiring handwritten chapter 13, Bread, Water, Wine. Two
// schema-v1 lessons held THREE headwords -- `le pain` in one and `l'eau, le vin`
// in the other -- and the chapter's own canDo promised the reader could REQUEST
// them, which nothing taught: `du pain` appeared in a Guided Practice line and
// in a `culture` block, and no lesson owned the partitive. Five lessons now do:
// one per noun, one for `de` + article, and the payoff. Net +5 measured rather
// than +3, because the two retired lessons were schema-v1 and never counted.
it("pins French lesson-content budgets", () =>
  expectLanguageLessonBudgets("french", {
    lessons: 140,
    idioms: 3,
    senses: 7,
    cultureClaims: 19,
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
