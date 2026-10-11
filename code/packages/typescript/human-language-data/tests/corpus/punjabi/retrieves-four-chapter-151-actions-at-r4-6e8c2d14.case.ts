import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { estimateLessonDuration } from "../../../src/report.js";

const lessons = loadTrackLessons("punjabi").filter(
  (lesson) => Number(lesson.frontmatter.sequence) <= 9020,
).sort(
  (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
);
const ids = [
  "PA-R165-live-again",
  "PA-R165-spend-again",
  "PA-R165-save-again",
  "PA-R165-sow-again",
];
const introducers = [
  "PA-C151-jiuna",
  "PA-C151-kharcna",
  "PA-C151-bacauna",
  "PA-C151-bijna",
];
const atoms = [
  "PA-LEX-C151-ACT151-02",
  "PA-LEX-C151-ACT151-03",
  "PA-LEX-C151-ACT151-04",
  "PA-LEX-C151-ACT151-05",
];
const reviews = lessons.filter((lesson) => ids.includes(lesson.realization.lessonId));
const before = measureContinuity(
  lessons.filter((lesson) => !ids.includes(lesson.realization.lessonId)),
);
const after = measureContinuity(lessons);
const missedPairs = (report: typeof before): Set<string> => new Set(
  report.reinforcement.flatMap((defect) =>
    defect.missed.map((window) => `${window}|${defect.atom}`),
  ),
);

it("retrieves four familiar Chapter 151 actions at R4 without granting A1 mock credit", () => {
  expect(reviews.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(reviews.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual([
    8990, 9000, 9010, 9020,
  ]);
  expect(reviews.every((lesson) => lesson.realization.type === "review")).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(reviews.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(reviews.every((lesson) => !lesson.body.includes("timed-assessment-production"))).toBe(true);
  expect(reviews.every((lesson) => estimateLessonDuration(lesson).effectiveSeconds <= 300)).toBe(true);
  expect(reviews.map((lesson, index) => lesson.frontmatter.prerequisites?.includes(introducers[index]!)))
    .toEqual([true, true, true, true]);
  expect(reviews.map((lesson, index) => lesson.frontmatter["practises.knowledge"]?.includes(atoms[index])))
    .toEqual([true, true, true, true]);

  const previousPairs = missedPairs(before);
  const currentPairs = missedPairs(after);
  expect(before.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 515, R4: 579 });
  expect([...previousPairs].filter((pair) => !currentPairs.has(pair)).sort()).toEqual(
    atoms.map((atom) => `R4|${atom}`).sort(),
  );
  // The two later moving-boundary pairs belong to #17006, after #16974.
  expect([...currentPairs].filter((pair) => !previousPairs.has(pair)).sort()).toEqual([
    "R4|PA-LEX-C152-ACT152-04",
    "R4|PA-LEX-C152-ACT152-05",
  ]);
  expect(after.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 515, R4: 577 });
});
