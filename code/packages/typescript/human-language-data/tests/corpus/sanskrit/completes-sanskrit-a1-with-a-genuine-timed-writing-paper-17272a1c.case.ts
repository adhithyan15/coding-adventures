import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";
import { directedLessonSeconds } from "../timed-lesson-budget.js";

it("keeps Sanskrit's timed checkpoint inside the lesson and the full paper separate", () => {
  const lesson = loadTrackLessons("sanskrit").find(
    (candidate) =>
      candidate.realization.lessonId === "SA-W65-a1-timed-production",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.skills).toEqual(["writing"]);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(280);

  const timed = lesson?.blocks.find(
    (block) => block.writingStage === "timed-assessment-production",
  );
  const markdown = lesson?.blocks.map((block) => `## ${block.title}\n${block.markdown}`).join("\n") ?? "";
  expect(directedLessonSeconds(markdown)).toBeLessThanOrEqual(
    Number(lesson?.frontmatter["duration.max_seconds"]),
  );
  expect(timed?.markdown).toContain("three-minute timer");
  expect(markdown).toContain("../mocks/a1/mock-1.md");
  expect(markdown).toContain("separate assessment session");
  expect(timed?.markdown).toContain("**stop**");
  expect(timed?.markdown).not.toMatch(/[\u0900-\u097F]/u);

  const sanskrit = languageWritingStages("sanskrit");
  expect(sanskrit.defects).toEqual([]);
  expect(sanskrit.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});

it("rejects split 7+13-minute tasks and an unbounded post-task review", () => {
  const split = [
    "## Writing", "Task 1 — form: 7 minutes", "Task 2 — message: 13 minutes",
    "## Wrap-up", "Spend at most one minute reviewing the attempt.",
  ].join("\n");
  expect(directedLessonSeconds(split)).toBe(21 * 60);
  expect(directedLessonSeconds(split)).toBeGreaterThan(300);
  expect(directedLessonSeconds(split.replace("Spend at most one minute", "Spend time")))
    .toBe(Number.POSITIVE_INFINITY);
});
