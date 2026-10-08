import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";

const lessons = loadTrackLessons("punjabi").sort(
  (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
);
// Preserve the Chapter 160 measurement at its own boundary. Later reviews
// close the Chapter 149 pairs that this test deliberately exposes.
const chapter160Lessons = lessons.filter((lesson) => Number(lesson.frontmatter.sequence) <= 8840);
const ids = [
  "PA-R160-fall-again",
  "PA-R160-teach-again",
  "PA-R160-lift-again",
  "PA-R160-jump-again",
];
const reviews = chapter160Lessons.filter((lesson) => ids.includes(lesson.realization.lessonId));
const before = measureContinuity(
  chapter160Lessons.filter((lesson) => !ids.includes(lesson.realization.lessonId)),
);
const after = measureContinuity(chapter160Lessons);
const missedPairs = (report: typeof before): Set<string> => new Set(
  report.reinforcement.flatMap((defect) =>
    defect.missed.map((window) => `${window}|${defect.atom}`),
  ),
);

it("retrieves four Chapter 148 actions in separate short R4 lessons without mock credit", () => {
  expect(reviews.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(reviews.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual([
    8810, 8820, 8830, 8840,
  ]);
  expect(reviews.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 300)).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(reviews.every((lesson) => lesson.realization.type === "review")).toBe(true);
  expect(reviews.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(reviews.every((lesson) => !lesson.body.includes("timed-assessment-production"))).toBe(true);

  const atoms = [2, 3, 4, 5].map((number) => `PA-LEX-C148-ACT148-0${number}`);
  expect(reviews.map((lesson) => lesson.frontmatter["practises.knowledge"]?.includes(
    atoms[ids.indexOf(lesson.realization.lessonId)],
  ))).toEqual([true, true, true, true]);

  const previousPairs = missedPairs(before);
  const currentPairs = missedPairs(after);
  expect(before.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 513, R4: 576 });
  expect(after.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 513, R4: 576 });
  expect([...previousPairs].filter((pair) => !currentPairs.has(pair)).sort()).toEqual(
    atoms.map((atom) => `R4|${atom}`).sort(),
  );
  // #16899 owns the four newly visible Chapter 149 action pairs. This
  // chapter must not silently claim to have taught those later words.
  expect([...currentPairs].filter((pair) => !previousPairs.has(pair)).sort()).toEqual(
    [1, 2, 3, 4].map((number) => `R4|PA-LEX-C149-ACT149-0${number}`),
  );
});
