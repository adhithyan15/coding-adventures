import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

// Gurmukhi reaches the app as a script tab of its own: the Punjabi track's 33
// letters, recognition-only except where GNPS's Gurmukhi Sikho tracing lesson
// cites an order.
export default [
  {
    suite: "independent (word-initial) vowels",
    suiteOrder: 10,
    caseOrder: 96,
    name: "keeps Gurmukhi the last script, with its letters and cited rows intact",
    verify: ({ SCRIPTS }) => {
      expect(SCRIPTS.at(-1)!.script).toBe("gurmukhi");
      const gurmukhi = SCRIPTS.find((script) => script.script === "gurmukhi")!;
      expect(gurmukhi.letters).toHaveLength(33);
      expect(gurmukhi.letters.slice(0, 5).map((letter) => letter.glyph)).toEqual(["ਅ", "ਆ", "ਇ", "ਈ", "ਉ"]);
      const cited = gurmukhi.letters.filter((letter) => letter.strokeOrderSource !== undefined);
      expect(cited).toHaveLength(27);
      expect(cited.map((letter) => letter.glyph)).not.toContain("ਝ");
      expect(cited.map((letter) => letter.glyph)).not.toContain("ਧ");
      for (const letter of cited) {
        expect(letter.penLifts, letter.glyph).toBeGreaterThanOrEqual(1);
        expect(letter.strokeOrderSource!.url).toMatch(
          /^https:\/\/github\.com\/codemanxdev\/gnps_learning_hub\/blob\/de1e56014cb0caf7e56de2320681781cae6a7e41\/lib\/data\/lessons\/lesson_tracing\.dart#L\d+$/,
        );
      }
      const ka = gurmukhi.letters.find((letter) => letter.glyph === "ਕ")!;
      expect(ka.strokeOrder[0]).toBe("draw the headline from left to right");
    },
  },
] satisfies readonly GlyphEvidence[];
