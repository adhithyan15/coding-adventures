import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import { defaultCurriculumRoot, loadChapterPolicy, loadTrackLessons } from "../../src/loader.js";
import { measureRamp, readingOrder } from "../../src/ramp.js";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Malayalam continuity", () => expectLanguageContinuity("malayalam"));
it("pins Malayalam modality", () => expectLanguageModality("malayalam"));
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

it("keeps Malayalam Chapter 7 meaning-first and below the three-glyph step budget", () => {
  const root = defaultCurriculumRoot();
  const lessons = loadTrackLessons("malayalam", root).sort(readingOrder);
  const chapter = lessons.filter((lesson) => /^ML-[CW]07-/.test(lesson.realization.lessonId));
  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "ML-C07-numbers-1-5",
    "ML-W07-digits-1-3",
    "ML-W07-digits-4-5",
    "ML-W07-number-words-1-5",
    "ML-W07-numbers-1-5-guided-copy",
    "ML-W07-numbers-1-5-delayed-copy",
    "ML-W07-numbers-1-5-dictation",
    "ML-C07-numbers-6-10",
    "ML-W07-digits-6-8",
    "ML-W07-digits-9-10",
    "ML-W07-number-words-6-10",
    "ML-W07-numbers-6-10-guided-copy",
    "ML-W07-numbers-6-10-delayed-copy",
    "ML-W07-numbers-6-10-dictation",
    "ML-C07-numbers-practice",
  ]);

  const spoken = chapter.filter((lesson) => lesson.realization.lessonId.startsWith("ML-C07-numbers-") && lesson.realization.lessonId !== "ML-C07-numbers-practice");
  expect(spoken).toHaveLength(2);
  expect(spoken.every((lesson) => !lesson.body.match(/\p{Script=Malayalam}/u))).toBe(true);
  expect(spoken.every((lesson) => lesson.frontmatter.skills?.join(",") === "listening,speaking")).toBe(true);

  const script = measureRamp(lessons, loadChapterPolicy(root)).script;
  expect(script.lessons.filter((lesson) => lesson.chapter === 7)).toEqual([]);
  expect(new Set(chapter.flatMap((lesson) =>
    [...lesson.body.matchAll(/hl-writing-stage:\s*([a-z-]+)/g)].map((match) => match[1]),
  ))).toEqual(new Set([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]));
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

it("gives Malayalam a complete pre-A1 writing runway", () => {
  const malayalam = languageWritingStages("malayalam");
  expect(malayalam.defects).toEqual([]);
  expect(malayalam.levels[0]).toMatchObject({
    level: "pre-A1",
    complete: true,
    missingStages: [],
  });
  expect(new Set(malayalam.validEvidence.map((entry) => entry.stage))).toEqual(new Set([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]));
});
