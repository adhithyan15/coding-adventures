import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("adds Marwadi A2 connected composition with no copyable model", () => {
  const lessons = loadTrackLessons("marwadi");
  const lesson = lessons.find(
    (candidate) => candidate.realization.lessonId === "MW-W44-a2-connected-composition",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.spine_node).toBe("SPINE-SAY-WHAT-I-DO");
  expect(Number(lesson?.frontmatter.sequence)).toBe(9420);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(280);

  const connected = lesson?.blocks.find((block) => block.writingStage === "connected-composition");
  expect(connected).toBeDefined();
  const learnerCopy = connected?.markdown ?? "";
  expect(learnerCopy).toContain("Reader:** your friend Kiran");
  expect(learnerCopy).toContain("Purpose:** help Kiran understand your usual");
  expect(learnerCopy).toContain("70–90 words in Marwari using Devanagari");
  expect(learnerCopy).toContain("one preference, one\nreason");
  expect(learnerCopy).toContain("one direct question for Kiran");
  expect(learnerCopy).toContain("content, not sentences");
  expect(learnerCopy).toContain("This first connected attempt is untimed");
  expect(learnerCopy).toContain("do not borrow a Hindi form");
  expect(learnerCopy).not.toMatch(/[\u0900-\u097f]/);

  const completeLesson = lesson?.blocks.map((block) => block.markdown).join("\n") ?? "";
  expect(completeLesson).toContain("no model, sentence bank, or copyable answer");
  expect(completeLesson).toContain("Put away romanization, dictionaries");
  expect(completeLesson).toContain("Task fulfilment and relevant content");
  expect(completeLesson).toContain("Comprehensibility and organisation");
  expect(completeLesson).toContain("Marwari vocabulary and grammar");
  expect(completeLesson).toContain("Orthographic control");
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const marwadi = languageWritingStages("marwadi");
  expect(marwadi.defects).toEqual([]);
  expect(marwadi.levels.find((level) => level.level === "A2")?.missingStages).toEqual([]);
});
