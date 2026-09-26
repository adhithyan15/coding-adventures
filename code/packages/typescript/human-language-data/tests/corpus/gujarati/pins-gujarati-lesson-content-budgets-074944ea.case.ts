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

it("pins Gujarati lesson-content budgets", () =>
  expectLanguageLessonBudgets("gujarati", {
    // HL-C286: 179 -> 228. Six chapters. Chapter 29 writes the six time words
    // chapter 28 left oral; chapter 30 is the fifth-return slab the previous
    // tranche filed rather than smuggled into a vocabulary chapter; chapters
    // 31-34 each teach five new pre-A1 headwords one lesson at a time, ear
    // first, and write exactly one of them. None declares an idiom, sense, or
    // culture claim, so only the lesson total moves.
    // 228 -> 263, and 15 -> 16 culture claims. Seven chapters, five lessons each,
    // closing the joining column the A1 inventory measured at 0 of 11. The single
    // culture claim is that mataf karo both apologises AND stops a stranger, which
    // is why one phrase closes a courtesy function and a repair strategy at once.
    // HL-C359: 263 -> 269. Chapter 42, the ordinal column, six lessons and
    // eight atoms. No new idiom, sense or culture claim, so only the lesson
    // total moves.
    // 269 -> 277. Chapter 43, the cardinals six to ten: five numbers, one
    // writing lesson for the retroflex aspirate that eight needs, one rule
    // lesson naming both edges of the -mun exception list, and a payoff. Seven
    // atoms. Again no new idiom, sense or culture claim.
    //
    // 277 -> 280: chapter 44, the reading rung -- six signboard words, six lines
    // the track had only ever said aloud, and a 35-word meeting. No new word:
    // every Gujarati token was checked to occur in a lesson with a lower
    // sequence number.
    //
    // 280 -> 281: HL-C432, one `review` lesson closing the track's
    // reinforcement debt -- the words readable unaided, set against the repair
    // kit for the ones that are not. No new atoms and no new headwords.
    //
    // 281 -> 285: HL-C443: the letter chapter adds one letter lesson per letter the reader had read in words and never written, plus two reviews. RE-MEASURED against the tree; a letter lesson introduces one script atom and no idiom, sense or culture claim.
    // 285 -> 519: the pre-A1 vocabulary tranche, chapters 46-91. 230 word
    // lessons (nineteen verbs) and four reviews; each word lesson introduces one
    // lexical atom and no idiom, sense or culture claim. Re-measured.
    // 519 -> 532: HL-C443, thirteen anchor words opening the runway chapters
    // (1 and 3-7). Each introduces one lexical atom and no idiom, sense or
    // culture claim.
    lessons: 532,
    idioms: 12,
    senses: 6,
    cultureClaims: 16,
    unitPrefix: "GU",
  }));
