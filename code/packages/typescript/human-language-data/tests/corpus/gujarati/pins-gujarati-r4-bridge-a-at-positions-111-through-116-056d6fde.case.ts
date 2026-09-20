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

it("pins Gujarati R4 bridge A at positions 111 through 116", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const bridgeIds = [
    "GU-R15-u-matra-r4",
    "GU-R15-chha-r4",
    "GU-R15-ka-r4",
    "GU-R15-nna-r4",
    "GU-R15-sha-r4",
    "GU-R15-name-exchange-r3",
  ];
  expect(ordered.slice(111, 117).map((lesson) => lesson.realization.lessonId)).toEqual(bridgeIds);
  expect(
    ordered.slice(111, 117).every((lesson) =>
      ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactR4 = [
    "GU-SCRIPT-U-MATRA-01",
    "GU-SCRIPT-CHHA-01",
    "GU-SCRIPT-KA-01",
    "GU-SCRIPT-NNA-01",
    "GU-SCRIPT-SHA-01",
  ];
  expect(
    exactR4.map((atom, offset) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return 111 + offset - introducedAt;
    }),
  ).toEqual([81, 81, 81, 81, 81]);

  const stillMissingR4 = measureContinuity(ordered.slice(0, 117)).reinforcement.filter(
    (defect) => exactR4.includes(defect.atom) && defect.missed.includes("R4"),
  );
  expect(stillMissingR4).toEqual([]);

  const beforeBridge = measureContinuity(ordered.slice(0, 111));
  const afterBridge = measureContinuity(ordered.slice(0, 117));
  const priorTrackEnd = 110;
  const firstEligibleDistance = new Map(
    REINFORCEMENT_WINDOWS.map((window) => [window.name, window.from]),
  );
  const priorWindowMissesAfterBridge = afterBridge.reinforcement.flatMap((defect) =>
    defect.missed.filter(
      (window) => defect.introducedAt + firstEligibleDistance.get(window)! <= priorTrackEnd,
    ),
  ).length;
  expect(beforeBridge.reinforcement.flatMap((defect) => defect.missed)).toHaveLength(201);
  expect(priorWindowMissesAfterBridge).toBe(188);
});
