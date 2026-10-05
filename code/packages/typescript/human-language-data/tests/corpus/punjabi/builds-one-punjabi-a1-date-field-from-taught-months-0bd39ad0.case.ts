import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { loadTrackLessons } from "../../../src/loader.js";

it("builds a Punjabi A1 date field from taught months to independent Gurmukhi entry", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "153");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W09-date-label",
    "PA-W09-month-map",
    "PA-W09-date-format",
    "PA-W09-date-a",
    "PA-W09-date-b",
    "PA-W09-date-select",
    "PA-W09-date-supported",
    "PA-W09-date-delayed",
    "PA-W09-date-dictation",
    "PA-W09-date-repair",
    "PA-W09-date-no-model",
  ]);
  expect(chapter.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 300))
    .toBe(true);

  const byId = new Map(chapter.map((lesson) => [lesson.realization.lessonId, lesson]));
  expect(byId.get("PA-W09-date-label")!.frontmatter.prerequisites).toContain("PA-C100-tarikh");
  expect(byId.get("PA-W09-month-map")!.frontmatter.prerequisites).toEqual(
    expect.arrayContaining(["PA-C101-janvari", "PA-C101-farvari"]),
  );
  expect(byId.get("PA-W09-date-supported")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["guided-copy"]);
  expect(byId.get("PA-W09-date-delayed")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["delayed-copy"]);
  expect(byId.get("PA-W09-date-dictation")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["dictation-transcription"]);

  for (const id of ["PA-W09-date-delayed", "PA-W09-date-dictation"]) {
    expect(String(byId.get(id)!.frontmatter.headword)).not.toMatch(/[੦-੯]/);
  }
  const [selection] = compileLessonActivities(byId.get("PA-W09-date-select")!.blocks);
  expect([selection!.answer, ...selection!.accepted].every(
    (response) => response.includes("B") && response.includes("੨੫/੦੨/੨੦੨੫"),
  )).toBe(true);
  const [dictation] = compileLessonActivities(byId.get("PA-W09-date-dictation")!.blocks);
  expect(dictation!.prompt).not.toMatch(/fifteenth|January|two thousand|[੦-੯]/i);

  const independent = byId.get("PA-W09-date-no-model")!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
    "controlled-composition",
  ]);
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toMatch(/[0-9੦-੯]/);
  expect(activity?.answer).toBe("੨੫/੦੨/੨੦੨੫");
});
