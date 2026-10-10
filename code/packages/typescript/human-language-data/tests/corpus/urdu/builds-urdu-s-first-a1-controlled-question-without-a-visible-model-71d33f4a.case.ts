import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { languageWritingStages } from "../assert-language-corpus.js";

it("builds Urdu's first A1 controlled question without a visible model", () => {
  const lesson = loadTrackLessons("urdu").find(
    (candidate) => candidate.realization.lessonId === "UR-C24-vahan",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("word");
  expect(lesson?.frontmatter.skills).toContain("writing");
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(290);

  const controlled = lesson?.blocks.find(
    (block) => block.writingStage === "controlled-composition",
  );
  expect(controlled?.markdown).toContain("no word bank, romanization");
  expect(controlled?.markdown).toContain("copyable sentence");
  expect(controlled?.markdown).toContain("composition is untimed");
  expect(controlled?.markdown).not.toContain("کمرہ کہاں ہے");

  const activity = controlled?.activities?.find(
    (candidate) => candidate.id === "UR-C24-vahan-controlled-question",
  );
  expect(activity?.prompt).toContain("no word bank, romanization, or copyable answer");
  // Was چائے کہاں ہے؟, "Where is the tea?". چ and the hamza on ye have no
  // script lesson by chapter 24, so script closure flagged the learner being
  // asked to write them. The room is spelled only with taught letters.
  expect(activity?.answer).toBe("کمرہ کہاں ہے؟");
});

it("keeps Urdu A1 complete after the controlled question's successor lands", () => {
  const urdu = languageWritingStages("urdu");
  expect(urdu.defects).toEqual([]);
  expect(urdu.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
