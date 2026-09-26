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
  // HL-C443: +1, from ਮੌਸਮ (mausam), the anchor word placed before the
  // chapter-20 ੌ lesson. Every bridge lesson sits after it.
  expect(bridge.map((lesson) => orderedAtR4.indexOf(lesson))).toEqual([159, 160, 161, 162, 163, 164, 165, 166]);
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
  // 95 -> 97: HL-C443's ਮੌਸਮ (mausam) makes this prefix one lesson longer, so
  // the R4 window of ਨੱਕ (the word and its etymon) now begins inside it. The
  // whole track still services both, in PA-R27-nose-heart-r4, which this
  // prefix leaves out by design.
  expect(report.summary.missedByWindow.R4).toBe(97);
});
