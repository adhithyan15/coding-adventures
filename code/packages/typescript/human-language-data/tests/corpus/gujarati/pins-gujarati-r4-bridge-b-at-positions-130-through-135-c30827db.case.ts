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

it("pins Gujarati R4 bridge B at positions 130 through 135", () => {
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
  const bridgeIds = [
    "GU-R16-naam-r4",
    "GU-R16-maarun-r4",
    "GU-R16-chhe-r4",
    "GU-R16-my-name-is-r4",
    "GU-R16-anand-r4",
    "GU-R16-wellbeing-r3",
  ];
  expect(ordered.slice(130, 136).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(130, 136).every((lesson) =>
      ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactR4 = [
    "GU-CONCEPT-C02-NAAM-01",
    "GU-CONCEPT-C02-MAARUN-01",
    "GU-CONCEPT-C02-CHHE-01",
    "GU-CONCEPT-C02-MAARUNNAAMCHHE-01",
    "GU-CONCEPT-C02-ANAND-01",
  ];
  expect(
    exactR4.map((atom, offset) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 130 + offset - introducedAt;
    }),
  ).toEqual([61, 61, 61, 61, 61]);

  const wellbeingR3 = new Set([
    "GU-CONCEPT-C03-HUN-01",
    "GU-CONCEPT-C03-KEM-01",
    "GU-CONCEPT-C03-MAJAA-01",
    "GU-CONCEPT-C03-PRACTICE-01",
    "GU-CONCEPT-C03-TAMEKEMCHHO-01",
    "GU-CONCEPT-C03-VANDHONAHI-01",
  ]);
  const continuity = measureContinuity(ordered.slice(0, 136));
  expect(
    continuity.reinforcement.filter(
      (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
  expect(
    continuity.reinforcement.filter(
      (defect) => wellbeingR3.has(defect.atom) && defect.missed.includes("R3"),
    ),
  ).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 130));
  const priorTrackEnd = 129;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = continuity.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(239);
  expect(priorWindowMissesAfterBridge).toBe(233);
});
