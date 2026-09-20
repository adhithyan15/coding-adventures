import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { readingOrder } from "../../../src/ramp.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Marathi's complete pre-A1 writing ramp", () => {
  const marathi = languageWritingStages("marathi");
  expect(marathi.defects).toEqual([]);
  expect(marathi.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
});
