import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";

it("keeps the weekday comparison in the right scripts with exact glosses", () => {
  const lesson = loadTrackLessons("malayalam").find(
    (candidate) => candidate.realization.lessonId === "ML-C10-azhcha",
  );

  expect(lesson?.body).toContain('**கிழமை** (*kizhamai*, "day of the week")');
  expect(lesson?.body).toContain('**ആഴ്ച** (*āḻca*, "week")');
  expect(lesson?.body).not.toContain("ஆழ்ச");
  expect(lesson?.body).not.toContain('(*āḻca*) — a different word for the same idea');
});
