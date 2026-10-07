import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";

const lessons = loadTrackLessons("punjabi").filter(
  (lesson) => Number(lesson.frontmatter.sequence) <= 8880,
).sort(
  (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
);
const ids = [
  "PA-R161-swim-again",
  "PA-R161-climb-again",
  "PA-R161-call-again",
  "PA-R161-use-again",
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

it("retrieves four Chapter 149 actions in separate short R4 lessons without mock credit", () => {
  expect(reviews.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(reviews.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual([
    8850, 8860, 8870, 8880,
  ]);
  expect(reviews.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 300)).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(reviews.every((lesson) => lesson.realization.type === "review")).toBe(true);
  expect(reviews.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(reviews.every((lesson) => !lesson.body.includes("timed-assessment-production"))).toBe(true);

  const atoms = [1, 2, 3, 4].map((number) => `PA-LEX-C149-ACT149-0${number}`);
  expect(reviews.map((lesson) => lesson.frontmatter["practises.knowledge"]?.includes(
    atoms[ids.indexOf(lesson.realization.lessonId)],
  ))).toEqual([true, true, true, true]);

  const previousPairs = missedPairs(before);
  const currentPairs = missedPairs(after);
  expect(before.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 513, R4: 576 });
  expect(after.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 516, R4: 576 });
  expect([...previousPairs].filter((pair) => !currentPairs.has(pair)).sort()).toEqual(
    atoms.map((atom) => `R4|${atom}`).sort(),
  );
  // #16915 and #16916 own the three R3 writing and four R4 action
  // pairs newly visible here. This chapter must not claim to teach them.
  expect([...currentPairs].filter((pair) => !previousPairs.has(pair)).sort()).toEqual([
    "R3|PA-A1-FORM-MIXED-CUES-01",
    "R3|PA-A1-FORM-TIMED-01",
    "R3|PA-A1-MESSAGE-PACING-01",
    "R4|PA-LEX-C149-ACT149-05",
    "R4|PA-LEX-C150-ACT150-01",
    "R4|PA-LEX-C150-ACT150-02",
    "R4|PA-LEX-C150-ACT150-03",
  ]);
});
