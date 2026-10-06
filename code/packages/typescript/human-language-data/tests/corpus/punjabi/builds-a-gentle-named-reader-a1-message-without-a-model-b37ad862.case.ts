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
    "PA-W12-wellbeing-supported",
    "PA-W12-interests-supported",
    "PA-W12-meeting-supported",
    "PA-W12-body-blocks-delayed",
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
  for (const id of ["PA-W12-two-sentence-delayed", "PA-W12-body-blocks-delayed"]) {
    expect(String(byId.get(id)!.realization.headword)).not.toMatch(/[\u0A00-\u0A7F]/u);
  }

  const independent = byId.get("PA-W12-message-no-model")!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean))
    .toContain("controlled-composition");
  const finalPrompt = independent.blocks.map((block) => block.markdown).join("\n");
  expect(finalPrompt).not.toMatch(/[\u0A00-\u0A7F]/u);
  expect(finalPrompt).not.toMatch(/romaniz|transliterat/i);
  expect(finalPrompt).toMatch(/30.{0,3}40.word/iu);
  expect(finalPrompt).toMatch(/reader|Manan/iu);
  expect(finalPrompt).toMatch(/As Manan, write Aman/iu);
  expect(finalPrompt).toMatch(/liking\s+for water/iu);
  expect(finalPrompt).toMatch(/wellbeing\s+reply\s+\*after\*/iu);
  expect(finalPrompt).toMatch(/agreement|spelling|spacing/iu);
  // Free composition has many valid outputs. An exact-answer activity would
  // incorrectly mark a different valid 30–40-word message wrong.
  expect(compileLessonActivities(independent.blocks)).toEqual([]);
});
