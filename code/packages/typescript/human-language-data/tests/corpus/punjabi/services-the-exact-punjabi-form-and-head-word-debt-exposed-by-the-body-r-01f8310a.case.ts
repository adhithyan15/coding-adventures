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

it("services the exact Punjabi form and head-word debt exposed by the body R4 bridge", () => {
  const allLessons = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bridgeIds = [
    "PA-R28-form-supported-r3",
    "PA-R28-head-na-r4",
  ];
  const lastBridgeIndex = allLessons.findIndex(
    (lesson) => lesson.realization.lessonId === bridgeIds.at(-1),
  );
  const ordered = allLessons.slice(0, lastBridgeIndex + 1);
  const bridge = bridgeIds.map((id) =>
    ordered.find((lesson) => lesson.realization.lessonId === id)!,
  );
  expect(bridge.map((lesson) => ordered.indexOf(lesson))).toEqual([172, 173]);
  expect(bridge.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 210)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);

  const [supportedForm, headAndNa] = bridge;
  expect(supportedForm!.frontmatter.skills).toEqual(["reading", "writing"]);
  expect(supportedForm!.body).toContain("hl-writing-stage: guided-copy");
  expect(supportedForm!.body).toContain("does not award independent Punjabi writing evidence");
  expect(headAndNa!.frontmatter.skills).toEqual(["listening", "speaking", "reading"]);
  expect(headAndNa!.body).not.toContain("hl-writing-stage");
  expect(headAndNa!.body).toContain("does not award independent Gurmukhi writing evidence");
  expect(bridge.flatMap((lesson) => compileLessonActivities(lesson.blocks))).toHaveLength(4);

  // The three recognition atoms are the #13068 spaced review: this lesson puts
  // those glyphs back on the page inside their R3/R4 window.
  expect(supportedForm!.frontmatter["practises.knowledge"]).toEqual([
    "PA-FORM-THREE-TWO-LINE-SUPPORTED-01",
    "PA-FORM-THREE-SUPPORTED-01",
    "PA-SCRIPT-RECOG-BHA-01",
    "PA-SCRIPT-RECOG-BIHARI-01",
    "PA-SCRIPT-RECOG-TA-01",
    "PA-SCRIPT-RECOG-LAVA-01",
  ]);
  expect(headAndNa!.frontmatter["practises.knowledge"]).toEqual([
    "PA-LEX-SIR",
    "PA-ETYMON-SIR-HORN",
    "PA-SCRIPT-NA-01",
    // the sihari met by eye in Chapter 4, back on the page inside its window
    "PA-SCRIPT-RECOG-SIHARI-01",
  ]);

  const servicedPairs = new Set([
    "R3|PA-FORM-THREE-SUPPORTED-01",
    "R3|PA-FORM-THREE-TWO-LINE-SUPPORTED-01",
    "R4|PA-ETYMON-SIR-HORN",
    "R4|PA-LEX-SIR",
    "R4|PA-SCRIPT-NA-01",
  ]);
  const report = measureContinuity(ordered);
  const stillMissing = report.reinforcement.flatMap((defect) =>
    defect.missed
      .filter((window) => servicedPairs.has(`${window}|${defect.atom}`))
      .map((window) => `${window}|${defect.atom}`),
  );
  expect(stillMissing).toEqual([]);
  // Chapters 4 and 5 stopped being hand-written .tex and became generated from their
  // lessons. Migrating those ten lessons to schema v2 (and splitting two of them)
  // declared 25 knowledge atoms the corpus had been teaching in prose and counting
  // nowhere. Every window they do not close is now VISIBLE, which is why these totals
  // rise rather than fall: the numbers moved because the measurement reaches further,
  // not because reinforcement got worse. The serviced-debt assertions above still hold
  // exactly. The residue -- Chapter 4 and 5 atoms with no later lesson putting them
  // back in front of the reader -- is named in BACKLOG.d as the next tranche's work.
  expect(report.summary.missedByWindow).toEqual({ R1: 37, R2: 88, R3: 150, R4: 91 });

  const before = measureContinuity(
    ordered.filter((lesson) => !bridgeIds.includes(lesson.realization.lessonId)),
  );
  const beforePairs = new Set(
    before.reinforcement.flatMap((defect) =>
      defect.missed.map((window) => `${window}|${defect.atom}`),
    ),
  );
  const afterPairs = new Set(
    report.reinforcement.flatMap((defect) =>
      defect.missed.map((window) => `${window}|${defect.atom}`),
    ),
  );
  expect([...afterPairs].filter((pair) => !beforePairs.has(pair)).sort()).toEqual([
    "R3|PA-FORM-THREE-SELECTION-REPAIR-01",
    "R3|PA-FORM-THREE-SPELLING-REPAIR-01",
    "R4|PA-SCRIPT-II-MATRA-01",
    "R4|PA-SCRIPT-MA-01",
  ]);
});
