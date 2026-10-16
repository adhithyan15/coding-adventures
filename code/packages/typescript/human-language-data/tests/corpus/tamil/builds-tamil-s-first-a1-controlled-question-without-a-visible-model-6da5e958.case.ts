import { expect, it } from "vitest";
import { loadTrackChapters, loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("builds Tamil's first A1 controlled question without a visible model", () => {
  const lesson = loadTrackLessons("tamil").find(
    (candidate) =>
      candidate.realization.lessonId === "TA-W281-enge-controlled-question",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.skills).toContain("writing");
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThan(300);
  expect(lesson?.frontmatter["introduces.knowledge"]).toEqual([]);

  const controlledBlocks = lesson?.blocks.filter(
    (block) => block.writingStage === "controlled-composition",
  );
  expect(controlledBlocks).toHaveLength(1);
  const controlled = controlledBlocks?.[0];
  expect(controlled?.markdown).toContain("no word bank");
  expect(controlled?.markdown).toContain("romanization");
  expect(controlled?.markdown).toContain("copyable sentence");
  expect(controlled?.markdown).toContain("composition is untimed");
  expect(controlled?.markdown).not.toMatch(/[\u0B80-\u0BFF]/u);

  expect(lesson?.frontmatter["requires.knowledge"]).toEqual([
    "TA-LEX-TENEER-01",
    "TA-LEX-C40-DEIXIS-06",
  ]);
  expect(lesson?.frontmatter["practises.knowledge"]).toEqual([
    "TA-LEX-TENEER-01",
    "TA-LEX-C40-DEIXIS-06",
  ]);

  const chapter = loadTrackChapters()
    .find((track) => track.language === "tamil")
    ?.chapters.find((candidate) => candidate.chapter === 281);
  expect(chapter?.payoff.lesson).toBe("TA-W281-enge-controlled-question");

  const tamil = languageWritingStages("tamil");
  expect(tamil.defects).toEqual([]);
  expect(
    tamil.levels.find((level) => level.level === "A1")?.missingStages,
  ).toEqual(["timed-assessment-production"]);
});
