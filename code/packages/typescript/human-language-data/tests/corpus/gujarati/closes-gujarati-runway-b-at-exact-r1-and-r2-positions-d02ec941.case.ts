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

it("closes Gujarati runway B at exact R1 and R2 positions", () => {
  const lessons = loadTrackLessons("gujarati");
  const ordered = [...lessons].sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  // HL-C443: every index +13, from the thirteen runway anchor words in
  // chapters 1-7. The distances are unchanged.
  // Gujarati A1: every index +2, from the two chapter-12 -more continuations.
  // The distances are unchanged again.
  expect([
    ordered[168]?.realization.lessonId,
    ordered[173]?.realization.lessonId,
    ordered[179]?.realization.lessonId,
  ]).toEqual([
    "GU-C22-shaalaa",
    "GU-R23-route-three-r1",
    "GU-R23-shaalaa-rasto-r2",
  ]);
  expect(
    [ordered[173], ordered[179]].every(
      (lesson) => ((lesson?.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactReturns = [
    ["GU-SCRIPT-SHAHAR-01", 168, 2],
    ["GU-PERFORMANCE-ROUTE-THREE-FOUR-SKILL-01", 173, 2],
    ["GU-LEX-SHAALAA-01", 179, 12],
    ["GU-SCRIPT-SHAALAA-01", 179, 11],
    ["GU-LEX-RASTO-01", 179, 10],
    ["GU-SCRIPT-RASTO-01", 179, 9],
  ] as const;
  expect(
    exactReturns.map(([atom, returnAt]) => {
      const introducedAt = ordered.findIndex((lesson) =>
        ((lesson.frontmatter["introduces.knowledge"] ?? []) as string[]).includes(atom),
      );
      expect(
        ((ordered[returnAt]?.frontmatter["practises.knowledge"] ?? []) as string[]),
      ).toContain(atom);
      return returnAt - introducedAt;
    }),
  ).toEqual(exactReturns.map(([, , distance]) => distance));

  const targets = new Set(exactReturns.map(([atom]) => atom));
  expect(
    measureContinuity(lessons).reinforcement.filter((defect) => targets.has(defect.atom)),
  ).toEqual([]);
});
