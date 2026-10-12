import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";
import { directedLessonSeconds } from "../timed-lesson-budget.js";

it("separates the Urdu A1 timed lesson from its genuine writing paper", () => {
  const lessons = loadTrackLessons("urdu");
  const lesson = lessons.find(
    (candidate) => candidate.realization.lessonId === "UR-W10-a1-timed-production",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.skills).toEqual(["writing"]);
  expect(Number(lesson?.frontmatter.sequence)).toBe(7520);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(280);
  const a2Successor = lessons.find(
    (candidate) => candidate.realization.lessonId === "UR-W11-a2-connected-composition",
  );
  expect(Number(lesson?.frontmatter.sequence)).toBeLessThan(
    Number(a2Successor?.frontmatter.sequence ?? Number.POSITIVE_INFINITY),
  );

  const timed = lesson?.blocks.find(
    (block) => block.writingStage === "timed-assessment-production",
  );
  expect(timed).toBeDefined();
  const learnerCopy = timed?.markdown ?? "";
  expect(learnerCopy).toContain("three-minute timer");
  expect(learnerCopy).toContain("all six fields in Urdu script");
  expect(learnerCopy).toContain("When the three-minute timer rings, **stop**");
  expect(learnerCopy).not.toContain("Task 2");
  expect(learnerCopy).not.toMatch(/[\u0600-\u06ff][^\n]*(?:is|means)/i);

  const completeLesson = lesson?.blocks.map((block) => `## ${block.title}\n${block.markdown}`).join("\n") ?? "";
  expect(directedLessonSeconds(completeLesson)).toBeLessThanOrEqual(
    Number(lesson?.frontmatter["duration.max_seconds"]),
  );
  expect(completeLesson).toContain("../mocks/a1/writing-paper-1.md");
  expect(completeLesson).toContain("separate assessment session");
  expect(completeLesson).not.toContain("Set one timer for **20 minutes**");
  expect(completeLesson).toContain(
    "Put away Roman Urdu, Devanagari, dictionaries, translators, and spell-checkers",
  );
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const paper = readFileSync(
    resolve(process.cwd(), "../../../learning/human-languages/urdu/mocks/a1/writing-paper-1.md"),
    "utf8",
  );
  expect(paper).toContain("20-minute");
  expect(paper).toContain("7 minutes");
  expect(paper).toContain("13 minutes");
  expect(paper).toContain("سارہ");
});

it("clears every cumulative Urdu A1 writing stage", () => {
  const urdu = languageWritingStages("urdu");
  expect(urdu.defects).toEqual([]);
  expect(urdu.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
