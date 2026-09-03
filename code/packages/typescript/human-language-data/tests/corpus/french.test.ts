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
// 119 -> 128 lessons: retiring handwritten chapter 2. Like chapter 1 before it, its
// nine schema-v1 lessons already mirrored the .tex section for section, so nothing had
// to be written and the count moves by exactly the nine that were typed. Twelve atoms
// again -- five words and phrases, four grammar rules, one etymon, one pronoun pair --
// and two of the twelve carry a disambiguating suffix rather than re-pointing a
// generated chapter: FR-LEX-COMMENT-HOW-10 sits beside chapter 32's FR-LEX-COMMENT-05
// and FR-GRAM-INVERSION-NAME-11 beside its FR-GRAMMAR-INVERSION-08.
// 20 -> 21 culture claims: French PREFERS the verb-based je m'appelle to the literal
// mon nom est, which exists, is understood, and sounds like a passport desk. It was the
// hand-written chapter's only `culture` block and no lesson owned it.
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
// 140 -> 156 lessons: retiring handwritten chapter 14, Numbers Eleven to
// Twenty, which is the SECOND French chapter split. Two schema-v1 lessons owned
// TEN numbers between them plus the formation rules, and ten numbers plus their
// formation cannot fit `maxNewAtomsPerChapter` at one atom per word. The chapter
// supplies its own seam: at seventeen French stops inheriting fused Latin teens
// and starts building numbers out of parts, AND reverses the order of those
// parts. So chapter 14 keeps the six welded ones (onze..seize, 8 atoms) and a
// new chapter 15 takes the built ones (dix-sept..vingt, 6 atoms). Every later
// French chapter renumbered by +1 (old 15-35 -> 16-36). Sixteen lessons replace
// two; net +16 measured, since the retired pair were schema-v1.
// Chapter 1 and chapters 9/12/13/14 were authored on separate branches from the
// same base of 108 and met here. Every total below is RE-MEASURED against the
// merged tree by running the suite, not obtained by adding the two branches'
// deltas: composing this line by arithmetic has been wrong before. Measured:
// 167 lessons, and 22 culture claims -- chapter 9's Februa and its two emperors,
// plus chapter 1's three register claims, on a base of 17.
// 167 -> 175 lessons, and 22 -> 23 culture claims: retiring handwritten chapter
// 16, Colours. Two schema-v1 lessons held FOUR colours, and the .tex taught
// adjective POSITION inside a `grammarlens` -- `le vin blanc, le vin rouge`,
// with the colour AFTER the noun, which is the opposite of English and which no
// lesson owned. Eight lessons now own one thing each: a lesson per colour, one
// for where Latin's `albus` went when `blanc` displaced it, one for adjective
// position, and one for the flag. The culture claim is that one: the tricolour
// is two-thirds Germanic, because `bleu` and `blanc` are Frankish borrowings
// inside a Romance language. It was a `culture` block owned by nobody.
// 175 -> 189 lessons, and 23 -> 24 culture claims: retiring handwritten chapter
// 17, To Have and How Old You Are. This chapter's cost is a PARADIGM, not
// vocabulary, and only the fourth reading finds it: the .tex's opening table has
// ONE ROW PER PERSON -- j'ai, tu as, il a, nous avons, vous avez, ils ont -- and
// `maxNewGrammarCellsPerLesson` is 1, so six cells is six lessons. Two schema-v1
// lessons held all six plus the age idiom. Fourteen lessons now own one thing
// each: six for the paradigm, one for the fact that the three singular forms are
// homophones (which is WHY French keeps subject pronouns), one for the habere
// root it shares with `habiter`, and five for having your years. The culture
// claim is the Romance/Germanic split -- every Romance language on the continent
// HAS its years and every Germanic one IS them -- which was a bare table in a
// `culture` block that no lesson owned.
// 189 -> 201 lessons: retiring handwritten chapter 18, The Compound Past. Two
// schema-v1 lessons held the whole participle system (three classes), the tense
// built on it, the Latin endings behind it, the possessive construction it grew
// out of, the agreement fossil that construction left, AND a second past tense
// with its Romance sisters and the areal change that retired it. Twelve lessons
// now own one thing each; nine atoms, well under the ceiling.
//
// `finir` and `vendre` appear as EXAMPLES of the -ir and -re participle classes
// and are deliberately not taught as vocabulary -- neither verb exists anywhere
// in this corpus, and smuggling two headwords in to illustrate a pattern would
// have been exactly the cramming the atom ceiling exists to prevent. The lesson
// says so in as many words.
// 24 -> 25 culture claims: the areal change. French, German and Italian each
// swapped a simple past for a compound one, and they did it as NEIGHBOURS --
// one change spreading by contact across a connected block -- while Spanish and
// Portuguese at the western edge kept theirs. That was a `culture` block owned
// by nobody.
// 201 -> 223 lessons: retiring handwritten chapter 19, To Be and the Past That
// Takes It -- the THIRD French chapter split, and the one HL-C286 flagged in
// advance. Four schema-v1 lessons held a six-cell paradigm, three stem
// etymologies, the etre-verb list with two exceptions, two agreement rules and
// the whole pronominal system: about nineteen atoms against a ceiling of twelve.
// The chapter's own structure supplies the seam -- the verb, then the past built
// on it -- so chapter 19 keeps etre (10 atoms) and a new chapter 20 takes the
// past that selects it, pronominals included (9 atoms). Every later French
// chapter renumbered by +1 (old 20-36 -> 21-37). Twenty-two lessons replace four.
// 25 -> 26 culture claims: Spanish kept ser and estar apart as two verbs and
// made every speaker choose, where French kept one verb and swallowed stare's
// whole et- limb. Same two Latin sources, opposite solutions -- a `culture`
// block that no lesson owned.
// The two branches above met in this merge, each written from its own base.
// Every figure below is RE-MEASURED against the merged tree by running the
// suite, never obtained by adding the two branches' deltas. Measured after the
// merge: 232 lessons and 27 culture claims -- chapter 2 landed from main while
// chapters 9-19 landed here.
it("pins French lesson-content budgets", () =>
  expectLanguageLessonBudgets("french", {
    lessons: 232,
    idioms: 3,
    senses: 7,
    cultureClaims: 27,
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
