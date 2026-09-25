import { expect, it } from "vitest";
import { loadTrackLessons } from "../../../src/loader.js";
import { readingOrder } from "../../../src/ramp.js";
import { measureScriptClosure } from "../../../src/script-closure.js";

// HL-C443 (gentle writing): a letter is written FROM a word the reader already
// knows. ऋ used to come one lesson before ऋतु, cold; it now comes right after
// it, so the reader meets the word first and then takes its first letter out
// to write. Script closure allows this by design: ऋतु arrives with its
// romanization, so its headword is exposure (script-closure.ts), and the
// closure measure must still report nothing for that lesson.
it("writes independent ऋ from ऋतु, right after the word, without opening a closure gap", () => {
  const ordered = loadTrackLessons("hindi").sort(readingOrder);
  const scriptLessonIndex = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "HI-S125-letter-vocalic-r",
  );
  const seasonLessonIndex = ordered.findIndex(
    (lesson) => lesson.realization.lessonId === "HI-C14-ritu",
  );

  expect(scriptLessonIndex).toBeGreaterThanOrEqual(0);
  expect(seasonLessonIndex).toBeGreaterThanOrEqual(0);
  expect(scriptLessonIndex).toBeGreaterThan(seasonLessonIndex);
  expect(ordered[scriptLessonIndex]?.body).toContain("ऋ");
  expect(ordered[seasonLessonIndex]?.realization.headword).toContain("ऋतु");
  expect(
    measureScriptClosure(ordered).violations.filter(
      (violation) => violation.lessonId === "HI-C14-ritu",
    ),
  ).toEqual([]);
});
