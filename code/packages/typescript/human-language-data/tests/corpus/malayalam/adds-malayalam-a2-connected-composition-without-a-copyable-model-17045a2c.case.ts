import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("adds Malayalam A2 connected composition without a copyable model", () => {
  const lesson = loadTrackLessons("malayalam").find(
    (candidate) => candidate.realization.lessonId === "ML-W338-a2-connected-composition",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.spine_node).toBe("SPINE-SAY-WHAT-I-DO");
  expect(Number(lesson?.frontmatter.sequence)).toBe(16070);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(280);

  const connected = lesson?.blocks.find((block) => block.writingStage === "connected-composition");
  expect(connected).toBeDefined();
  const learnerCopy = connected?.markdown ?? "";
  expect(learnerCopy).toContain("Reader:** Anu");
  expect(learnerCopy).toContain("Purpose:** help Anu understand your usual weekday");
  expect(learnerCopy).toContain("70–90 words in Malayalam script");
  expect(learnerCopy).toMatch(/one contrast or\s+reason/);
  expect(learnerCopy).toContain("one sequence link");
  expect(learnerCopy).toContain("one direct question for Anu");
  expect(learnerCopy).toContain("content, not sentences");
  expect(learnerCopy).toContain("This first connected attempt is untimed");
  expect(learnerCopy).not.toMatch(/[\u0D00-\u0D7F]/u);

  const completeLesson = lesson?.blocks.map((block) => block.markdown).join("\n") ?? "";
  expect(completeLesson).toContain("no model, sentence bank, or copyable answer");
  expect(completeLesson).toContain("Put away romanization, dictionaries");
  expect(completeLesson).toContain("either reformed or traditional Malayalam orthography");
  expect(completeLesson).toContain("Task fulfilment and relevant content");
  expect(completeLesson).toContain("Organisation and cohesion");
  expect(completeLesson).toContain("Malayalam vocabulary and grammar");
  expect(completeLesson).toContain("Consistent control of one accepted Malayalam orthography");
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const malayalam = languageWritingStages("malayalam");
  expect(malayalam.defects).toEqual([]);
  expect(malayalam.levels.find((level) => level.level === "A2")?.missingStages).toEqual([]);
});
