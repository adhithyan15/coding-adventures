import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

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
  ).toBe("GU-W10-a1-timed-production");

  const timed = lesson?.blocks.find(
    (block) => block.writingStage === "timed-assessment-production",
  );
  expect(timed).toBeDefined();
  const learnerCopy = timed?.markdown ?? "";
  expect(learnerCopy).toContain("20-minute timer");
  expect(learnerCopy).toContain("7 minutes, 40 points");
  expect(learnerCopy).toContain("all six fields in Gujarati script");
  expect(learnerCopy).toContain("13 minutes, 60 points");
  expect(learnerCopy).toContain("Reader:** Mira");
  expect(learnerCopy).toContain("Purpose:** help Mira recognise you");
  expect(learnerCopy).toContain("30–40 words in Gujarati script");
  expect(learnerCopy).toContain("When the 20-minute timer rings, **stop**");
  expect(learnerCopy).toContain("not sentences");
  expect(learnerCopy).not.toMatch(/[અ-હ][^\n]*(?:is|means)/i);

  const completeLesson = lesson?.blocks.map((block) => block.markdown).join("\n") ?? "";
  expect(completeLesson).toContain("Task fulfilment and relevant content");
  expect(completeLesson).toContain("Comprehensibility and organisation");
  expect(completeLesson).toContain("Gujarati vocabulary and grammar");
  expect(completeLesson).toContain("Orthographic control");
  expect(completeLesson).toContain("Put away romanization, dictionaries, translators, and spell-checkers");
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const gujarati = languageWritingStages("gujarati");
  expect(gujarati.defects).toEqual([]);
  expect(gujarati.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
