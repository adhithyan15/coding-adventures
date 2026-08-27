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
    lessons: 68,
    idioms: 4,
    senses: 4,
    cultureClaims: 4,
    unitPrefix: "FA",
  }));

it("pins Persian's lesson-one observe-and-copy writing bridge", () => {
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
  expect(persian.validEvidence.map(({ lessonId, stage }) => [lessonId, stage])).toEqual([
    ["FA-C01-salam", "observe-trace"],
    ["FA-W00-alef-guided-copy", "guided-copy"],
  ]);
  expect(persian.levels[0]).toMatchObject({
    level: "pre-A1",
    evidencedStages: ["observe-trace", "guided-copy"],
    missingStages: ["delayed-copy", "dictation-transcription"],
    complete: false,
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
    payoffs: ["FA-W00-alef-guided-copy", "FA-C01-practice"],
    payoffCount: 2,
  });
});
