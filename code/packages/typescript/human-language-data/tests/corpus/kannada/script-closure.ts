import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadEverything,
  loadExamInventory,
  loadTrackLessons,
} from "../../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../../src/exam-inventory.js";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";
it("pins how many Kannada characters the corpus prints and never teaches", () => {
  const lessons = loadTrackLessons("kannada", defaultCurriculumRoot());
  const isInventory = (headword: string) =>
    headword
      .replace(/◌/g, "")
      .trim()
      .split(/[\s/·]+/)
      .filter(Boolean)
      .every((token) => [...token].length <= 2);

  const taught = new Set<string>();
  for (const lesson of lessons) {
    const headword = lesson.frontmatter.headword ?? "";
    if (!headword || !isInventory(headword)) continue;
    for (const glyph of headword.replace(/◌/g, "")) taught.add(glyph);
  }

  const used = new Set<string>();
  for (const lesson of lessons) {
    for (const glyph of lesson.body) {
      const code = glyph.codePointAt(0)!;
      if (code >= 0x0c80 && code <= 0x0cff) used.add(glyph);
    }
  }

  const untaught = [...used].filter((glyph) => !taught.has(glyph));
  expect(untaught).toEqual([]);
});
