import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";

it("keeps the chapter 93 plural recall inside the learner's available walk", () => {
  const lesson = loadTrackLessons("malayalam").find(
    (candidate) => candidate.realization.lessonId === "ML-C93-angal",
  );
  const recall = lesson?.blocks.find((block) => block.type === "recall");

  expect(recall?.markdown).toContain("respectful *you*");
  expect(recall?.markdown).not.toMatch(/\bwe\b/i);
  expect(recall?.knowledge?.assesses).not.toContain("ML-LEX-NJAANGAL-01");
});
