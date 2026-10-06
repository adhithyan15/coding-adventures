import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

// Bengali reaches the app as a script tab of its own: the track's 30 letters,
// recognition-only except where native writers' pen traces cite an order.
export default [
  {
    suite: "independent (word-initial) vowels",
    suiteOrder: 10,
    caseOrder: 95,
    name: "keeps Bengali after every older script, with its letters and cited rows intact",
    verify: ({ SCRIPTS }) => {
      // Bengali was the last tab until Gurmukhi joined after it.
      expect(SCRIPTS.at(-2)!.script).toBe("bengali");
      const bengali = SCRIPTS.find((script) => script.script === "bengali")!;
      expect(bengali.letters).toHaveLength(30);
      expect(bengali.letters.slice(0, 4).map((letter) => letter.glyph)).toEqual(["আ", "ই", "এ", "ও"]);
      const cited = bengali.letters.filter((letter) => letter.strokeOrderSource !== undefined);
      expect(cited.map((letter) => letter.glyph)).toEqual(["এ", "ও", "খ", "ঞ", "থ", "ব", "র"]);
      for (const letter of cited) {
        expect(letter.penLifts, letter.glyph).toBeGreaterThanOrEqual(0);
        expect(letter.strokeOrderSource!.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
      }
      const e = bengali.letters.find((letter) => letter.glyph === "এ")!;
      expect(e.strokeOrder[0]).toMatch(/inside the curl.*clockwise/i);
    },
  },
] satisfies readonly GlyphEvidence[];
