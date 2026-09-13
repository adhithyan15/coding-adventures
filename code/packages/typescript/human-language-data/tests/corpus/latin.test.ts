import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
import { loadTrackLessons } from "../../src/loader.js";
import { passageLength } from "../../src/reading-reach.js";
it("pins Latin continuity", () => expectLanguageContinuity("latin"));
it("pins Latin modality", () => expectLanguageModality("latin"));
it("pins Latin lesson-content budgets", () =>
  expectLanguageLessonBudgets("latin", {
    //
    // 160 -> 163: chapter 58, the reading rung -- six words, six lines and a
    // 42-word passage. No new word: every token was checked to occur in a
    // lesson with a lower sequence number, which for Latin means the exact
    // INFLECTED form, not the lemma.
    // 163 -> 168: chapter 59 adds a three-lesson case-and-place bridge before
    // the 73-word social exchange and 140-word adapted account use its forms.
    lessons: 168,
    idioms: 16,
    senses: 6,
    cultureClaims: 17,
    unitPrefix: "LA",
  }));

it("pins Latin's two A1 reading shapes and their exact-form closure", () => {
  const lessons = loadTrackLessons("latin");
  const targetIds = ["LA-C59-social-exchange", "LA-C59-dies-meus"];
  const lengths = targetIds.map((id) => {
    const lesson = lessons.find((candidate) => candidate.realization.lessonId === id)!;
    const passage = lesson.blocks.find((block) => block.type === "comprehension")!.markdown;
    const seenEarlier = lessons
      .filter((candidate) => Number(candidate.frontmatter.sequence) < Number(lesson.frontmatter.sequence))
      .map((candidate) => `${String(candidate.frontmatter.headword ?? "")}\n${candidate.body}`)
      .join("\n")
      .toLocaleLowerCase("la");
    const forms = passage.toLocaleLowerCase("la").match(/[\p{L}\p{M}]+/gu) ?? [];
    for (const form of forms) {
      expect(seenEarlier, `${id}: '${form}' must occur before the passage`).toContain(form);
    }
    return passageLength(passage);
  });

  expect(lengths).toEqual([73, 140]);
});

it("pins Latin's current writing ramp", () => {
  const latin = languageWritingStages("latin");
  expect(latin.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]);
});
