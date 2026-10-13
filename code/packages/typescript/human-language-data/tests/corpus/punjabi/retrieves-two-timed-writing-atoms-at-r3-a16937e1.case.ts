import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { estimateLessonDuration } from "../../../src/report.js";

const ids = ["PA-R173-message-timed-again", "PA-R173-timed-repair-again"];
const atoms = ["PA-A1-MESSAGE-TIMED-01", "PA-A1-TIMED-REPAIR-01"];
const lessons = loadTrackLessons("punjabi").filter(
  (lesson) => Number(lesson.frontmatter.sequence) <= 9170,
).sort(
  (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
);
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

it("retrieves two taught Punjabi timed-writing atoms at R3 without mock credit", () => {
  expect(reviews.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(reviews.map((lesson) => Number(lesson.frontmatter.sequence))).toEqual([9160, 9170]);
  expect(reviews.every((lesson) => lesson.realization.type === "writing")).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter.modality === "pen")).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter["introduces.knowledge"]?.length === 0)).toBe(true);
  expect(reviews.every((lesson) => lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(reviews.every((lesson) => estimateLessonDuration(lesson).effectiveSeconds <= 300)).toBe(true);
  expect(reviews.map((lesson, index) => lesson.frontmatter["practises.knowledge"]?.includes(atoms[index])))
    .toEqual([true, true]);
  expect(reviews[0]?.frontmatter.prerequisites).toContain("PA-W13-message-timed");
  expect(reviews[1]?.frontmatter.prerequisites).toContain("PA-W13-timed-repair");
  expect(reviews[0]?.body).toContain("hl-writing-stage: timed-assessment-production");
  expect(reviews[1]?.body).toContain("hl-writing-stage: controlled-composition");
  expect(reviews.every((lesson) => lesson.body.includes("no model"))).toBe(true);
  expect(reviews.every((lesson) => lesson.body.includes("not a complete A1 mock"))).toBe(true);
  expect(reviews[1]?.body).toContain("first attempt unchanged");

  const previousPairs = missedPairs(before);
  const currentPairs = missedPairs(after);
  expect(before.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 514, R4: 573 });
  expect([...previousPairs].filter((pair) => !currentPairs.has(pair)).sort()).toEqual(
    atoms.map((atom) => `R3|${atom}`).sort(),
  );
  // The longer tail exposes one distinct R4 correction, owned by #17270.
  expect([...currentPairs].filter((pair) => !previousPairs.has(pair))).toEqual([
    "R4|PA-FORM-AGE-PHONE-DATE-REPAIR-01",
  ]);
  expect(currentPairs.has("R4|PA-FORM-AGE-PHONE-DATE-CUE-01")).toBe(true); // #17160
  expect(after.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 512, R4: 574 });
});
