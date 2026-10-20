import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";
import { directedLessonSeconds } from "../timed-lesson-budget.js";

it("separates the Kannada A1 timed lesson from its genuine writing paper", () => {
  const lesson = loadTrackLessons("kannada").find(
    (candidate) => candidate.realization.lessonId === "KA-W77-a1-timed-production",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.skills).toEqual(["writing"]);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(280);

  const timed = lesson?.blocks.find(
    (block) => block.writingStage === "timed-assessment-production",
  );
  const learnerCopy = timed?.markdown ?? "";
  expect(learnerCopy).toContain("three-minute timer");
  expect(learnerCopy).toContain("all six fields in Kannada script");
  expect(learnerCopy).toContain("When the three-minute timer rings, **stop**");
  expect(learnerCopy).not.toContain("Task 2");
  expect(learnerCopy).not.toMatch(/[\u0C80-\u0CFF]/u);

  const completeLesson = lesson?.blocks.map((block) => `## ${block.title}\n${block.markdown}`).join("\n") ?? "";
  expect(directedLessonSeconds(completeLesson)).toBeLessThanOrEqual(
    Number(lesson?.frontmatter["duration.max_seconds"]),
  );
  expect(completeLesson).toContain("../mocks/a1/writing-paper-1.md");
  expect(completeLesson).toContain("separate assessment session");
  expect(completeLesson).not.toContain("Set one timer for **20 minutes**");
  expect(completeLesson).toContain("not show you any copyable Kannada answer");

  const paper = readFileSync(
    resolve(process.cwd(), "../../../learning/human-languages/kannada/mocks/a1/writing-paper-1.md"),
    "utf8",
  );
  expect(paper).toContain("20-minute");
  expect(paper).toContain("7 minutes");
  expect(paper).toContain("13 minutes");
  expect(paper).toContain("ಅನು");
});

it("clears every cumulative Kannada A1 writing stage", () => {
  const kannada = languageWritingStages("kannada");
  expect(kannada.defects).toEqual([]);
  expect(kannada.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
