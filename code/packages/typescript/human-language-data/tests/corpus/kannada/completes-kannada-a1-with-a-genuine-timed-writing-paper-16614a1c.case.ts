import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("completes Kannada A1 with a genuine timed writing paper", () => {
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
  expect(timed?.markdown).toContain("Task 1");
  expect(timed?.markdown).toContain("Task 2");
  expect(timed?.markdown).toContain("Mira");
  expect(timed?.markdown).toContain("20–30 words");
  expect(timed?.markdown).toContain("20-minute timer");
  expect(timed?.markdown).toContain("stop");
  expect(timed?.markdown).not.toMatch(/[\u0C80-\u0CFF]/u);

  const completePaper = lesson?.blocks.map((block) => block.markdown).join("\n") ?? "";
  expect(completePaper).toContain("timer for **20 minutes**");
  expect(completePaper).toContain("not show you any copyable Kannada answer");
});

it("clears every cumulative Kannada A1 writing stage", () => {
  const kannada = languageWritingStages("kannada");
  expect(kannada.defects).toEqual([]);
  expect(kannada.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
