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

it("services the exact Punjabi body-word R4 debt exposed by the R1-R3 bridge", () => {
  const laterBridgeIds = new Set([
    "PA-R28-form-supported-r3",
    "PA-R28-head-na-r4",
  ]);
  const allLessons = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence));
  const lastBodyR4Bridge = allLessons.findIndex(
    (lesson) => lesson.realization.lessonId === "PA-R27-nose-heart-r4",
  );
  const ordered = allLessons
    .slice(0, lastBodyR4Bridge + 1)
    .filter((lesson) => !laterBridgeIds.has(lesson.realization.lessonId));
  const bridgeIds = [
    "PA-R27-ear-mouth-r4",
    "PA-R27-nose-heart-r4",
  ];
  const bridge = bridgeIds.map((id) =>
    ordered.find((lesson) => lesson.realization.lessonId === id)!,
  );
  expect(bridge.map((lesson) => ordered.indexOf(lesson))).toEqual([170, 171]);
  expect(bridge.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 210)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("reading"))).toBe(true);
  expect(bridge.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(bridge.every((lesson) => !lesson.body.includes("hl-writing-stage"))).toBe(true);
  expect(bridge.every((lesson) => lesson.body.includes("does not award independent Gurmukhi writing evidence"))).toBe(true);
  expect(bridge.flatMap((lesson) => compileLessonActivities(lesson.blocks))).toHaveLength(4);

  const servicedAtoms = new Set([
    "PA-LEX-KANN",
    "PA-CONTRAST-KANN-KARNA",
    "PA-LEX-MUNH",
    "PA-ETYMON-MUNH-MUKHA",
    "PA-LEX-NAKK",
    "PA-ETYMON-NAKK-NOSE",
    "PA-LEX-DIL",
    "PA-ETYMON-DIL-HEART",
  ]);
  const practised = new Set(
    bridge.flatMap((lesson) => lesson.frontmatter["practises.knowledge"] ?? []),
  );
  expect([...servicedAtoms].every((atom) => practised.has(atom))).toBe(true);

  const report = measureContinuity(ordered);
  expect(
    report.reinforcement.filter(
      (defect) => servicedAtoms.has(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
  // Chapters 4 and 5 stopped being hand-written .tex and became generated from their
  // lessons. Migrating those ten lessons to schema v2 (and splitting two of them)
  // declared 25 knowledge atoms the corpus had been teaching in prose and counting
  // nowhere. Every window they do not close is now VISIBLE, which is why these totals
  // rise rather than fall: the numbers moved because the measurement reaches further,
  // not because reinforcement got worse. The serviced-debt assertions above still hold
  // exactly. The residue -- Chapter 4 and 5 atoms with no later lesson putting them
  // back in front of the reader -- is named in BACKLOG.d as the next tranche's work.
  expect(report.summary.missedByWindow).toEqual({ R1: 37, R2: 88, R3: 150, R4: 97 });

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
    "R3|PA-FORM-THREE-SUPPORTED-01",
    "R3|PA-FORM-THREE-TWO-LINE-SUPPORTED-01",
    "R4|PA-ETYMON-SIR-HORN",
    "R4|PA-LEX-SIR",
    "R4|PA-SCRIPT-NA-01",
  ]);
});
