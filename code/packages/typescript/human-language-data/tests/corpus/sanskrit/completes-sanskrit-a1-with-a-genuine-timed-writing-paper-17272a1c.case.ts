import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("completes Sanskrit A1 with a genuine timed writing paper", () => {
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
  const markdown = lesson?.blocks.map((block) => block.markdown).join("\n") ?? "";
  expect(markdown).toContain("timer for **20 minutes**");
  expect(timed?.markdown).toContain("Task 1 — practical form: 7 minutes");
  expect(timed?.markdown).toContain("Task 2 — message: 13 minutes");
  expect(timed?.markdown).toContain("**Reader:** Mira");
  expect(timed?.markdown).toContain("**30–40 words in Devanagari**");
  expect(timed?.markdown).toContain("**stop**");
  expect(timed?.markdown).not.toMatch(/[\u0900-\u097F]/u);

  const sanskrit = languageWritingStages("sanskrit");
  expect(sanskrit.defects).toEqual([]);
  expect(sanskrit.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
