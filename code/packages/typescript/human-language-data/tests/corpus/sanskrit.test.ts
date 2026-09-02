import { it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
} from "./assert-language-corpus.js";
it("pins Sanskrit continuity", () => expectLanguageContinuity("sanskrit"));
it("pins Sanskrit modality", () => expectLanguageModality("sanskrit"));
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
    lessons: 304,
    idioms: 11,
    senses: 12,
    cultureClaims: 13,
    unitPrefix: "SA",
  }));
