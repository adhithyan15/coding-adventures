import { expect, it } from "vitest";
import { loadEverything, loadExamInventory } from "../../../src/loader.js";
import { formatExamCoverage, measureExamCoverage } from "../../../src/exam-inventory.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Sanskrit lesson-content budgets", () =>
  expectLanguageLessonBudgets("sanskrit", {
    // 245 -> 270: the script ladder was rebuilt and moved earlier. Twenty-five new
    // recognition segments (SA-S200..SA-S224) join the twenty-three that already
    // existed, and every one of the forty-eight now credits EXACTLY ONE new
    // Devanagari character, scheduled so the character lands before the first lesson
    // that asks the reader to decode it.
    //
    // Each declares zero idioms, senses and culture claims, so only the measured-lesson
    // count moves; the three content totals below are unchanged, which is the check that
    // the segments really are script lessons and not vocabulary wearing a script label.
    //
    // 270 -> 304: chapters 1-5 left the hand-written set, so their thirty lessons
    // migrated to schema v2 and became measurable at all (the track now has zero
    // measurement-blind lessons); three lessons that each packed two headwords
    // split in two — yes/no, the two words for "you", and "well" versus the reply
    // "I am well"; and chapter 2 gained the recap lesson it never had, so the
    // closing exchange the hand-written chapter carried is still in the book.
    // The three content totals below are again unchanged: the split halves, the
    // migrated lessons and the new recap declare no new idioms, senses or culture
    // claims, so nothing was smuggled in under cover of the migration.
    //
    // 304 -> 335: the clause-joining and past-tense tranche, chapters 52-61.
    // Thirty-one lessons: four joining particles and a review; the ya- / ta-
    // correlative; iti and the clause it makes into an object; the -tva gerund
    // and the -tum infinitive; the imperfect; can and want; the dative-experiencer
    // liking frame; the origin words; the danda; and rtu with the independent ऋ
    // it finally gives a headword to. Re-measured against the tree.
    //
    // The three content totals below are unchanged, and that is the check worth
    // making here: a grammar tranche should declare no new idioms, senses or
    // culture claims, and this one declares none.
    //
    // 335 -> 347: the ordinal tranche, chapters 62-63. Twelve lessons -- ten
    // ordinals one word a lesson, plus each chapter's retrieval payoff -- and
    // thirteen atoms, because three of the twelve carry a naming atom for an
    // ENDING beside the word that shows it. Re-measured against the tree.
    // Idioms, senses and culture claims are again unchanged at 11 / 12 / 13, and
    // here that is a deliberate claim rather than an accident: dvitiyah also
    // means "a companion" in Macdonell and the lesson says so on the page, but
    // no SENSE atom is declared for it, because the tranche teaches the numeral
    // and does not drill the second reading.
    //
    // 347 -> 350: the reading rung, chapter 64. Three lessons -- six words, six
    // lines, one passage -- and NO new atoms at all beyond the three reading
    // skills themselves, because the rung introduces no vocabulary: every word
    // on all three pages was paid for by an earlier chapter, and the check that
    // says so is that no token in them is absent from a lower-sequence lesson.
    // Idioms, senses and culture claims are unchanged at 11 / 12 / 13. The
    // passage's closing note about शान्तिः is prose about where the word is
    // used, not a culture claim the track asserts and drills, so none is
    // declared. Re-measured against the tree.
    //
    // 350 -> 353: the pre-A1 writing ladder. Sanskrit had no writing-stage
    // evidence at all, so three lessons were added on न -- a guided copy that
    // follows the three sourced strokes and counts pen lifts, a delayed copy,
    // and a dictation -- while the existing letter lesson's trace block gained
    // the observe-trace marker it had always earned. No new atoms: all three
    // practise SA-SCRIPT-RECOG-02, which the letter lesson already introduced.
    lessons: 353,
    idioms: 11,
    senses: 12,
    cultureClaims: 13,
    unitPrefix: "SA",
  }));
