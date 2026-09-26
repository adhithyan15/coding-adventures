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
    //
    // 347 -> 354: HL-C434, seven `practice` lessons closing the track's pre-A1
    // reinforcement debt. `practice` and not `review` because german has no
    // `review` lesson anywhere; both types sit outside `CONTENT_TYPES`, so
    // neither adds a headword. 19 of the 37 thin atoms had zero revisits and the
    // criterion asks for two, so the true cost was 56 retrieval slots rather
    // than 37. None of the seven introduces an atom, so every other number in
    // this object is unchanged.
    //
    // 354 -> 606: eight `-more` continuations split off the eight lessons that
    // introduced more than three atoms, then chapters 56-103 (240 word lessons,
    // five per chapter) and their four review lessons. None of them introduces
    // an idiom, a sense or a culture claim, so those counts stand.
    lessons: 606,
    idioms: 1,
    senses: 5,
    cultureClaims: 32,
    unitPrefix: "GE",
  }));
