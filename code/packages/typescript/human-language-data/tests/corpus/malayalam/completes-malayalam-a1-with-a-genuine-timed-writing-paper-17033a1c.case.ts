import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("completes Malayalam A1 with a genuine timed writing paper", () => {
  const lesson = loadTrackLessons("malayalam").find(
    (candidate) => candidate.realization.lessonId === "ML-W108-a1-timed-production",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.skills).toEqual(["writing"]);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(280);

  const timed = lesson?.blocks.find(
    (block) => block.writingStage === "timed-assessment-production",
  );
  expect(timed?.markdown).toContain("Task 1");
  expect(timed?.markdown).toContain("7 minutes, 40 points");
  expect(timed?.markdown).toContain("all six fields in Malayalam script");
  expect(timed?.markdown).toContain("Task 2");
  expect(timed?.markdown).toContain("13 minutes, 60 points");
  expect(timed?.markdown).toContain("Reader:** Anu");
  expect(timed?.markdown).toContain("Purpose:** help Anu recognise you");
  expect(timed?.markdown).toContain("30–40 words in Malayalam script");
  expect(timed?.markdown).toContain("When the 20-minute timer rings, **stop**");
  expect(timed?.markdown).toContain("content, not sentences");
  expect(timed?.markdown).not.toMatch(/[\u0D00-\u0D7F]/u);

  const completePaper = lesson?.blocks.map((block) => block.markdown).join("\n") ?? "";
  expect(completePaper).toContain("timer for **20 minutes**");
  expect(completePaper).toContain("not show you any copyable Malayalam answer");
  expect(completePaper).toContain("Task fulfilment and relevant content");
  expect(completePaper).toContain("Comprehensibility and organisation");
  expect(completePaper).toContain("Malayalam vocabulary, grammar, and register");
  expect(completePaper).toContain("Malayalam orthographic control");
  expect(completePaper).not.toContain("<!-- hl-activity:");

  const malayalam = languageWritingStages("malayalam");
  expect(malayalam.defects).toEqual([]);
  expect(malayalam.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
