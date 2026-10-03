import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("adds Gujarati A2 connected composition with no copyable model", () => {
  const lessons = loadTrackLessons("gujarati");
  const lesson = lessons.find(
    (candidate) => candidate.realization.lessonId === "GU-W11-a2-connected-composition",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.spine_node).toBe("SPINE-SAY-WHAT-I-DO");
  expect(Number(lesson?.frontmatter.sequence)).toBe(7580);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(280);

  const connected = lesson?.blocks.find((block) => block.writingStage === "connected-composition");
  expect(connected).toBeDefined();
  const learnerCopy = connected?.markdown ?? "";
  expect(learnerCopy).toContain("Reader:** your friend Kiran");
  expect(learnerCopy).toContain("Purpose:** help Kiran understand your familiar");
  expect(learnerCopy).toContain("70–90 words in Gujarati using Gujarati script");
  expect(learnerCopy).toContain("one preference, one reason");
  expect(learnerCopy).toContain("one direct question for");
  expect(learnerCopy).toContain("content, not sentences");
  expect(learnerCopy).toContain("This first connected attempt is untimed");
  expect(learnerCopy).toContain("do not borrow a Hindi form");
  expect(learnerCopy).not.toMatch(/[\u0A80-\u0AFF]/);

  const completeLesson = lesson?.blocks.map((block) => block.markdown).join("\n") ?? "";
  expect(completeLesson).toContain("no model, sentence bank, or copyable answer");
  expect(completeLesson).toContain("Put away romanization, dictionaries");
  expect(completeLesson).toContain("Task fulfilment and relevant content");
  expect(completeLesson).toContain("Comprehensibility and organisation");
  expect(completeLesson).toContain("Gujarati vocabulary and grammar");
  expect(completeLesson).toContain("Orthographic control");
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const gujarati = languageWritingStages("gujarati");
  expect(gujarati.defects).toEqual([]);
  expect(gujarati.levels.find((level) => level.level === "A2")?.missingStages).toEqual([]);
});
