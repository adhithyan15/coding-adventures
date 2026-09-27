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

it("services Punjabi's three-field R4 debt without moving the boundary forward", () => {
  const ordered = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const lowerWindowBridgeIds = new Set([
    "PA-R23-three-no-model-r1",
    "PA-R26-three-repair-r2",
    "PA-R26-work-build-r3",
    "PA-R26-work-control-r3",
    "PA-R27-ear-mouth-r4",
    "PA-R27-nose-heart-r4",
    "PA-R28-form-supported-r3",
    "PA-R28-head-na-r4",
  ]);
  const lastR4Bridge = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "PA-R25-kin-eye-r4",
  );
  const orderedAtR4 = ordered.slice(0, lastR4Bridge + 1).filter(
    (lesson) => !lowerWindowBridgeIds.has(lesson.realization.lessonId),
  );
  const bridgeIds = [
    "PA-R24-know-think-r4",
    "PA-R24-understand-read-r4",
    "PA-R24-write-take-ask-r4",
    "PA-R24-help-like-r4",
    "PA-R25-drink-request-r4",
    "PA-R25-milk-bread-r4",
    "PA-R25-friend-family-r4",
    "PA-R25-kin-eye-r4",
  ];
  const bridge = bridgeIds.map((id) =>
    orderedAtR4.find((lesson) => lesson.realization.lessonId === id)!,
  );
  expect(bridge.map((lesson) => orderedAtR4.indexOf(lesson))).toEqual([158, 159, 160, 161, 162, 163, 164, 165]);
  expect(bridge.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 220)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  expect(bridge.every((lesson) => lesson.frontmatter.skills?.includes("reading"))).toBe(true);
  expect(bridge.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(bridge.every((lesson) => !lesson.body.includes("hl-writing-stage"))).toBe(true);
  expect(bridge.flatMap((lesson) => compileLessonActivities(lesson.blocks))).toHaveLength(17);

  const exposedByThreeFieldIntegration = [
    "PA-ETYMON-PASAND-SHINE",
    "PA-CONTRAST-JANA-JANNA",
    "PA-ETYMON-JANNA-KNOW",
    "PA-ETYMON-LAINA-LABH",
    "PA-ETYMON-LIKH-SCRATCH",
    "PA-ETYMON-MADAD-ARABIC",
    "PA-ETYMON-PAANI-DRINK",
    "PA-ETYMON-PARHNA-PATH",
    "PA-ETYMON-PUCHHNA-PRACH",
    "PA-ETYMON-SAMAJH-BUDH",
    "PA-ETYMON-SOCHNA-SHUC",
    "PA-GRAMMAR-DATIVE-LIKING",
    "PA-GRAMMAR-NOUN-PLUS-KARNA",
    "PA-LEX-JANNA",
    "PA-LEX-LAINA",
    "PA-LEX-LIKHNA",
    "PA-LEX-PAANI",
    "PA-LEX-PASAND",
    "PA-LEX-PUCHHNA",
    "PA-LEX-SAMAJHNA",
    "PA-LEX-SOCHNA",
    "PA-PHRASE-KIRPA-KARKE",
    "PA-SCRIPT-SUBJOINED-HA",
    "PA-LEX-MADAD-KARNA",
    "PA-LEX-PARHNA",
    "PA-SOUND-TONE-FALLING",
  ];
  expect(exposedByThreeFieldIntegration).toHaveLength(26);

  const movingBoundaryAtoms = [
    "PA-LEX-CHA",
    "PA-ETYMON-CHA-CHINESE",
    "PA-SOUND-TONE-HIGH-LEVEL",
    "PA-LEX-DUDH",
    "PA-ETYMON-DUDH-DUGDHA",
    "PA-LEX-ROTI",
    "PA-ETYMON-ROTI-UNKNOWN",
    "PA-LEX-DOST",
    "PA-ETYMON-DOST-CHOOSE",
    "PA-LEX-PARIVAR",
    "PA-ETYMON-PARIVAR-SURROUND",
    "PA-LEX-BHARA",
    "PA-ETYMON-BHARA-BROTHER",
    "PA-SOUND-TONE-LOW",
    "PA-LEX-BHAIN",
    "PA-ETYMON-BHAIN-BHAGA",
    "PA-LEX-AKKH",
    "PA-ETYMON-AKKH-EYE",
  ];
  expect(movingBoundaryAtoms).toHaveLength(18);

  const practised = new Set(
    bridge.flatMap((lesson) => lesson.frontmatter["practises.knowledge"] ?? []),
  );
  expect(
    [...exposedByThreeFieldIntegration, ...movingBoundaryAtoms].every((atom) => practised.has(atom)),
  ).toBe(true);

  const report = measureContinuity(orderedAtR4);
  const serviced = new Set([...exposedByThreeFieldIntegration, ...movingBoundaryAtoms]);
  expect(
    report.reinforcement.filter(
      (defect) => serviced.has(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
  // R4 residue from the #13068 recognition runway; see the note above.
  // Chapters 4 and 5 stopped being hand-written .tex and became generated from their
  // lessons. Migrating those ten lessons to schema v2 (and splitting two of them)
  // declared 25 knowledge atoms the corpus had been teaching in prose and counting
  // nowhere. Every window they do not close is now VISIBLE, which is why these totals
  // rise rather than fall: the numbers moved because the measurement reaches further,
  // not because reinforcement got worse. The serviced-debt assertions above still hold
  // exactly. The residue -- Chapter 4 and 5 atoms with no later lesson putting them
  // back in front of the reader -- is named in BACKLOG.d as the next tranche's work.
  expect(report.summary.missedByWindow.R4).toBe(95);
});

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
