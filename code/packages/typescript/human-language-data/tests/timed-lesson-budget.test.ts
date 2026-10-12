import { expect, it } from "vitest";
import { directedLessonSeconds } from "./corpus/timed-lesson-budget.js";

it("counts the longest explicit clock and separately named subtasks", () => {
  const lesson = `## Warm-up
Set a three-minute timer.
## Writing
### Task 1 — practical form: 7 minutes
### Task 2 — message: 13 minutes
## Wrap-up Recall
Spend at most one minute reviewing.`;
  expect(directedLessonSeconds(lesson)).toBe(21 * 60);
});

it("counts review beyond the writing clock and requires an explicit review cap", () => {
  const lesson = `## Warm-up
Set a three-minute timer.
## Writing
Stop when it rings.
## Wrap-up Recall
Spend at most two minutes reviewing.`;
  expect(directedLessonSeconds(lesson)).toBe(5 * 60);
  expect(directedLessonSeconds(lesson.replace("at most two minutes", "some time"))).toBe(
    Number.POSITIVE_INFINITY,
  );
});

it("rejects a single over-budget timer even without named subtasks", () => {
  const lesson = `## Warm-up
Set one timer for 20 minutes.
## Writing
Write independently.
## Wrap-up Recall
Spend at most one minute reviewing.`;
  expect(directedLessonSeconds(lesson)).toBeGreaterThan(300);
});
