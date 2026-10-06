import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { loadTrackLessons } from "../../../src/loader.js";

it("integrates only taught Punjabi A1 age, phone, and date fields before the six-field form", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "154");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W10-form-order",
    "PA-W10-form-select",
    "PA-W10-form-supported",
    "PA-W10-form-delayed",
    "PA-W10-form-repair-values",
    "PA-W10-form-repair-layout",
    "PA-W10-form-no-model",
  ]);
  expect(chapter.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 300))
    .toBe(true);

  const byId = new Map(chapter.map((lesson) => [lesson.realization.lessonId, lesson]));
  expect(byId.get("PA-W10-form-order")!.frontmatter.prerequisites).toEqual(
    expect.arrayContaining([
      "PA-W07-age-no-model",
      "PA-W08-phone-no-model",
      "PA-W09-date-no-model",
    ]),
  );
  expect(byId.get("PA-W10-form-supported")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["guided-copy"]);
  expect(byId.get("PA-W10-form-delayed")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["delayed-copy"]);

  const [selector] = compileLessonActivities(byId.get("PA-W10-form-select")!.blocks);
  expect(selector!.answer).toBe("B, B, B");
  const [supported] = compileLessonActivities(byId.get("PA-W10-form-supported")!.blocks);
  expect(supported!.answer).toBe("ਉਮਰ: ੧੫\nਫ਼ੋਨ: ੦੧੨ ੨੫੧\nਤਾਰੀਖ਼: ੧੫/੦੧/੨੦੨੫");
  const [delayed] = compileLessonActivities(byId.get("PA-W10-form-delayed")!.blocks);
  expect(delayed!.answer).toBe("ਉਮਰ: ੨੫\nਫ਼ੋਨ: ੦੨੫ ੧੨੫\nਤਾਰੀਖ਼: ੨੫/੦੨/੨੦੨੫");

  const independent = byId.get("PA-W10-form-no-model")!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["controlled-composition", "controlled-composition"]);
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity!.prompt).not.toMatch(/[0-9੦-੯]/);
  expect(activity!.answer).toBe("ਉਮਰ: ੨੫\nਫ਼ੋਨ: ੦੨੫ ੧੨੫\nਤਾਰੀਖ਼: ੨੫/੦੨/੨੦੨੫");
  expect(independent.frontmatter.headword).not.toMatch(/[੦-੯]/);
});
