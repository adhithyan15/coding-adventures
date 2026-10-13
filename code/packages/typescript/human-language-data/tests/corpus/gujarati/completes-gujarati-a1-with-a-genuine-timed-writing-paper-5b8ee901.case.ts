import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";
import { directedLessonSeconds } from "../timed-lesson-budget.js";

it("completes Gujarati A1 with a genuine timed writing paper", () => {
  const lessons = loadTrackLessons("gujarati");
  const lesson = lessons.find(
    (candidate) => candidate.realization.lessonId === "GU-W10-a1-timed-production",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(Number(lesson?.frontmatter.sequence)).toBe(7570);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(280);
  expect(
    lessons.toSorted(
      (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
    ).at(-1)?.realization.lessonId,
  ).toBe("GU-W11-a2-connected-composition");

  const timed = lesson?.blocks.find(
    (block) => block.writingStage === "timed-assessment-production",
  );
  expect(timed).toBeDefined();
  const learnerCopy = timed?.markdown ?? "";
  expect(learnerCopy).toContain("three-minute timer");
  expect(learnerCopy).toContain("all six fields in Gujarati script");
  expect(learnerCopy).toContain("When the three-minute timer rings, **stop**");
  expect(learnerCopy).not.toContain("Task 2");
  expect(learnerCopy).not.toMatch(/[અ-હ][^\n]*(?:is|means)/i);

  const completeLesson = lesson?.blocks.map((block) => `## ${block.title}\n${block.markdown}`).join("\n") ?? "";
  expect(directedLessonSeconds(completeLesson)).toBeLessThanOrEqual(
    Number(lesson?.frontmatter["duration.max_seconds"]),
  );
  expect(completeLesson).toContain("../mocks/a1/writing-paper-1.md");
  expect(completeLesson).toContain("separate assessment session");
  expect(completeLesson).not.toContain("Set one timer for **20 minutes**");
  expect(completeLesson).toContain("Put away romanization, dictionaries, translators, and spell-checkers");
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const gujarati = languageWritingStages("gujarati");
  expect(gujarati.defects).toEqual([]);
  expect(gujarati.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
