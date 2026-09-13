import { expect, it } from "vitest";
import { loadAssessmentPolicy, loadChapterPolicy, loadEverything, loadTrackLessons } from "../../src/loader.js";
import { buildRootLedger } from "../../src/root-ledger.js";
import { measureWritingStages } from "../../src/writing-stages.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
} from "./assert-language-corpus.js";

it("pins Persian continuity", () => expectLanguageContinuity("persian"));
it("pins Persian modality", () => expectLanguageModality("persian"));
it("pins Persian lesson-content budgets", () =>
  expectLanguageLessonBudgets("persian", {
    //
    // 97 -> 100: chapter 21, the reading rung. Three lessons, no new word and no
    // new letter -- every shape is one of the nine the script ladder has taught,
    // which is why the passage is words, answers and figures rather than prose.
    // 100 -> 102: the two writing stages Persian did not prove. A delayed copy
    // of alef with the model covered, and the same stroke written from the long
    // vowel alone. Both stay on ONE letter: the stages are about what the hand
    // is asked to do, not about how much language is on the page.
    lessons: 102,
    idioms: 4,
    senses: 4,
    cultureClaims: 4,
    unitPrefix: "FA",
  }));

it("pins Persian's lesson-one writing ladder, now complete through dictation", () => {
  const { lessons, curricula, spine } = loadEverything();
  const report = measureWritingStages(
    loadAssessmentPolicy(),
    ["persian"],
    lessons,
    curricula.filter((curriculum) => curriculum.language === "persian"),
    spine,
  );
  const persian = report.tracks[0]!;

  expect(persian.defects).toEqual([]);
  // All four stages now sit inside chapter one, on one letter. The last two are
  // new: a delayed copy of alef with the model covered, and the same stroke
  // written from the long vowel alone. Keeping them on ONE stroke is the point
  // -- the stages are about what the hand is being asked to do, not about how
  // much language is on the page, and alef is enough to ask all four.
  expect(persian.validEvidence.map(({ lessonId, stage }) => [lessonId, stage])).toEqual([
    ["FA-C01-salam", "observe-trace"],
    ["FA-W00-alef-guided-copy", "guided-copy"],
    ["FA-W00-alef-delayed-copy", "delayed-copy"],
    ["FA-W00-alef-dictation", "dictation-transcription"],
  ]);
  expect(persian.levels[0]).toMatchObject({
    level: "pre-A1",
    evidencedStages: ["observe-trace", "guided-copy", "delayed-copy", "dictation-transcription"],
    missingStages: [],
    complete: true,
  });
});

it("pins Persian's peace-root payoff inside the chapter-one practice", () => {
  const ledger = buildRootLedger(
    loadTrackLessons("persian"),
    loadChapterPolicy().rootLedgerMinReuse ?? 3,
  );
  expect(ledger.entries.find((entry) => entry.namespace === "roots" && entry.root === "s-l-m"))
    .toMatchObject({
      introducedBy: "FA-C01-salam",
      payoffs: ["FA-C01-practice"],
      payoffCount: 1,
    });
  expect(ledger.entries.find((entry) =>
    entry.namespace === "etymon-atom" && entry.root === "FA-ETYMON-SALAM-SLM-02"
  )).toMatchObject({
    introducedBy: "FA-C01-salam",
    // FA-C01-mamnoon joined when it was migrated to schema v2 and declared what
    // it actually leans on: its own cousinweb points back at salâm's
    // three-consonant family to explain m-n-n. A third spend of the peace root,
    // not a looser assertion.
    payoffs: ["FA-W00-alef-guided-copy", "FA-C01-mamnoon", "FA-C01-practice"],
    payoffCount: 3,
  });
});
