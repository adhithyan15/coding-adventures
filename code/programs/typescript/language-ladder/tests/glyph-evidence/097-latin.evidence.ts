import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

// Latin reaches the app as a script tab of its own: the Latin-script tracks'
// 57 rows (letters, ordinal indicators and opening marks), recognition-only except where the
// Grundschrift-App (a school model) or UJIpenchars2's native Spanish writers
// cite an order. a is never cited: Noto prints a two-storey a, and every
// source draws the one-storey a.
export default [
  {
    suite: "independent (word-initial) vowels",
    suiteOrder: 10,
    caseOrder: 97,
    name: "keeps Latin the last script, with its letters and cited rows intact",
    verify: ({ SCRIPTS }) => {
      expect(SCRIPTS.at(-1)!.script).toBe("latin");
      const latin = SCRIPTS.find((script) => script.script === "latin")!;
      expect(latin.letters).toHaveLength(57);
      expect(latin.letters.slice(0, 5).map((letter) => letter.glyph)).toEqual(["a", "b", "c", "d", "e"]);
      const cited = latin.letters.filter((letter) => letter.strokeOrderSource !== undefined);
      expect(cited.map((letter) => letter.glyph).join("")).toBe("bceghilnorsuwßñG¿¡");
      expect(cited.map((letter) => letter.glyph)).not.toContain("a");
      for (const letter of cited) {
        expect(letter.penLifts, letter.glyph).toBeGreaterThanOrEqual(0);
        expect(letter.strokeOrderSource!.url).toMatch(
          /^https:\/\/(github\.com\/Medien-Treibhaus\/grundschrift-app-source\/blob\/f6dbd807adbb3fc2f94207fe578def439f6e9c49\/assets\/levels\/\w+\/\w+\/metadata\.json|archive\.ics\.uci\.edu\/dataset\/177\/uji\+pen\+characters\+version\+2)$/,
        );
      }
      const enye = latin.letters.find((letter) => letter.glyph === "ñ")!;
      expect(enye.strokeOrder.at(-1)).toBe("lift, then the tilde, left to right");
    },
  },
] satisfies readonly GlyphEvidence[];
