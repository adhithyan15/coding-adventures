import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { estimateLessonDuration } from "../../../src/report.js";

const lessons = loadTrackLessons("punjabi").filter(
  (lesson) => Number(lesson.frontmatter.sequence) <= 9100,
).sort(
  (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
);
const ids = [
  "PA-R168-date-label-again",
  "PA-R168-month-map-again",
  "PA-R168-date-format-again",
];
const introducers = ["PA-W09-date-label", "PA-W09-month-map", "PA-W09-date-format"];
const atoms = [
  "PA-FORM-LABEL-DATE-01",
  "PA-FORM-DATE-MONTH-MAP-01",
  "PA-FORM-DATE-FORMAT-01",
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

it("retrieves three already-taught Punjabi date-form atoms at R4 without scored mock credit", () => {
  expect(reviews.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(reviews.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual([9080, 9090, 9100]);
  expect(reviews.every((lesson) => lesson.realization.type === "review")).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(reviews.every((lesson) => lesson.body.includes("hl-writing-stage: guided-copy"))).toBe(true);
  expect(reviews.every((lesson) => !lesson.body.includes("timed-assessment-production"))).toBe(true);
  expect(reviews.every((lesson) => estimateLessonDuration(lesson).effectiveSeconds <= 300)).toBe(true);
  expect(reviews.map((lesson, index) => lesson.frontmatter.prerequisites?.includes(introducers[index]!)))
    .toEqual([true, true, true]);
  expect(reviews.map((lesson, index) => lesson.frontmatter["practises.knowledge"]?.includes(atoms[index])))
    .toEqual([true, true, true]);
  expect(reviews[0]!.body).toContain("ਤਾਰੀਖ਼:");
  expect(reviews[1]!.body).toContain("੦੧");
  expect(reviews[1]!.body).toContain("੦੨");
  expect(reviews[2]!.body).toContain("day / month / year");

  const previousPairs = missedPairs(before);
  const currentPairs = missedPairs(after);
  expect(before.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 515, R4: 577 });
  expect([...previousPairs].filter((pair) => !currentPairs.has(pair)).sort()).toEqual(
    atoms.map((atom) => `R4|${atom}`).sort(),
  );
  // The later selector-map boundary is separately tracked in #17058, after #17041.
  expect([...currentPairs].filter((pair) => !previousPairs.has(pair)).sort()).toEqual([
    "R4|PA-FORM-DATE-CUE-MAP-01",
  ]);
  expect(after.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 515, R4: 575 });
});
