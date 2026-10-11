import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { estimateLessonDuration } from "../../../src/report.js";

const lessons = loadTrackLessons("punjabi").filter(
  (lesson) => Number(lesson.frontmatter.sequence) <= 9120,
).sort(
  (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
);
const ids = ["PA-R169-date-a-again", "PA-R169-date-b-again"];
const introducers = ["PA-W09-date-a", "PA-W09-date-b"];
const atoms = ["PA-FORM-DATE-A-01", "PA-FORM-DATE-B-01"];
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

it("retrieves both taught Punjabi date examples at R4 without scored A1 writing", () => {
  expect(reviews.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(reviews.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual([9110, 9120]);
  expect(reviews.every((lesson) => lesson.realization.type === "review")).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter.modality === "pen")).toBe(true);
  expect(reviews.every((lesson) => lesson.body.includes("hl-writing-stage: guided-copy"))).toBe(true);
  expect(reviews.every((lesson) => !lesson.body.includes("timed-assessment-production"))).toBe(true);
  expect(reviews.every((lesson) => estimateLessonDuration(lesson).effectiveSeconds <= 300)).toBe(true);
  expect(reviews.map((lesson, index) => lesson.frontmatter.prerequisites?.includes(introducers[index]!)))
    .toEqual([true, true]);
  expect(reviews.map((lesson, index) => lesson.frontmatter["practises.knowledge"]?.includes(atoms[index])))
    .toEqual([true, true]);
  expect(reviews[0]!.body).toContain("੧੫/੦੧/੨੦੨੫");
  expect(reviews[1]!.body).toContain("੨੫/੦੨/੨੦੨੫");

  const previousPairs = missedPairs(before);
  const currentPairs = missedPairs(after);
  expect(before.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 515, R4: 575 });
  expect([...previousPairs].filter((pair) => !currentPairs.has(pair)).sort()).toEqual(
    atoms.map((atom) => `R4|${atom}`).sort(),
  );
  // The newly exposed repair pair is separately tracked in #17085.
  // The selector-map pair stays separate in #17058.
  expect([...currentPairs].filter((pair) => !previousPairs.has(pair)).sort()).toEqual([
    "R4|PA-FORM-DATE-REPAIR-01",
  ]);
  expect(after.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 515, R4: 574 });
});
