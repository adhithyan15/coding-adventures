import { expect, it } from "vitest";
import { loadTrackChapters, loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("adds Sanskrit A2 connected composition with no copyable model", () => {
  const lessons = loadTrackLessons("sanskrit");
  const lesson = lessons.find(
    (candidate) => candidate.realization.lessonId === "SA-W258-a2-connected-composition",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.spine_node).toBe("SPINE-SAY-WHAT-I-DO");
  expect(Number(lesson?.frontmatter.chapter)).toBe(258);
  expect(Number(lesson?.frontmatter.sequence)).toBe(15470);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(280);
  expect(lesson?.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
  expect(lesson?.frontmatter.introduces_senses).toEqual([]);
  expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);

  const connected = lesson?.blocks.filter(
    (block) => block.writingStage === "connected-composition",
  ) ?? [];
  expect(connected).toHaveLength(1);
  const learnerCopy = (connected[0]?.markdown ?? "").replace(/\s+/gu, " ");
  expect(learnerCopy).toContain("Reader:** your friend Kiran");
  expect(learnerCopy).toContain("Purpose:** help Kiran understand your familiar day");
  expect(learnerCopy).toContain("70–90 words in Sanskrit using Devanagari");
  expect(learnerCopy).toContain("one preference, one reason");
  expect(learnerCopy).toContain("one direct question for Kiran");
  expect(learnerCopy).toContain("content, not sentences");
  expect(learnerCopy).toContain("This first connected attempt is untimed");
  expect(learnerCopy).not.toMatch(/[\u0900-\u097F]/u);

  const completeLessonRaw = lesson?.blocks.map((block) => block.markdown).join("\n") ?? "";
  const completeLesson = completeLessonRaw.replace(/\s+/gu, " ");
  expect(completeLesson).toContain("no model, Sanskrit sentence bank, or copyable answer");
  expect(completeLesson).toContain("Put away romanization, dictionaries");
  expect(completeLesson).toContain("generative assistants");
  expect(completeLesson).toContain("Task fulfilment and relevant content");
  expect(completeLesson).toContain("Comprehensibility and organisation");
  expect(completeLesson).toContain("Sanskrit vocabulary and grammar");
  expect(completeLesson).toContain("Devanagari orthographic control");
  expect(completeLesson).toContain("external-sandhi");
  expect(completeLesson).toContain("word-final visarga or anusvara");
  expect(completeLesson).toContain("continuous śirorekhā");
  expect(completeLesson).toContain("does not prove A2 attainment or readiness");
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const ledger = loadTrackChapters().find((track) => track.language === "sanskrit");
  expect(ledger?.chapters.at(-1)).toMatchObject({
    chapter: 258,
    title: "A connected A2 message",
    payoff: {
      lesson: "SA-W258-a2-connected-composition",
      kind: "production",
    },
  });

  const sanskrit = languageWritingStages("sanskrit");
  expect(sanskrit.defects).toEqual([]);
  expect(sanskrit.levels.find((level) => level.level === "A2")?.missingStages).toEqual([]);
});
