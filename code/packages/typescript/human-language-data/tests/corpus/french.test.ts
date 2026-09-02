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
// 108 -> 119 lessons: retiring handwritten chapter 1, the first thing a reader ever
// meets. Its eleven schema-v1 lessons already mirrored the .tex section for section --
// the block gap was ZERO in both directions -- so nothing had to be written; all eleven
// were typed instead. The chapter lands at exactly twelve atoms, the ceiling: ten words
// and phrases (salut, bien, bon, jour, bonjour, soir, bonsoir, nuit, bonne nuit), the two
// grammar rules the greetings run on (adjective agreement, and the gender the article
// carries), and the writing runway's FR-ORTHO-SALUT-01. Net +11 (0 new, 0 retired).
// 17 -> 20 culture claims: the three `culture` blocks the hand-written chapter printed --
// salut is strictly informal, bonjour is near-obligatory on entering anywhere, and bonne
// nuit is a bedtime farewell rather than an evening greeting -- were prose in the lessons
// and typed by nobody. Each is now owned and assessed.
it("pins French lesson-content budgets", () =>
  expectLanguageLessonBudgets("french", {
    lessons: 119,
    idioms: 3,
    senses: 7,
    cultureClaims: 20,
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
