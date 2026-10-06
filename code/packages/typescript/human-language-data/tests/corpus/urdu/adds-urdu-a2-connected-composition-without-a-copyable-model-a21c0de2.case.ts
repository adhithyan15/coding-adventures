import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("adds Urdu A2 connected composition without a copyable model", () => {
  const lessons = loadTrackLessons("urdu");
  const lesson = lessons.find(
    (candidate) => candidate.realization.lessonId === "UR-W11-a2-connected-composition",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.spine_node).toBe("SPINE-SAY-WHAT-I-DO");
  expect(Number(lesson?.frontmatter.sequence)).toBe(7530);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(280);

  const connected = lesson?.blocks.find((block) => block.writingStage === "connected-composition");
  expect(connected).toBeDefined();
  const learnerCopy = connected?.markdown ?? "";
  expect(learnerCopy).toContain("Reader:** Sana");
  expect(learnerCopy).toContain("Purpose:** help Sana understand your usual weekday");
  expect(learnerCopy).toContain("70–90 words in Urdu script");
  expect(learnerCopy).toMatch(/one contrast or\s+reason/);
  expect(learnerCopy).toContain("one sequence link");
  expect(learnerCopy).toContain("one direct question for Sana");
  expect(learnerCopy).toContain("content, not sentences");
  expect(learnerCopy).toContain("This first connected attempt is untimed");
  expect(learnerCopy).not.toMatch(/[\u0600-\u06ff]/);

  const completeLesson = lesson?.blocks.map((block) => block.markdown).join("\n") ?? "";
  expect(completeLesson).toContain("no model, sentence bank, or copyable answer");
  expect(completeLesson).toContain("Put away Roman Urdu, Devanagari");
  expect(completeLesson).toContain("Ordinary legible Urdu handwriting is enough");
  expect(completeLesson).toContain("Task fulfilment and relevant content");
  expect(completeLesson).toContain("Comprehensibility and organisation");
  expect(completeLesson).toContain("Urdu vocabulary, grammar, and register");
  expect(completeLesson).toContain("Urdu orthographic control");
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const urdu = languageWritingStages("urdu");
  expect(urdu.defects).toEqual([]);
  expect(urdu.levels.find((level) => level.level === "A2")?.missingStages).toEqual([]);
});
