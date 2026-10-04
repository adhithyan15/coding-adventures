import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("builds Bengali's first A1 controlled question without a visible model", () => {
  const lesson = loadTrackLessons("bengali").find(
    (candidate) =>
      candidate.realization.lessonId === "BN-W26-kothay-controlled-question",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.skills).toContain("writing");
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(150);

  const controlled = lesson?.blocks.find(
    (block) => block.writingStage === "controlled-composition",
  );
  expect(controlled?.markdown).toContain("no word bank, romanization");
  expect(controlled?.markdown).toContain("copyable sentence");
  expect(controlled?.markdown).toContain("composition is untimed");
  expect(controlled?.markdown).not.toContain("চা কোথায়");

  const activity = controlled?.activities?.find(
    (candidate) => candidate.id === "BN-W26-kothay-controlled-question-check",
  );
  expect(activity?.prompt).toContain("no word bank, romanization, or copyable answer");
  expect(activity?.answer).toBe("চা কোথায়?");
});

it("keeps Bengali A1 writing complete after the controlled question's successor lands", () => {
  const bengali = languageWritingStages("bengali");
  expect(bengali.defects).toEqual([]);
  expect(bengali.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
