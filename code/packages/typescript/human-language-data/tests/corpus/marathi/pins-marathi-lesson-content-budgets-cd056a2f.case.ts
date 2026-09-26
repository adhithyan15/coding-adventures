import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { readingOrder } from "../../../src/ramp.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Marathi lesson-content budgets", () =>
  expectLanguageLessonBudgets("marathi", {
    // 142 -> 179: the second Devanagari runway (the new chapters 5-8) adds
    // twenty-four sign lessons and four retrieval payoffs, and nine ear-only
    // retrieval lessons carry those twenty-four atoms through R2, R3 and R4.
    // Not one of the thirty-seven declares an idiom, a sense or a culture claim
    // -- a letter lesson has no business declaring any -- so only the
    // reviewed-lesson count moves and the three content counters below are
    // unchanged. That is the evidence the runway declared nothing new against
    // those budgets rather than that nobody looked.
    //
    // 179 -> 188: chapter 9's nine schema-v1 lessons migrated to v2, the last
    // thing standing between ch09-introductions and the generator. Not one is
    // a new lesson -- every one was already written and already in the book;
    // declaring their atoms is what made them MEASURABLE, so this budget can
    // see them. Re-measured against the tree, never derived. Idioms, senses
    // and culture claims are unchanged at 5 / 4 / 7: the migration typed the
    // knowledge that was already on the page and authored no new vocabulary.
    //
    // 188 -> 194: chapter 10's six schema-v1 lessons migrated to v2. Same shape
    // as chapter 9 -- every one was already written and already in the book, so
    // this counts lessons that became MEASURABLE, not lessons that were added.
    // Idioms, senses and culture claims stay at 5 / 4 / 7.
    //
    // 194 -> 205: chapters 11 and 12's eleven schema-v1 lessons migrated to v2,
    // which retires the last of Marathi's hand-written chapters. Same shape
    // again -- all eleven were already written and already in the book, so
    // this counts lessons that became MEASURABLE. With 9, 10, 11 and 12 done,
    // marathi/lessons holds NO schema-v1 lesson at all: 179 -> 205 is the
    // whole v1 island, and idioms, senses and culture claims never moved off
    // 5 / 4 / 7 across any of it.
    //
    // 205 -> 230: the joining tranche (chapters 30-36) adds twenty-five
    // lessons -- seventeen items, two punctuation marks and seven reviews.
    // Unlike every movement above it, these ARE new lessons. Four movements
    // have now met in this one number, so it was RE-MEASURED against the
    // merged tree rather than derived by adding 9, 6, 11 and 25 to 179, which
    // would have been right only by luck. Idioms, senses and culture claims
    // stay at 5 / 4 / 7: a conjunction is not an idiom.
    //
    // 230 -> 243: the asking-word tranche (chapters 37-40) adds thirteen
    // lessons -- nine items, of which two are script signs, and four reviews.
    // RE-MEASURED against the merged tree, not derived by adding 13. Idioms,
    // senses and culture claims stay at 5 / 4 / 7 for the same reason as
    // above: an interrogative is not an idiom, and naming the k- / i- / t-
    // series is a grammar statement rather than a culture claim.
    //
    // 243 -> 258: the place tranche (chapters 41-44) adds fifteen lessons --
    // eleven items and four reviews. RE-MEASURED against the tree. Idioms,
    // senses and culture claims stay at 5 / 4 / 7 once more: a postposition is
    // none of the three, and the one lesson that could have claimed a culture
    // note (samor, built on the mukh/tond doublet) states a WORD history, which
    // is etymology and already has its own strand.
    //
    // 258 -> 273: the accompaniment tranche (chapters 45-48) adds fifteen
    // lessons -- eleven items and four reviews. RE-MEASURED. Idioms, senses
    // and culture claims stay at 5 / 4 / 7 for the third tranche running.
    //
    // 273 -> 289: the adjective tranche (chapters 49-52) adds sixteen lessons
    // -- twelve items and four reviews. RE-MEASURED. Idioms, senses and
    // culture claims stay at 5 / 4 / 7 for the fourth tranche running; the
    // one lesson that touched an existing declaration (MR-C07-khane, whose
    // gender table was retargeted onto a word the reader owns) kept its
    // single sense atom and added none.
    //
    // 289 -> 298: the degree-and-amount tranche (chapters 53-54) adds nine
    // lessons -- seven items and two reviews. RE-MEASURED. Idioms, senses and
    // culture claims stay at 5 / 4 / 7 for the fifth tranche running.
    //
    // 298 -> 322: the numbers tranche (chapters 55-59) adds twenty-four
    // lessons -- nineteen items and five reviews. RE-MEASURED. Idioms, senses
    // and culture claims stay at 5 / 4 / 7 for the sixth tranche running: a
    // numeral is none of the three, and the rupee's Sanskrit history is
    // etymology, which has its own strand.
    //
    // 322 -> 334: the ordinal tranche (chapters 60-61) adds twelve lessons --
    // ten items and two reviews. RE-MEASURED against the tree. Idioms, senses
    // and culture claims stay at 5 / 4 / 7 for the seventh tranche running.
    // That is the assertion worth making about the ordinals: dusraa is
    // re-opened in chapter 60 as a member of a set and NO sense atom is
    // declared for it, because the lesson teaches a set rather than a new
    // meaning of a word, and pahilaa's adverbial second life is named on the
    // page and deliberately not declared as a sense the reader is drilled on.
    //
    // 334 -> 337: chapter 62, the reading rung -- six form labels, a six-line
    // message, and an 83-word paragraph. It declares NO new word: every
    // Devanagari token in all three was checked to occur in a lesson with a
    // lower sequence number. The count moves because reading is its own skill.
    // 337 -> 338: the timed A1 writing paper -- one lesson, no new atoms, and
    // the last of the seven writing stages. Marathi already proved the other
    // six; this is the one that asks whether the writing survives conditions
    // the candidate does not choose.
    // 338 -> 342: CHAPTER 64 CLOSES BOTH REMAINING STOP ROWS AND THE DANDA.
    // MR-A1-OR-05 wanted ddha, the fifth of the curled row, and MR-A1-OR-07
    // wanted pha, the gap in the lip row -- one script lesson each, the route
    // MR-A1-OR-06 took at chapter 39. With pha the script has FIVE plain-and-
    // aspirated pairs and no stop is left without a partner. MR-A1-PU-01 is the
    // danda, and it is the FIRST MARK IN THE TRACK TAUGHT FOR RECOGNITION RATHER
    // THAN PRODUCTION: the corpus punctuates with a full stop, which is what
    // Marathi does now, so the reader keeps writing that and learns to read the
    // upright stroke in poetry, older printing and unreset signs. The fourth
    // lesson is a cold-retrieval review, and it is there for a measured reason --
    // an atom introduced by a chapter's LAST lesson can never be revisited, so
    // the danda would have moved atomsNeverRevisited on its own. Marathi's script
    // closure was CLEAN before this chapter (0 violations, 0 never-taught glyphs,
    // 50 taught) and stays clean: every example word uses only those 50 glyphs
    // plus the letter being taught.
    // 360 -> 364: CHAPTER 69 CLOSES ALL THREE DEMONSTRATIVE POINTS AND
    // UNBLOCKS THE VERB WORK. MR-A1-DEM was 3 of 3 open; DEM-01 (the forms in
    // three genders), DEM-02 (two-way near/far against Spanish's three-way) and
    // DEM-03 (prenominal) all close together because they are one grid.
    // THE REASON IT CAME FIRST IS HL-C395. to / tee / te are Marathi's
    // third-person PRONOUNS as well as its demonstratives, and counted as
    // TOKENS rather than substrings they were barely present: to appeared in a
    // sentence exactly once (a table row in MR-C07-jane) and most of te's
    // tokens were the RANGE word, as in ek te paach. So MR-A1-V-01, "the
    // present habitual, all persons", had no third-person subject to conjugate
    // for. This chapter supplies it.
    // TWO FRONTS AND THREE ENDINGS MAKE SIX WORDS: the front carries distance
    // and is the speaker's, the ending carries gender and is the noun's, and
    // neither decision touches the other. Taught on mitra, kholi and ghar,
    // whose genders are STATED in the corpus; chahaa was dropped from a draft
    // because its gender is nowhere stated.
    // 356 -> 360: CHAPTER 68 FINISHES THE NODE AND CLOSES MR-A1-F5-03.
    // SPINE-TIME-OF-DAY now realizes 8 of 9 and omits only GREETING-DAY, which
    // is exactly the shape kannada, latin, malayalam, telugu, tamil and hindi
    // converge on. F5-03's note named both halves of the hole -- "omits all four
    // of its greeting concepts along with its four time words" -- and chapters
    // 66 to 68 closed both.
    // ONE NEW WORD BUYS THREE GREETINGS: shubh, in front of parts of the day
    // chapter 66 already taught. The chapter refuses two things ON THE PAGE: the
    // night, whose greeting builds on raatree rather than the bare raatra and
    // would be mis-built by the pattern, and any suggestion that these are what
    // people say -- namaskaar is, and it is already a culture claim as the
    // track's default all-purpose greeting. Hindi's suprabhaat lesson gives the
    // same problem the same treatment.
    // 353 -> 356: CHAPTER 67 GIVES THE CLOCK, AND CLOSES MR-A1-NT-01.
    // NT-01 is "the clock, AND the parts of the day"; chapter 66 did the second
    // half and deliberately did not claim the point. This is the first half and
    // the point now closes -- marathi coverage 166 -> 167 of 301.
    // IT ADDS NO NEW NUMBERS. All twelve cardinals ek..baaraa were already
    // taught and kiti was already the how-many question word, so the reader had
    // both halves of "kiti vaajle?" before the chapter opened. Until now the
    // only thing the numbers could do was count.
    // TWO CLOCK WORDS, ONE OF WHICH COUNTS: the telling verb agrees in number
    // (vaajlaa after ek, vaajle after every other hour) while the placing form
    // vaajtaa is invariant. The lesson keeps that plural APART from the respect
    // plural of tumhi kase aahaat -- same shape, different reason.
    // 347 -> 353: CHAPTER 66 REALIZES A CORE SPINE NODE THAT WAS EMPTY.
    // SPINE-TIME-OF-DAY is "core": true at stage A1 with only SPINE-MEET-GREET
    // before it, and Marathi's ledger read segments: [] with ALL NINE concepts
    // omitted -- no lesson in 347 carried the node at all. This chapter realizes
    // five of the nine: the four parts of the day and the whole day that holds
    // them. The four GREETING-* concepts stay omitted and are the next chapter.
    // IT CLOSES NO EXAM POINT AND THAT IS CORRECT. MR-A1-NT-01 is "the clock,
    // AND the parts of the day"; the clock needs the numbers, which Marathi has
    // to baaraa, so it is reachable and is deliberately left to its own chapter
    // rather than claimed here. The NT column stays 6 of 6 open.
    // ONE ENDING CARRIES THE CHAPTER: -ii turns a part of the day into an
    // at-that-time word, shown on all four, and divas refuses it and takes -aa
    // because it is MASCULINE where the four parts are feminine -- an exception
    // the reader can predict from a gender system taught since chapter 10.
    // Filed as HL-C394: ten of twenty-three tracks omit all nine of this node.
    // 342 -> 347: CHAPTER 65 GIVES FOUR VOWELS THE POSITION THEY WERE MISSING.
    // MR-A1-OR-12 wanted the independent ii, o, ai and au. Every one of the four
    // had a SIGN the reader already drew -- ii and o since chapter ONE, ai and au
    // at 59 and 56 -- so the gap was never in the sound but in the POSITION, which
    // is exactly what made it easy to miss. A sign hangs on a consonant; a letter
    // starts a word; the two shapes look nothing alike and are learned separately.
    // That is why the point stays distinct from MR-A1-OR-14, which is those same
    // signs and closed at 56 and 59. It was blocking a number sitting in the middle
    // of an ordinary counting sequence -- ऐंशी, eighty, opens with the independent
    // ai -- and nothing about a word like that announces that one of its four
    // shapes had never been drawn. The fifth lesson is the cold-retrieval review,
    // there for the same measured reason as chapter 64's.
    //
    // 364 -> 366: HL-C432, two `review` lessons closing the track's
    // reinforcement debt -- the first meeting retrieved as one exchange, and
    // the three script marks of chapters 64-65. No new atoms, no new headwords.
    //
    // 366 -> 627: chapters 70-120, 255 word lessons and six reviews. None
    // introduces an idiom, a sense or a culture claim.
    // 627 -> 633: HL-C443 anchor words for six letters in chapters 56-65.
    // 633 -> 652: HL-C443 anchor words for the opening runways, chapters 2 and
    // 5-8. Nineteen short words, each read before the letters it holds. None
    // introduces an idiom, a sense or a culture claim.
    lessons: 652,
    idioms: 5,
    senses: 4,
    // 7 -> 8: MR-CULTURE-SHUBH-FORMAL-WRITTEN-REGISTER-01. The shubh greetings
    // are formal and written; namaskaar is what is spoken at any hour. That is a
    // claim about USE rather than about meaning, so it is a culture claim and
    // not a lexical atom -- the same shape as the track's existing claim that
    // namaskaar is the Marathi default greeting.
    cultureClaims: 8,
    unitPrefix: "MR",
  }));
