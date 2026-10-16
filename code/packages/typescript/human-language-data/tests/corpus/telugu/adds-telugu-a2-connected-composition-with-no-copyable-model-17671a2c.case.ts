import { expect, it } from "vitest";
import { loadTrackChapters, loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("adds Telugu A2 connected composition with no copyable model", () => {
  const lesson = loadTrackLessons("telugu").find(
    (candidate) =>
      candidate.realization.lessonId === "TE-W315-a2-connected-composition",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.spine_node).toBe("SPINE-SAY-WHAT-I-DO");
  expect(Number(lesson?.frontmatter.chapter)).toBe(315);
  expect(Number(lesson?.frontmatter.sequence)).toBe(15140);
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
  expect(connected).toContain("Reader:** your friend Sita");
  expect(connected).toContain("Purpose:** help Sita understand your familiar day");
  expect(connected).toContain("70–90 words in Telugu script");
  expect(connected).toContain("one sequence link");
  expect(connected).toContain("one contrast or reason");
  expect(connected).toContain("one direct question for Sita");
  expect(connected).toContain("content, not sentences");
  expect(connected).toContain("first connected attempt is untimed");
  expect(connected).not.toMatch(/[\u0C00-\u0C7F]/u);

  const completeLesson = (
    lesson?.blocks.map((block) => block.markdown).join("\n") ?? ""
  ).replace(/\s+/gu, " ");
  for (const boundary of [
    "no Telugu model, sentence bank, word bank, romanization, or copyable answer",
    "dictionaries, grammar references, translators, generative assistants, and spell-checkers",
    "Task fulfilment and relevant content",
    "Organisation and cohesion",
    "Telugu vocabulary and grammar",
    "Telugu-script control",
    "not the complete 30-minute A2 writing paper",
    "does not prove A2 attainment or readiness",
    "human validation",
  ]) {
    expect(completeLesson).toContain(boundary);
  }
  expect(completeLesson).not.toContain("<!-- hl-activity:");

  const chapter = loadTrackChapters()
    .find((track) => track.language === "telugu")
    ?.chapters.find((candidate) => candidate.chapter === 315);
  expect(chapter).toMatchObject({
    chapter: 315,
    title: "A connected A2 Telugu message",
    payoff: {
      lesson: "TE-W315-a2-connected-composition",
      kind: "production",
    },
  });

  const telugu = languageWritingStages("telugu");
  expect(telugu.defects).toEqual([]);
  expect(
    telugu.levels.find((level) => level.level === "A2")?.missingStages,
  ).toEqual([]);
});
