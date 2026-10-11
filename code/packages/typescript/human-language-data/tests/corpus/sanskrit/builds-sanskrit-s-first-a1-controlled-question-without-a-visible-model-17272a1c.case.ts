import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("builds Sanskrit's first A1 controlled question without a visible model", () => {
  const lesson = loadTrackLessons("sanskrit").find(
    (candidate) =>
      candidate.realization.lessonId === "SA-W65-kutra-controlled-question",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.skills).toContain("writing");
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(150);

  const controlled = lesson?.blocks.find(
    (block) => block.writingStage === "controlled-composition",
  );
  expect(controlled?.markdown).toContain("no word bank");
  expect(controlled?.markdown).toContain("copyable sentence");
  expect(controlled?.markdown).toMatch(/composition is\s+untimed/u);
  expect(controlled?.markdown).not.toMatch(/[\u0900-\u097F]/u);

  const activity = controlled?.activities?.find(
    (candidate) => candidate.id === "SA-W65-kutra-controlled-question-check",
  );
  expect(activity?.prompt).toContain("no word bank, romanization, or copyable answer");
  expect(activity?.answer).toBe("पुस्तकम् कुत्र अस्ति?");
});

it("keeps Sanskrit A1 writing complete after the controlled question's successor lands", () => {
  const sanskrit = languageWritingStages("sanskrit");
  expect(sanskrit.defects).toEqual([]);
  expect(sanskrit.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
