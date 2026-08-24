import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import { defaultCurriculumRoot, loadChapterPolicy, loadTrackLessons } from "../../src/loader.js";
import { measureRamp, readingOrder } from "../../src/ramp.js";
import { measureScriptClosure } from "../../src/script-closure.js";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Tamil continuity", () => expectLanguageContinuity("tamil"));
it("pins Tamil modality", () => expectLanguageModality("tamil"));
it("keeps Tamil's opening free of future farewells and pronouns", () => {
  const references = measureContinuity(
    loadTrackLessons("tamil", defaultCurriculumRoot()),
  ).forwardReferences;
  expect(references.length).toBeLessThanOrEqual(13);
  expect(references.filter((reference) => /-C0[12]-/.test(reference.lessonId))).toEqual([]);
});

it("keeps Tamil Chapter 7 meaning-first and below the three-glyph step budget", () => {
  const root = defaultCurriculumRoot();
  const lessons = loadTrackLessons("tamil", root).sort(readingOrder);
  const chapter = lessons.filter((lesson) =>
    /^TA-(?:C07-numbers|W07-(?:digits|number-words|numbers))/.test(lesson.realization.lessonId),
  );
  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "TA-C07-numbers-1-5",
    "TA-W07-digits-1-3",
    "TA-W07-digits-4-5",
    "TA-W07-number-words-1-5",
    "TA-W07-numbers-1-5-guided-copy",
    "TA-W07-numbers-1-5-delayed-copy",
    "TA-W07-numbers-1-5-dictation",
    "TA-C07-numbers-1-5-family",
    "TA-C07-numbers-6-10",
    "TA-W07-digits-6-8",
    "TA-W07-digits-9-10",
    "TA-W07-number-words-6-10",
    "TA-W07-numbers-6-10-guided-copy",
    "TA-W07-numbers-6-10-delayed-copy",
    "TA-W07-numbers-6-10-dictation",
    "TA-C07-numbers-6-10-family",
    "TA-C07-numbers-practice",
  ]);

  const spoken = chapter.filter((lesson) =>
    ["TA-C07-numbers-1-5", "TA-C07-numbers-6-10"].includes(lesson.realization.lessonId),
  );
  expect(spoken.every((lesson) => !lesson.body.match(/\p{Script=Tamil}/u))).toBe(true);
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

it("teaches all ten Tamil numeral glyphs and leaves only the separate short-o sign debt", () => {
  const track = measureScriptClosure(loadTrackLessons("tamil")).tracks.find(
    (candidate) => candidate.language === "tamil",
  );
  expect(track?.neverTaughtGlyphs).toBe(1);
});
