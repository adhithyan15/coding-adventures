import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { expect, it } from "vitest";
import { loadTrackChapters, loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";
import { directedLessonSeconds } from "../timed-lesson-budget.js";

it("completes Tamil A1 with a short honest timed writing checkpoint", () => {
  const lessons = loadTrackLessons("tamil");
  const lesson = lessons.find(
    (candidate) =>
      candidate.realization.lessonId === "TA-W282-a1-timed-production",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.skills).toEqual(["writing"]);
  expect(lesson?.frontmatter.prerequisites).toContain(
    "TA-W281-enge-controlled-question",
  );
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThan(300);
  expect(lesson?.frontmatter["introduces.knowledge"]).toEqual([]);

  const timedBlocks = lesson?.blocks.filter(
    (block) => block.writingStage === "timed-assessment-production",
  );
  expect(timedBlocks).toHaveLength(1);
  const timed = timedBlocks?.[0]?.markdown ?? "";
  expect(timed).toContain("three-minute timer");
  expect(timed).toContain("all six fields in Tamil script");
  expect(timed).toContain("When the three-minute timer rings, **stop**");
  expect(timed).not.toMatch(/[\u0B80-\u0BFF]/u);

  const completeLesson =
    lesson?.blocks
      .map((block) => `## ${block.title}\n${block.markdown}`)
      .join("\n") ?? "";
  expect(directedLessonSeconds(completeLesson)).toBeLessThanOrEqual(
    Number(lesson?.frontmatter["duration.max_seconds"]),
  );
  for (const boundary of [
    "not show you any copyable Tamil answer",
    "word bank",
    "romanization",
    "dictionaries",
    "translators",
    "spell-checkers",
    "not the complete A1 writing paper",
    "does not establish A1",
    "attainment, readiness, completeness, or human validation",
  ]) {
    expect(completeLesson).toContain(boundary);
  }
  expect(completeLesson).not.toContain("../mocks/");

  const chapter = loadTrackChapters()
    .find((track) => track.language === "tamil")
    ?.chapters.find((candidate) => candidate.chapter === 282);
  expect(chapter?.payoff.lesson).toBe("TA-W282-a1-timed-production");

  const tamil = languageWritingStages("tamil");
  expect(tamil.defects).toEqual([]);
  expect(
    tamil.levels.find((level) => level.level === "A1")?.missingStages,
  ).toEqual([]);

  expect(
    existsSync(
      resolve(
        process.cwd(),
        "../../../learning/human-languages/tamil/assessment.json",
      ),
    ),
  ).toBe(false);
});
