import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity } from "../../../src/continuity.js";
import {
  DOC_SHARD_PLANS,
  defaultRepoRoot,
  unshardDocContents,
} from "../../../src/doc-shard-cli.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

// 173 -> 196 with the pre-A1 courtesy-and-parting tranche (Chapters 31-36): 23 new
// lessons, of which 14 are content lessons carrying a headword and 9 are single-glyph
// script sessions the budget counter does not measure.
// 196 -> 214 with the pre-A1 script runway inserted into Chapters 2-13 (#13068): 18
// recognition sessions, each teaching at most three Gurmukhi pieces BEFORE a lesson
// asks the reader to decode them. They are what took Punjabi's script-closure debt
// from 40 violating lessons to 0 and its never-taught glyph count from 8 to 0.
// 214 -> 216 when Chapters 4 and 5 stopped being hand-written .tex and became
// generated from their lessons: the migration to schema v2 split the two Chapter 5
// sessions that had packed several headwords each, so panjābī and karnā now get a
// lesson apiece. The same migration is why idioms and culture claims move: the
// farewell lessons finally DECLARE the units they were always teaching
// (phir milāṁge and rabb rākhā as idioms, rabb rākhā's Arabic-plus-Sanskrit blend
// as a culture claim), which a schema-v1 lesson had no field to say.
it("pins Punjabi lesson-content budgets", () =>
  expectLanguageLessonBudgets("punjabi", {
    // 226 -> 261. Seven chapters of five, one new item per lesson, closing the
    // Jorr column the A1 inventory measured at 0 of 11. One culture claim: the
    // Sanskritic / Perso-Arabic pair rule, which the inventory said one lesson
    // would close and which this book has owed since it taught dhanvaad and
    // shukriya side by side in its first chapter.
    //
    // 261 -> 267: the ordinal tranche, chapter 44. Five ordinals one word a
    // lesson plus the chapter's retrieval payoff, and seven atoms, because two
    // of the six carry a naming atom for a SET or an ENDING beside the word
    // that shows it. Re-measured against the tree. Idioms, senses and culture
    // claims are unchanged at 6 / 3 / 9, and that is the claim worth making: an
    // ordinal is none of the three, and the Sanskrit etymologies the tranche
    // cites are etymology, which has its own strand.
    //
    // 267 -> 270: chapter 45, the reading rung -- six labels, a filled form, and
    // the same six facts as a 72-word paragraph. No new word: every Gurmukhi
    // token was checked to occur in a lesson with a lower sequence number.
    //
    // 270 -> 271: chapter 46, the timed A1 writing paper. One lesson, no new
    // atoms -- it practises the form atoms the track already teaches and adds
    // only the condition the six stages before it withheld: a clock.
    //
    // 271 -> 272: chapter 47, connected composition. Also no new vocabulary --
    // ate and par are the joiners chapters 38 and 39 already taught, and the
    // exercise is the joining rather than the reaching. Between them the two
    // lessons close Punjabi's whole writing ramp, A1 through C2.
    //
    // 272 -> 278: HL-C437, six `review` lessons in a new chapter 48 closing the
    // track's pre-A1 reinforcement debt. Punjabi's forty thin atoms are spread
    // across SEVENTEEN path segments -- no earlier chapter could reach them --
    // so the tranche is appended at the end rather than placed in among the
    // material it retrieves. None of the six introduces an atom or a headword.
    lessons: 278,
    idioms: 6,
    senses: 3,
    cultureClaims: 9,
    unitPrefix: "PA",
  }));
