import { it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
} from "./assert-language-corpus.js";
it("pins Urdu continuity", () => expectLanguageContinuity("urdu"));
it("pins Urdu modality", () => expectLanguageModality("urdu"));
it("pins Urdu lesson-content budgets", () =>
  expectLanguageLessonBudgets("urdu", {
    // 68 -> 88: HL-C240, chapters 17 and 18. Twenty schema-v2 lessons that
    // interleave six Nastaliq letters with six new pre-A1 headwords, so the
    // ladder is gloss-first and never a block of alphabet. Idioms, senses and
    // culture claims are unchanged: none of the twenty declares one.
    //
    // 88 -> 89: UR-C01-salam migrated from schema v1 to v2, the last thing
    // standing between chapter 1 and the generator. The lesson was already
    // written; declaring its one atom is what made it MEASURABLE, so this
    // budget can see it. Re-measured against the tree, never derived. Idioms,
    // senses and culture claims are again unchanged at 2 / 4 / 4: the
    // migration declared an atom, it did not author vocabulary.
    //
    // 89 -> 108: the negation-and-joining tranche (chapters 19-22) adds nineteen
    // lessons -- fifteen items and four reviews. RE-MEASURED against the tree.
    // Idioms, senses and culture claims stay at 2 / 4 / 4: a conjunction is
    // none of the three, and the one lesson that could have claimed a culture
    // note (ārām se, whose politeness comes from asking for ease rather than
    // slowness) states what the WORD means, which is etymology and already has
    // its own strand.
    //
    // 108 -> 134: the oblique tranche (chapters 23-27) adds twenty-six lessons --
    // twenty-one items and five reviews. RE-MEASURED against the tree. Idioms,
    // senses and culture claims stay at 2 / 4 / 4: a case ending is none of the
    // three, and the two lessons that came closest to a culture claim -- the
    // vocative, where an Urdu speaker addresses almost nobody by name, and the
    // nisba, where a person's last name WAS their birthplace -- both state a fact
    // about how the language is used rather than a claim about a people, which is
    // pragmatics and etymology and both already have their own strands.
    lessons: 134,
    idioms: 2,
    senses: 4,
    cultureClaims: 4,
    unitPrefix: "UR",
  }));
