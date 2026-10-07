import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { estimateLessonDuration } from "../../../src/report.js";

const lessons = loadTrackLessons("punjabi").filter(
  (lesson) => Number(lesson.frontmatter.sequence) <= 8910,
).sort(
  (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
);
const ids = [
  "PA-W16-form-mixed-return",
  "PA-W16-form-timed-return",
  "PA-W16-message-pacing-return",
];
const atoms = [
  "PA-A1-FORM-MIXED-CUES-01",
  "PA-A1-FORM-TIMED-01",
  "PA-A1-MESSAGE-PACING-01",
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

it("returns three familiar A1 writing atoms at R3 without mock credit", () => {
  expect(reviews.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(reviews.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual([
    8890, 8900, 8910,
  ]);
  expect(reviews.every((lesson) => lesson.realization.type === "writing")).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter.delivery === "script")).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(reviews.every((lesson) => estimateLessonDuration(lesson).effectiveSeconds < 300)).toBe(true);
  expect(reviews.map((lesson, index) => lesson.frontmatter["practises.knowledge"]?.includes(atoms[index])))
    .toEqual([true, true, true]);
  expect(reviews[0]?.body).toContain("hl-writing-stage: guided-copy");
  expect(reviews[1]?.body).toContain("hl-writing-stage: timed-assessment-production");
  expect(reviews[2]?.body).toContain("hl-writing-stage: controlled-composition");
  expect(reviews.every((lesson) => lesson.body.includes("not a complete A1 mock"))).toBe(true);

  const previousPairs = missedPairs(before);
  const currentPairs = missedPairs(after);
  expect(before.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 516, R4: 576 });
  expect([...previousPairs].filter((pair) => !currentPairs.has(pair)).sort()).toEqual(
    atoms.map((atom) => `R3|${atom}`).sort(),
  );
  // The longer tail exposes five distinct pairs, owned by #16937 and #16938.
  expect([...currentPairs].filter((pair) => !previousPairs.has(pair)).sort()).toEqual([
    "R3|PA-A1-MESSAGE-TIMED-01",
    "R3|PA-A1-TIMED-REPAIR-01",
    "R4|PA-LEX-C150-ACT150-04",
    "R4|PA-LEX-C150-ACT150-05",
    "R4|PA-LEX-C151-ACT151-01",
  ]);
  expect(after.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 515, R4: 579 });
});
