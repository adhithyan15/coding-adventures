import { it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
} from "./assert-language-corpus.js";
it("pins German continuity", () => expectLanguageContinuity("german"));
it("pins German modality", () => expectLanguageModality("german"));
it("pins German lesson-content budgets", () =>
  expectLanguageLessonBudgets("german", {
    //
    // 339 -> 342: chapter 52, the reading rung -- six nouns, six sentences and a
    // 48-word paragraph. No new word: every token was checked to occur in a
    // lesson with a lower sequence number.
    // 342 -> 344: the two writing stages German did not prove. A delayed copy of
    // Hallo with the model covered -- where the capital is the expected miss,
    // because the sound does not carry it -- and a dictation, where the doubled
    // l is, because German writes vowel length with what follows the vowel.
    //
    // 344 -> 347: the three writing stages German had never proven, on the
    // SPINE-SAY-WHY node where weil already lives. No new atoms in any of the
    // three -- they practise the joiners and the possessive the track already
    // teaches, and add only what the stages themselves are: a choice with no
    // model, a clock, and a join.
    lessons: 347,
    idioms: 1,
    senses: 5,
    cultureClaims: 32,
    unitPrefix: "GE",
  }));
