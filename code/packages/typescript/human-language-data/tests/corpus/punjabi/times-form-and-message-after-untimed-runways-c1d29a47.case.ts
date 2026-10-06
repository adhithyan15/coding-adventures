import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { estimateLessonDuration } from "../../../src/report.js";

it("times Punjabi form and message only after both independent untimed runways", () => {
  const lessons = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const byId = new Map(lessons.map((lesson) => [lesson.realization.lessonId, lesson]));
  const earlyOrientation = byId.get("PA-W10-a1-con-reloj")!;
  expect(earlyOrientation.blocks.map((block) => block.writingStage).filter(Boolean))
    .not.toContain("timed-assessment-production");
  expect(earlyOrientation.blocks.map((block) => block.markdown).join("\n"))
    .toMatch(/awards no timed-writing credit/);

  const chapter = lessons.filter((lesson) => lesson.frontmatter.chapter === "157");
  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W13-form-mixed-cues",
    "PA-W13-form-timed",
    "PA-W13-message-pacing",
    "PA-W13-message-timed",
    "PA-W13-timed-repair",
  ]);
  expect(chapter.every((lesson) => estimateLessonDuration(lesson).effectiveSeconds < 300))
    .toBe(true);

  const form = byId.get("PA-W13-form-timed")!;
  expect(form.frontmatter.prerequisites).toEqual(expect.arrayContaining([
    "PA-W11-form-six-no-model", "PA-W13-form-mixed-cues",
  ]));
  expect(form.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["timed-assessment-production"]);
  const [formActivity] = compileLessonActivities(form.blocks);
  expect(formActivity!.prompt).not.toMatch(/[੦-੯]/u);
  expect(formActivity!.answer).toBe(
    "ਨਾਂ: ਅਮਨ\nਭਾਸ਼ਾ: ਹਿੰਦੀ\nਰਿਹਾਇਸ਼: ਪਿੰਡ\nਕੰਮ: ਨੌਕਰੀ\nਉਮਰ: ੧੫\nਫ਼ੋਨ: ੦੨੫ ੧੨੫",
  );
  expect(formActivity!.responseSeconds).toBe(180);

  const message = byId.get("PA-W13-message-timed")!;
  expect(message.frontmatter.prerequisites).toEqual(expect.arrayContaining([
    "PA-W12-message-no-model", "PA-W13-message-pacing",
  ]));
  expect(message.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["timed-assessment-production"]);
  const prompt = message.blocks.map((block) => block.markdown).join("\n");
  expect(prompt).not.toMatch(/[\u0A00-\u0A7F]/u);
  expect(prompt).toMatch(/As Aman, write Manan a \*\*30–40-word\*\* Gurmukhi message/);
  expect(prompt).toMatch(/water/);
  expect(compileLessonActivities(message.blocks)).toEqual([]);

  const repair = byId.get("PA-W13-timed-repair")!;
  expect(repair.blocks.map((block) => block.writingStage).filter(Boolean))
    .not.toContain("timed-assessment-production");
  expect(repair.blocks.map((block) => block.markdown).join("\n"))
    .toMatch(/original timed form and message unchanged/);
});
