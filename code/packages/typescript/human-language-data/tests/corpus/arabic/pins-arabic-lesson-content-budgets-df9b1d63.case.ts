import { it } from "vitest";
import { expectLanguageLessonBudgets } from "../assert-language-corpus.js";

it("pins Arabic lesson-content budgets", () =>
  expectLanguageLessonBudgets("arabic", {
    // HL-C286: 90 -> 102. Chapter 2's twelve lessons were schema v1, which
    // declares no atoms, so `book.ts` refused to generate the chapter and this
    // budget could not see them. Migrating them to v2 is what retired Arabic's
    // last hand-written chapter. RE-MEASURED against the tree, not derived: the
    // idiom, sense and culture-claim totals are unchanged at 2 / 3 / 14, because
    // the migration declared atoms and renamed headings without authoring new
    // vocabulary.
    //
    // 102 -> 123: the present-tense-and-joining tranche (chapters 37-41) adds
    // twenty-one lessons -- sixteen items and five reviews. RE-MEASURED against
    // the tree. Idioms, senses and culture claims stay at 2 / 3 / 14: a
    // conjunction is none of the three, and the one lesson that could have
    // claimed a culture note (يا, whose absence means a learner cannot address
    // anybody) states a GRAMMATICAL fact about the vocative particle.
    // 123 -> 126: chapter 42 adds the first reading rung -- six familiar
    // words, six familiar lines and one connected encounter. It introduces no
    // vocabulary, idiom, sense or culture claim; it removes romanization in
    // three gentle steps and keeps ALPT's unpublished stimulus length honest.
    //
    // 126 -> 129: the three writing stages Arabic had never proven, on the
    // SPINE-SAY-WHY node where li-anna already lives. No new atoms in any of
    // them -- they practise wa, lakin and li-anna, which the track already
    // teaches, and add only what the stages themselves are.
    //
    // 129 -> 144: the reinforcement tranche (HL-C441) adds fifteen `type: review`
    // lessons and no vocabulary at all. Arabic was the last of twenty-three tracks
    // still carrying pre-A1 atoms revisited fewer than twice; seventy-eight of them,
    // sixty-eight never revisited, which is (68 x 2) + 10 = 146 retrieval slots.
    // RE-MEASURED against the tree. Idioms, senses and culture claims stay at
    // 2 / 3 / 14: a retrieval page introduces nothing, so there is nothing for the
    // per-lesson budgets to count. Every one of the fifteen went onto an EXISTING
    // path segment whose spine_node already matched, so no new chapter, segment or
    // extension either.
    //
    // 144 -> 160: HL-C443 chapters 46-49 add fourteen letter lessons (one per
    // letter the reader had read in words and never written) and two reviews.
    // RE-MEASURED against the tree. Idioms, senses and culture claims stay at
    // 2 / 3 / 14: a letter lesson introduces one script atom and nothing else.
    // 160 -> 414: chapters 50-99, 250 word lessons and four reviews. None
    // introduces an idiom, a sense or a culture claim.
    // 414 -> 419: HL-C443 anchor words for five letter sets in chapters 1-3
    // (ثابت, عيد, كوخ, ثمن, حاج), each placed before the set it anchors.
    // 419 -> 421: HL-C443 anchor words for the two mark sets of chapter 2
    // (مُدَرِّس, أَهْلًا). No idioms, senses or culture claims.
    // 421 -> 713: chapters 100-155 carry Arabic to A1 -- 280 word lessons
    // (places, ability, wanting, things and qualities) and twelve reviews.
    // RE-MEASURED against the tree. None introduces an idiom, a sense or a
    // culture claim, so those stay at 2 / 3 / 14.
    // 713 -> 730: chapters 156-158 realize the last three A2 spine nodes (the
    // past, the future, practical texts) with fifteen word lessons and two
    // reviews. No idiom, sense or culture claim.
    // 730 -> 903: chapters 159-191, 165 A2 words and eight reviews. No idiom, sense or culture claim.
    // 903 -> 1066: chapters 192-222, 155 A2 words and eight reviews. No idiom, sense or culture claim.
    lessons: 1066,
    idioms: 2,
    senses: 3,
    cultureClaims: 14,
    unitPrefix: "AR",
  }));
