import { expect, it } from "vitest";
import { loadTrackChapters, loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("adds Kannada A2 connected composition with no copyable model", () => {
  const lesson = loadTrackLessons("kannada").find(
    (candidate) =>
      candidate.realization.lessonId === "KA-W308-a2-connected-composition",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.spine_node).toBe("SPINE-SAY-WHAT-I-DO");
  expect(Number(lesson?.frontmatter.chapter)).toBe(308);
  expect(Number(lesson?.frontmatter.sequence)).toBe(14890);
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
  expect(connected).toContain("Reader:** your friend Anu");
  expect(connected).toContain("Purpose:** help Anu understand your familiar day");
  expect(connected).toContain("70–90 words in Kannada script");
  expect(connected).toContain("one sequence link");
  expect(connected).toContain("one contrast or reason");
  expect(connected).toContain("one direct question for Anu");
  expect(connected).toContain("content, not sentences");
  expect(connected).toContain("first connected attempt is untimed");
  expect(connected).not.toMatch(/[\u0C80-\u0CFF]/u);

  const completeLesson = (
    lesson?.blocks.map((block) => block.markdown).join("\n") ?? ""
  ).replace(/\s+/gu, " ");
  for (const boundary of [
    "no Kannada model, sentence bank, word bank, romanization, or copyable answer",
    "dictionaries, grammar references, translators, generative assistants, and spell-checkers",
    "Task fulfilment and relevant content",
    "Organisation and cohesion",
    "Kannada vocabulary and grammar",
    "Kannada-script control",
    "not the complete 30-minute A2 writing paper",
    "does not prove A2 attainment or readiness",
    "human validation",
  ]) {
    expect(completeLesson).toContain(boundary);
  }
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const chapter = loadTrackChapters()
    .find((track) => track.language === "kannada")
    ?.chapters.find((candidate) => candidate.chapter === 308);
  expect(chapter).toMatchObject({
    chapter: 308,
    title: "A connected A2 Kannada message",
    payoff: {
      lesson: "KA-W308-a2-connected-composition",
      kind: "production",
    },
  });

  const kannada = languageWritingStages("kannada");
  expect(kannada.defects).toEqual([]);
  expect(
    kannada.levels.find((level) => level.level === "A2")?.missingStages,
  ).toEqual([]);
});
