import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { loadTrackLessons } from "../../../src/loader.js";

it("joins six taught Punjabi form fields only after bounded rehearsals", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "155");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W11-form-six-order",
    "PA-W11-form-six-select",
    "PA-W11-form-six-supported",
    "PA-W11-form-six-delayed",
    "PA-W11-form-six-repair",
    "PA-W11-form-six-no-model",
  ]);
  expect(chapter.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 300))
    .toBe(true);

  const byId = new Map(chapter.map((lesson) => [lesson.realization.lessonId, lesson]));
  expect(byId.get("PA-W11-form-six-order")!.frontmatter.prerequisites).toEqual(
    expect.arrayContaining([
      "PA-W02-name-no-model",
      "PA-W06-three-field-no-model",
      "PA-W10-form-no-model",
    ]),
  );
  expect(byId.get("PA-W11-form-six-supported")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["guided-copy"]);
  expect(byId.get("PA-W11-form-six-delayed")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["delayed-copy"]);

  const independent = byId.get("PA-W11-form-six-no-model")!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["controlled-composition", "controlled-composition"]);
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity!.prompt).not.toMatch(/[0-9੦-੯]/);
  expect(activity!.answer).toBe(
    "ਨਾਂ: ਮਨਨ\nਭਾਸ਼ਾ: ਹਿੰਦੀ\nਰਿਹਾਇਸ਼: ਸ਼ਹਿਰ\nਕੰਮ: ਨੌਕਰੀ\nਉਮਰ: ੨੫\nਫ਼ੋਨ: ੦੨੫ ੧੨੫",
  );
  expect(independent.frontmatter.headword).not.toMatch(/[੦-੯]/);
});
