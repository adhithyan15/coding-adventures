import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("builds Gujarati's first A1 controlled question without a visible model", () => {
  const lesson = loadTrackLessons("gujarati").find(
    (candidate) => candidate.realization.lessonId === "GU-C37-kyaan-write",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.romanization).toBe("kyā̃");
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(260);

  const stages = lesson?.blocks
    .map((block) => block.writingStage)
    .filter((stage): stage is string => stage !== undefined);
  expect(stages).toEqual(["guided-copy", "controlled-composition"]);

  const controlled = lesson?.blocks.find(
    (block) => block.writingStage === "controlled-composition",
  );
  const learnerCopy = controlled?.markdown.split("<!-- hl-activity")[0] ?? "";
  expect(learnerCopy).toContain("There is no word bank, romanization, or copyable sentence");
  expect(learnerCopy).toContain("untimed: finish the line");
  expect(learnerCopy).not.toContain("ઘર ક્યાં છે");

  const activity = lesson?.blocks
    .flatMap((block) => block.activities ?? [])
    .find((candidate) => candidate.id === "GU-C37-kyaan-write-controlled-question");
  expect(activity?.prompt).toContain("no word bank, romanization, or copyable answer");
  expect(activity?.answer).toBe("ઘર ક્યાં છે?");
});

it("leaves only timed assessment production missing from Gujarati A1", () => {
  const gujarati = languageWritingStages("gujarati");
  expect(gujarati.defects).toEqual([]);
  expect(gujarati.levels.find((level) => level.level === "A1")?.missingStages).toEqual([
    "timed-assessment-production",
  ]);
});
