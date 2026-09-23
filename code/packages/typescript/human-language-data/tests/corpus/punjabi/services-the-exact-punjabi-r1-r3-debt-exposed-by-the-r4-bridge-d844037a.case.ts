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

it("services the exact Punjabi R1-R3 debt exposed by the R4 bridge", () => {
  const ordered = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bodyR4BridgeIds = new Set([
    "PA-R27-ear-mouth-r4",
    "PA-R27-nose-heart-r4",
    "PA-R28-form-supported-r3",
    "PA-R28-head-na-r4",
  ]);
  const lastLowerWindowBridge = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "PA-R26-work-control-r3",
  );
  const orderedBeforeBodyR4 = ordered.slice(0, lastLowerWindowBridge + 1).filter(
    (lesson) => !bodyR4BridgeIds.has(lesson.realization.lessonId),
  );
  const bridgeIds = [
    "PA-R23-three-no-model-r1",
    "PA-R26-three-repair-r2",
    "PA-R26-work-build-r3",
    "PA-R26-work-control-r3",
  ];
  const bridge = bridgeIds.map((id) =>
    orderedBeforeBodyR4.find((lesson) => lesson.realization.lessonId === id)!,
  );
  expect(bridge.map((lesson) => orderedBeforeBodyR4.indexOf(lesson))).toEqual([158, 167, 168, 169]);
  expect(bridge.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 220)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("reading"))).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(bridge.every((lesson) => lesson.body.includes("hl-writing-stage: controlled-composition"))).toBe(true);
  expect(bridge.flatMap((lesson) => compileLessonActivities(lesson.blocks))).toHaveLength(4);

  const servicedPairs = new Set([
    "R3|PA-FORM-WORK-DELAYED-ENTRY-01",
    "R3|PA-FORM-WORK-SUPPORTED-ENTRY-01",
    "R1|PA-FORM-THREE-NO-MODEL-01",
    "R2|PA-FORM-THREE-NO-MODEL-01",
    "R3|PA-FORM-WORK-AGREEMENT-01",
    "R3|PA-SCRIPT-AU-MATRA-01",
    "R2|PA-FORM-THREE-MIXED-REPAIR-01",
    "R2|PA-FORM-THREE-PLACEMENT-REPAIR-01",
    "R2|PA-FORM-THREE-SPACING-REPAIR-01",
    "R2|PA-FORM-THREE-SPELLING-REPAIR-01",
    "R3|PA-FORM-WORK-CUE-MAP-01",
    "R3|PA-FORM-WORK-JOB-01",
    "R3|PA-FORM-WORK-SPACING-01",
    "R3|PA-FORM-WORK-SPELLING-CHECK-01",
    "R3|PA-FORM-WORK-REPAIR-01",
    "R3|PA-FORM-WORK-NO-MODEL-01",
    "R3|PA-FORM-THREE-LABEL-ORDER-01",
    "R3|PA-FORM-THREE-CUE-SELECTION-01",
  ]);
  const report = measureContinuity(orderedBeforeBodyR4);
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
  expect(report.summary.missedByWindow).toEqual({ R1: 37, R2: 88, R3: 148, R4: 102 });

  const bodyBoundaryAtoms = new Set([
    "PA-LEX-KANN",
    "PA-CONTRAST-KANN-KARNA",
    "PA-LEX-MUNH",
    "PA-ETYMON-MUNH-MUKHA",
    "PA-LEX-NAKK",
    "PA-ETYMON-NAKK-NOSE",
    "PA-LEX-DIL",
    "PA-ETYMON-DIL-HEART",
  ]);
  expect(
    report.reinforcement
      .filter((defect) => defect.missed.includes("R4") && bodyBoundaryAtoms.has(defect.atom))
      .map((defect) => defect.atom)
      .sort(),
  ).toEqual([...bodyBoundaryAtoms].sort());
});
