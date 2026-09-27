import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity, REINFORCEMENT_WINDOWS } from "../../../src/continuity.js";
import { defaultCurriculumRoot, loadTrackChapters, loadTrackLessons } from "../../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Gujarati R4 bridge C at positions 138 through 143", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  // HL-C443 moved every position here by 13: the opening runways now open
  // with thirteen anchor words (chapter 1 gains one, chapters 3-7 twelve), each
  // read before the letters it holds. Distances from atoms introduced after
  // chapter 7 are unchanged; distances from chapter 3-7 script atoms grew with
  // the words inserted between them and the bridge, and every window asserted
  // below still closes.
  // Gujarati A1: every position here +2. Chapter 12's two over-budget number
  // lessons split into -more continuations at positions 96 and 98, ahead of
  // this bridge. Atoms introduced before them are two lessons further away;
  // every window asserted below still closes.
  const bridgeIds = [
    "GU-R17-you-r4",
    "GU-R17-shun-r4",
    "GU-R17-whats-your-name-r4",
    "GU-R17-introduction-r4",
    "GU-R17-hun-r4",
    "GU-R17-kem-r4",
  ];
  expect(ordered.slice(138, 144).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(138, 144).every((lesson) =>
      ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactR4 = [
    "GU-CONCEPT-C02-TUTAME-01",
    "GU-CONCEPT-C02-SHUN-01",
    "GU-CONCEPT-C02-TAMARUNNAAMSHUNCHHE-01",
    "GU-CONCEPT-C02-PRACTICE-01",
    "GU-CONCEPT-C03-HUN-01",
    "GU-CONCEPT-C03-KEM-01",
  ];
  expect(
    exactR4.map((atom, offset) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 138 + offset - introducedAt;
    }),
  ).toEqual([64, 64, 64, 63, 63, 63]);

  const continuity = measureContinuity(ordered.slice(0, 144));
  expect(
    continuity.reinforcement.filter(
      (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 138));
  const priorTrackEnd = 137;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = continuity.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(252); // Gujarati A1: 254 -> 252. The splits' R1/R3 gains and the new retrievals outweigh windows the +2 prefix newly measures.
  expect(priorWindowMissesAfterBridge).toBe(249); // Gujarati A1: 251 -> 249, the same R1/R3 gains.
});
