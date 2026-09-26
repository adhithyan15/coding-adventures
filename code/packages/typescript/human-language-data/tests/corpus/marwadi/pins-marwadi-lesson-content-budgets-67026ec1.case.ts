import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { measureScriptClosure } from "../../../src/script-closure.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Marwadi lesson-content budgets", () =>
  expectLanguageLessonBudgets("marwadi", {
    // 341 -> 344: chapter 40, the reading rung. Three lessons and NO new word --
    // every Devanagari token in all three was checked to occur in a lesson with
    // a lower sequence number. The count moves because reading is its own skill.
    //
    // 344 -> 346: chapters 41-42, the two writing stages Marwadi could honestly
    // prove. connected-composition is NOT among them and the changelog says why:
    // the track teaches no conjunction at all -- no and, but, because or or --
    // so a lesson asking for connected sentences would be asking for something
    // the book has not paid for.
    //
    // 346 -> 347: HL-C428, one `review` lesson giving the four-skill refusal its
    // second revisit. No new atom.
    //
    // 347 -> 350: HL-C443: the letter chapter adds one letter lesson per letter the reader had read in words and never written, plus two reviews. RE-MEASURED against the tree; a letter lesson introduces one script atom and no idiom, sense or culture claim.
    // 350 -> 584: chapters 44-89, 230 word lessons and four reviews. None
    // introduces an idiom, a sense or a culture claim.
    lessons: 584,
    idioms: 7,
    senses: 3,
    cultureClaims: 5,
    unitPrefix: "MW",
  }));
