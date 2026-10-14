import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { estimateLessonDuration } from "../../../src/report.js";

const id = "PA-R175-form-repair-again";
const atom = "PA-FORM-AGE-PHONE-DATE-REPAIR-01";
const lessons = loadTrackLessons("punjabi").filter(
  (lesson) => Number(lesson.frontmatter.sequence) <= 9190,
).sort(
  (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
);
const review = lessons.find((lesson) => lesson.realization.lessonId === id)!;
const before = measureContinuity(lessons.filter((lesson) => lesson.realization.lessonId !== id));
const after = measureContinuity(lessons);
const missedPairs = (report: typeof before): Set<string> => new Set(
  report.reinforcement.flatMap((defect) =>
    defect.missed.map((window) => `${window}|${defect.atom}`),
  ),
);

it("retrieves one genuine Punjabi three-field repair at R4 without mock credit", () => {
  expect(Number(review.frontmatter.sequence)).toBe(9190);
  expect(review.realization.type).toBe("writing");
  expect(review.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(review.frontmatter["practises.knowledge"]).toEqual([atom]);
  expect(review.frontmatter.prerequisites).toContain("PA-W10-form-repair-values");
  expect(review.frontmatter.prerequisites).toContain("PA-R174-form-card-cue-again");
  expect(review.frontmatter.modality).toBe("pen");
  expect(review.body).toContain("hl-writing-stage: guided-copy");
  expect(review.body).toContain("hl-writing-stage: delayed-copy");
  expect(review.body).toContain("੦੨੫ ੧੫੨");
  expect(review.body).toContain("੦੨੫ ੧੨੫");
  expect(review.body).toContain("੧੫/੦੧੨੦੨੫");
  expect(review.body).toContain("੧੫/੦੧/੨੦੨੫");
  expect(review.body).toContain("not a scored A1 mock");
  expect(estimateLessonDuration(review).effectiveSeconds).toBeLessThanOrEqual(300);

  const previousPairs = missedPairs(before);
  const currentPairs = missedPairs(after);
  expect(before.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 512, R4: 573 });
  expect([...previousPairs].filter((pair) => !currentPairs.has(pair))).toEqual([`R4|${atom}`]);
  expect([...currentPairs].filter((pair) => !previousPairs.has(pair))).toEqual([]);
  expect(after.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 512, R4: 572 });
});
