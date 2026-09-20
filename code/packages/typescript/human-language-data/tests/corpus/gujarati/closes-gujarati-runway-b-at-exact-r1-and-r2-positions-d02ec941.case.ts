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
  expect([
    ordered[153]?.realization.lessonId,
    ordered[158]?.realization.lessonId,
    ordered[164]?.realization.lessonId,
  ]).toEqual([
    "GU-C22-shaalaa",
    "GU-R23-route-three-r1",
    "GU-R23-shaalaa-rasto-r2",
  ]);
  expect(
    [ordered[158], ordered[164]].every(
      (lesson) => ((lesson?.frontmatter["introduces.knowledge"] ?? []) as string[]).length === 0,
    ),
  ).toBe(true);

  const exactReturns = [
    ["GU-SCRIPT-SHAHAR-01", 153, 2],
    ["GU-PERFORMANCE-ROUTE-THREE-FOUR-SKILL-01", 158, 2],
    ["GU-LEX-SHAALAA-01", 164, 12],
    ["GU-SCRIPT-SHAALAA-01", 164, 11],
    ["GU-LEX-RASTO-01", 164, 10],
    ["GU-SCRIPT-RASTO-01", 164, 9],
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
