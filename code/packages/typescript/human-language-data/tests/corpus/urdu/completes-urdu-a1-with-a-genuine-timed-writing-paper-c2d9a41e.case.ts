import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("completes Urdu A1 with a genuine timed writing paper", () => {
  const lessons = loadTrackLessons("urdu");
  const lesson = lessons.find(
    (candidate) => candidate.realization.lessonId === "UR-W10-a1-timed-production",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(Number(lesson?.frontmatter.sequence)).toBe(7520);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(280);
  const a2Successor = lessons.find(
    (candidate) => candidate.realization.lessonId === "UR-W11-a2-connected-composition",
  );
  expect(Number(lesson?.frontmatter.sequence)).toBeLessThan(
    Number(a2Successor?.frontmatter.sequence ?? Number.POSITIVE_INFINITY),
  );

  const timed = lesson?.blocks.find(
    (block) => block.writingStage === "timed-assessment-production",
  );
  expect(timed).toBeDefined();
  const learnerCopy = timed?.markdown ?? "";
  expect(learnerCopy).toContain("20-minute timer");
  expect(learnerCopy).toContain("7 minutes, 40 points");
  expect(learnerCopy).toContain("all six fields in Urdu script");
  expect(learnerCopy).toContain("13 minutes, 60 points");
  expect(learnerCopy).toContain("Reader:** Amina");
  expect(learnerCopy).toContain("Purpose:** help Amina recognise you");
  expect(learnerCopy).toContain("30–40 words in Urdu script");
  expect(learnerCopy).toContain("When the 20-minute timer rings, **stop**");
  expect(learnerCopy).toContain("not sentences");
  expect(learnerCopy).not.toMatch(/[\u0600-\u06ff][^\n]*(?:is|means)/i);

  const completeLesson = lesson?.blocks.map((block) => block.markdown).join("\n") ?? "";
  expect(completeLesson).toContain("Task fulfilment and relevant content");
  expect(completeLesson).toContain("Comprehensibility and organisation");
  expect(completeLesson).toContain("Urdu vocabulary, grammar, and register");
  expect(completeLesson).toContain("Urdu orthographic control");
  expect(completeLesson).toContain(
    "Put away Roman Urdu, Devanagari, dictionaries, translators, and spell-checkers",
  );
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const urdu = languageWritingStages("urdu");
  expect(urdu.defects).toEqual([]);
  expect(urdu.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
