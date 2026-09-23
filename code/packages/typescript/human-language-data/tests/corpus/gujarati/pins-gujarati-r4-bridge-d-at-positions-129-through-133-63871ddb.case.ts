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

it("pins Gujarati R4 bridge D at positions 129 through 133", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bridgeIds = [
    "GU-R18-how-are-you-r4",
    "GU-R18-majaa-r4",
    "GU-R18-no-problem-r4",
    "GU-R18-wellbeing-r4",
    "GU-R18-farewell-r4",
  ];
  expect(ordered.slice(129, 134).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(129, 134).every((lesson) =>
      ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactR4 = [
    "GU-CONCEPT-C03-TAMEKEMCHHO-01",
    "GU-CONCEPT-C03-MAJAA-01",
    "GU-CONCEPT-C03-VANDHONAHI-01",
    "GU-CONCEPT-C03-PRACTICE-01",
    "GU-CONCEPT-C04-MALISHUN-01",
  ];
  expect(
    exactR4.map((atom, offset) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 129 + offset - introducedAt;
    }),
  ).toEqual([61, 61, 61, 61, 61]);

  const farewellR3 = [
    "GU-CONCEPT-C04-KAALE-01",
    "GU-CONCEPT-C04-PACHHA-01",
    "GU-CONCEPT-C04-PACHHAMALISHUN-01",
    "GU-CONCEPT-C04-PRACTICE-01",
  ];
  expect(
    farewellR3.map((atom) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 133 - introducedAt;
    }),
  ).toEqual([60, 59, 58, 57]);

  const continuity = measureContinuity(ordered.slice(0, 134));
  expect(
    continuity.reinforcement.filter(
      (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
    ),
  ).toEqual([]);
  expect(
    continuity.reinforcement.filter(
      (defect) => farewellR3.includes(defect.atom) && defect.missed.includes("R3"),
    ),
  ).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 129));
  const priorTrackEnd = 128;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = continuity.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(245);
  expect(priorWindowMissesAfterBridge).toBe(239);
});
