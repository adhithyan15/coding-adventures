import { expect, it } from "vitest";
import { loadTrackChapters, loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("adds Bengali A2 connected composition with no copyable model", () => {
  const lessons = loadTrackLessons("bengali");
  const lesson = lessons.find(
    (candidate) => candidate.realization.lessonId === "BN-W263-a2-connected-composition",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.spine_node).toBe("SPINE-SAY-WHAT-I-DO");
  expect(Number(lesson?.frontmatter.chapter)).toBe(263);
  expect(Number(lesson?.frontmatter.sequence)).toBe(16490);
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
  expect(learnerCopy).toContain("Reader:** your friend Rina");
  expect(learnerCopy).toContain("Purpose:** help Rina understand your familiar day");
  expect(learnerCopy).toContain("70–90 words in Bengali script");
  expect(learnerCopy).toContain("one preference, one reason");
  expect(learnerCopy).toContain("one direct question for Rina");
  expect(learnerCopy).toContain("content, not sentences");
  expect(learnerCopy).toContain("This first connected attempt is untimed");
  expect(learnerCopy).not.toMatch(/[\u0980-\u09FF]/u);

  const completeLessonRaw = lesson?.blocks.map((block) => block.markdown).join("\n") ?? "";
  const completeLesson = completeLessonRaw.replace(/\s+/gu, " ");
  expect(completeLesson).toContain("no model, Bengali sentence bank, or copyable answer");
  expect(completeLesson).toContain("Put away romanization, dictionaries");
  expect(completeLesson).toContain("generative assistants");
  expect(completeLesson).toContain("Task fulfilment and relevant content");
  expect(completeLesson).toContain("Comprehensibility and organisation");
  expect(completeLesson).toContain("Bengali vocabulary and grammar");
  expect(completeLesson).toContain("Bengali-script control");
  expect(completeLesson).toContain("letter shapes, vowel signs, conjuncts");
  expect(completeLesson).toContain("does not prove A2 attainment or readiness");
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const ledger = loadTrackChapters().find((track) => track.language === "bengali");
  expect(ledger?.chapters.find((chapter) => chapter.chapter === 263)).toMatchObject({
    chapter: 263,
    title: "A connected A2 message",
    payoff: {
      lesson: "BN-W263-a2-connected-composition",
      kind: "production",
    },
  });

  const bengali = languageWritingStages("bengali");
  expect(bengali.defects).toEqual([]);
  expect(bengali.levels.find((level) => level.level === "A2")?.missingStages).toEqual([]);
});
