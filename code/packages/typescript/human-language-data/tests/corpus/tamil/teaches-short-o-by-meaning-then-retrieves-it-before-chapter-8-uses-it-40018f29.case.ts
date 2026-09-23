import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadChapterPolicy,
  loadEverything,
  loadExamInventory,
  loadTrackLessons,
} from "../../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../../src/exam-inventory.js";
import { measureRamp, readingOrder } from "../../../src/ramp.js";
import { measureScriptClosure } from "../../../src/script-closure.js";
import { expectLanguageContinuity, expectLanguageModality } from "../assert-language-corpus.js";

it("teaches short o by meaning, then retrieves it before Chapter 8 uses it", () => {
  const lessons = loadTrackLessons("tamil").sort(readingOrder);
  const chapter = lessons.filter((lesson) =>
    /^TA-(?:C08-(?:tayavuseytu|sollungal|please-register)|W08-(?:short-o|sollungal))/.test(
      lesson.realization.lessonId,
    ),
  );
  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "TA-C08-tayavuseytu",
    "TA-C08-sollungal",
    "TA-W08-short-o-observe",
    "TA-W08-sollungal-guided-copy",
    "TA-W08-sollungal-delayed-copy",
    "TA-W08-sollungal-dictation",
    "TA-C08-please-register",
  ]);

  const spoken = chapter.find((lesson) => lesson.realization.lessonId === "TA-C08-sollungal");
  expect(spoken?.body.match(/\p{Script=Tamil}/u)).toBeNull();
  expect(spoken?.frontmatter.skills?.join(",")).toBe("listening,speaking");
  expect(new Set(chapter.flatMap((lesson) =>
    [...lesson.body.matchAll(/hl-writing-stage:\s*([a-z-]+)/g)].map((match) => match[1]),
  ))).toEqual(new Set([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]));

  const closure = measureScriptClosure(lessons);
  const track = closure.tracks.find(
    (candidate) => candidate.language === "tamil",
  );
  expect(track?.neverTaughtGlyphs).toBe(0);
  // Zero, and pinned AS zero rather than re-pinned to a smaller count. The 21
  // that stood here were all one defect wearing twenty-one faces: chapters 1-6
  // printed Tamil words in running prose and recap tables beside their own
  // romanization, which the reader was never asked to decode and the script
  // strand does not reach until much later. Those chapters are the sound-first
  // opening, so the script came out of their bodies and stayed only in each
  // lesson's own romanized headword, where the exposure rule already covers it.
  // The last two -- ch9 and ch32 -- needed the letter உ, which was the one Tamil
  // letter with no lesson at all; TA-S125-letter-u now teaches it before either.
  // An exact zero, not a ceiling: a single new violation is a lesson asking the
  // reader to decode something nobody taught, and there is no longer a backlog
  // for it to hide inside.
  expect(track?.violations).toBe(0);
  // HL-C194: every Tamil headword now declares how to say it, so no headword is
  // load-bearing script. Pinned at zero rather than at a count, because this is
  // the one number in the closure report that an author can only make worse by
  // shipping a lesson whose headword nobody can pronounce.
  expect(track?.headwordsWithoutRomanization).toBe(0);
  expect(closure.violations.filter((violation) =>
    violation.language === "tamil" && (
      violation.glyphs.includes("ொ") ||
      ["TA-C08-please-register", "TA-C20-pathinondru-irupathu"].includes(violation.lessonId)
    )
  )).toEqual([]);
});
