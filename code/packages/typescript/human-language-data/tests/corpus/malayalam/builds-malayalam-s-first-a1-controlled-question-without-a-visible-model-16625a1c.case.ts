import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";

it("builds Malayalam's first A1 controlled question without a visible model", () => {
  const lesson = loadTrackLessons("malayalam").find(
    (candidate) => candidate.realization.lessonId === "ML-W108-evide-controlled-question",
  );
  expect(lesson).toBeDefined();
  expect(lesson?.frontmatter.type).toBe("writing");
  expect(lesson?.frontmatter.skills).toContain("writing");
  expect(Number(lesson?.frontmatter["duration.max_seconds"])).toBeLessThanOrEqual(150);

  const controlled = lesson?.blocks.find(
    (block) => block.writingStage === "controlled-composition",
  );
  expect(controlled?.markdown).toContain("no word bank");
  expect(controlled?.markdown).toContain("romanization");
  expect(controlled?.markdown).toContain("copyable sentence");
  expect(controlled?.markdown).toContain("composition is untimed");
  expect(controlled?.markdown).not.toContain("ചായ എവിടെ");

  const activity = controlled?.activities?.find(
    (candidate) => candidate.id === "ML-W108-evide-controlled-question-check",
  );
  expect(activity?.prompt).toContain("no word bank, romanization, or copyable answer");
  expect(activity?.answer).toBe("ചായ എവിടെ?");
});
