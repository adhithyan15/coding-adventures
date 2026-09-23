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

it("closes R3 for every Gujarati doorway form after the first name exchange", () => {
  const doorway = new Set([
    "GU-SCRIPT-JA-01",
    "GU-SCRIPT-O-MATRA-01",
    "GU-SCRIPT-ANUSVARA-01",
    "GU-SCRIPT-II-MATRA-01",
    "GU-SCRIPT-U-MATRA-01",
    "GU-SCRIPT-CHHA-01",
    "GU-SCRIPT-KA-01",
    "GU-SCRIPT-NNA-01",
    "GU-SCRIPT-SHA-01",
  ]);
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const checkpointIndex = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "GU-R04-doorway-nine-r2",
  );
  const checkpoint = ordered[checkpointIndex]!;
  expect(checkpoint.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(
    [...doorway].map((atom) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      return checkpointIndex - introducedAt;
    }),
  ).toEqual([38, 37, 36, 35, 34, 33, 32, 31, 30]);

  const stillMissingR3 = measureContinuity(lessons).reinforcement.filter(
    (defect) => doorway.has(defect.atom) && defect.missed.includes("R3"),
  );
  expect(stillMissingR3).toEqual([]);
});
