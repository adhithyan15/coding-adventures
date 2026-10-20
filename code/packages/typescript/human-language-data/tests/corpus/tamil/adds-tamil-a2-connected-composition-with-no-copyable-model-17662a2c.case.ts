import { expect, it } from "vitest";
import { loadTrackChapters, loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("adds Tamil A2 connected composition with no copyable model", () => {
  const lesson = loadTrackLessons("tamil").find(
    (candidate) =>
      candidate.realization.lessonId === "TA-W283-a2-connected-composition",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.spine_node).toBe("SPINE-SAY-WHAT-I-DO");
  expect(Number(lesson?.frontmatter.chapter)).toBe(283);
  expect(Number(lesson?.frontmatter.sequence)).toBe(16090);
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThan(300);
  expect(lesson?.frontmatter["introduces.knowledge"]).toEqual([]);
  expect(lesson?.frontmatter.introduces_idioms).toEqual([]);
  expect(lesson?.frontmatter.introduces_senses).toEqual([]);
  expect(lesson?.frontmatter.introduces_culture_claims).toEqual([]);

  const connectedBlocks = lesson?.blocks.filter(
    (block) => block.writingStage === "connected-composition",
  );
  expect(connectedBlocks).toHaveLength(1);
  const connected = (connectedBlocks?.[0]?.markdown ?? "").replace(/\s+/gu, " ");
  expect(connected).toContain("Reader:** your friend Meena");
  expect(connected).toContain("Purpose:** help Meena understand your familiar day");
  expect(connected).toContain("70–90 words in Tamil script");
  expect(connected).toContain("one sequence link");
  expect(connected).toContain("one contrast or reason");
  expect(connected).toContain("one direct question for Meena");
  expect(connected).toContain("content, not sentences");
  expect(connected).toContain("first connected attempt is untimed");
  expect(connected).not.toMatch(/[\u0B80-\u0BFF]/u);

  const completeLesson = (
    lesson?.blocks.map((block) => block.markdown).join("\n") ?? ""
  ).replace(/\s+/gu, " ");
  for (const boundary of [
    "no Tamil model, sentence bank, word bank, romanization, or copyable answer",
    "dictionaries, grammar references, translators, generative assistants, and spell-checkers",
    "Task fulfilment and relevant content",
    "Organisation and cohesion",
    "Tamil vocabulary and grammar",
    "Tamil-script control",
    "not the complete 30-minute A2 writing paper",
    "does not prove A2 attainment or readiness",
    "human validation",
  ]) {
    expect(completeLesson).toContain(boundary);
  }
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const chapter = loadTrackChapters()
    .find((track) => track.language === "tamil")
    ?.chapters.find((candidate) => candidate.chapter === 283);
  expect(chapter).toMatchObject({
    chapter: 283,
    title: "A connected A2 Tamil message",
    payoff: {
      lesson: "TA-W283-a2-connected-composition",
      kind: "production",
    },
  });

  const tamil = languageWritingStages("tamil");
  expect(tamil.defects).toEqual([]);
  expect(
    tamil.levels.find((level) => level.level === "A2")?.missingStages,
  ).toEqual([]);
});
