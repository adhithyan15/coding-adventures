import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { estimateLessonDuration } from "../../../src/report.js";

const id = "PA-R170-date-selector-again";
const atom = "PA-FORM-DATE-CUE-MAP-01";
const lessons = loadTrackLessons("punjabi").filter(
  (lesson) => Number(lesson.frontmatter.sequence) <= 9130,
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

it("retrieves the taught Punjabi date selector map at R4 without scored A1 writing", () => {
  expect(Number(review.frontmatter.sequence)).toBe(9130);
  expect(review.realization.type).toBe("review");
  expect(review.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(review.frontmatter["practises.knowledge"]).toEqual([atom]);
  expect(review.frontmatter.prerequisites).toContain("PA-W09-date-select");
  expect(review.frontmatter.prerequisites).toContain("PA-R169-date-b-again");
  expect(review.frontmatter.modality).toBe("pen");
  expect(review.frontmatter.skills).toContain("writing");
  expect(review.body).toContain("hl-writing-stage: guided-copy");
  expect(review.body).toContain("**ਕ** asks for A; **ਖ** asks for B");
  expect(review.body).not.toContain("timed-assessment-production");
  expect(estimateLessonDuration(review).effectiveSeconds).toBeLessThanOrEqual(300);

  const previousPairs = missedPairs(before);
  const currentPairs = missedPairs(after);
  expect(before.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 515, R4: 574 });
  expect([...previousPairs].filter((pair) => !currentPairs.has(pair))).toEqual([`R4|${atom}`]);
  expect([...currentPairs].filter((pair) => !previousPairs.has(pair))).toEqual([]);
  // Date repair stays in the separately tracked #17085 slice.
  expect(currentPairs.has("R4|PA-FORM-DATE-REPAIR-01")).toBe(true);
  expect(after.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 515, R4: 573 });
});
