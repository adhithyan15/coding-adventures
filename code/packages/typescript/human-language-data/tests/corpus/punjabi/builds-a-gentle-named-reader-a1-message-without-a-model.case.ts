import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { loadTrackLessons } from "../../../src/loader.js";

it("builds a gentle Punjabi named-reader message from separately practised parts", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "156");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W12-greeting-choice",
    "PA-W12-greeting-supported",
    "PA-W12-closing-choice",
    "PA-W12-closing-supported",
    "PA-W12-boundary",
    "PA-W12-two-sentence-order",
    "PA-W12-two-sentence-supported",
    "PA-W12-two-sentence-delayed",
    "PA-W12-agreement-repair",
    "PA-W12-spacing-repair",
    "PA-W12-spelling-repair",
    "PA-W12-punctuation-repair",
    "PA-W12-message-supported",
    "PA-W12-message-delayed",
    "PA-W12-message-no-model",
  ]);
  expect(chapter.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 300))
    .toBe(true);

  const byId = new Map(chapter.map((lesson) => [lesson.realization.lessonId, lesson]));
  expect(byId.get("PA-W12-greeting-supported")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toContain("guided-copy");
  expect(byId.get("PA-W12-message-delayed")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toContain("delayed-copy");

  const independent = byId.get("PA-W12-message-no-model")!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean))
    .toContain("controlled-composition");
  const activities = compileLessonActivities(independent.blocks);
  const message = activities.find((activity) => activity.id === "PA-W12-message-no-model-write");
  expect(message).toBeDefined();
  expect(message!.prompt).not.toMatch(/[\u0A00-\u0A7F]/u);
  expect(message!.prompt).not.toMatch(/romaniz|copy|model/i);
  const wordCount = message!.answer.trim().split(/\s+/u).length;
  expect(wordCount).toBeGreaterThanOrEqual(30);
  expect(wordCount).toBeLessThanOrEqual(40);
  expect(message!.answer).toContain("ਨਮਸਤੇ");
  expect(message!.answer).toContain("ਮਨਨ");
  expect(message!.answer).toContain("ਫਿਰ ਮਿਲਾਂਗੇ");
});
