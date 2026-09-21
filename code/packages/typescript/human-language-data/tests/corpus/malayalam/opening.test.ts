import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadChapterPolicy,
  loadTrackLessons,
} from "../../../src/loader.js";
import { measureRamp, readingOrder } from "../../../src/ramp.js";

it("keeps Malayalam's opening free of genuine future farewells and pronouns", () => {
  const references = measureContinuity(
    loadTrackLessons("malayalam", defaultCurriculumRoot()),
  ).forwardReferences;
  expect(references.length).toBeLessThanOrEqual(12);
  expect(references.filter((reference) => /-C0[12]-/.test(reference.lessonId))).toEqual([]);
});

it("keeps the santosham payoff inside the three-glyph lesson budget", () => {
  const root = defaultCurriculumRoot();
  const report = measureRamp(loadTrackLessons("malayalam", root), loadChapterPolicy(root)).script;
  expect(report.lessons.find((lesson) => lesson.lessonId === "ML-C02-santosham")).toBeUndefined();
});

it("keeps Malayalam's first greeting meaning-first and its script runway learner-visible", () => {
  const opening = loadTrackLessons("malayalam").sort(readingOrder).slice(0, 9);
  expect(opening.map((lesson) => lesson.realization.lessonId)).toEqual([
    "ML-C01-namaskaram",
    "ML-W01-na-ma-trace",
    "ML-W01-na-ma-guided-copy",
    "ML-W01-na-ma-delayed-copy",
    "ML-W01-na-ma-dictation",
    "ML-W01-sa-chandrakkala-ka",
    "ML-W01-aa-ra-anusvaram",
    "ML-W01-namaskaram-read",
    "ML-W01-namaskaram-dictation",
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);
  expect(opening[0]?.frontmatter.skills).toEqual(["listening", "speaking"]);
  expect(opening[0]?.body).not.toMatch(/\p{Script=Malayalam}/u);
});
