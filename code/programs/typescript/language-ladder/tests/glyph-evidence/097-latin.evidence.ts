import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

// Latin reaches the app as a script tab of its own: the Latin-script tracks'
// 71 rows (letters, the precomposed ñ á é í ó ú ü è ê ë ï ä ö ē ç, ordinal
// indicators and opening marks), recognition-only except where the
// Grundschrift-App (a school model) or UJIpenchars2's native Spanish writers
// cite an order, or where a marked letter is drawn BY ANALOGY with the cited
// ü, acute and tilde (its record says it is not separately sourced). The one-storey
// a is cited since its paths moved to LatinPrint-Subset.ttf (from SIL's
// literacy typeface Andika), whose a has the shape every source teaches.
export default [
  {
    suite: "independent (word-initial) vowels",
    suiteOrder: 10,
    caseOrder: 97,
    name: "keeps Latin the last script, with its letters and cited rows intact",
    verify: ({ SCRIPTS }) => {
      expect(SCRIPTS.at(-1)!.script).toBe("latin");
      const latin = SCRIPTS.find((script) => script.script === "latin")!;
      expect(latin.letters).toHaveLength(71);
      expect(latin.letters.slice(0, 5).map((letter) => letter.glyph)).toEqual(["a", "b", "c", "d", "e"]);
      const cited = latin.letters.filter((letter) => letter.strokeOrderSource !== undefined);
      expect(cited.map((letter) => letter.glyph).join("")).toBe("abcdeghilmnopqrstuvwyßñáéíóúüèêëïäöēçGHR¿¡");
      for (const letter of cited) {
        expect(letter.penLifts, letter.glyph).toBeGreaterThanOrEqual(0);
        expect(letter.strokeOrderSource!.url).toMatch(
          /^https:\/\/(github\.com\/Medien-Treibhaus\/grundschrift-app-source\/blob\/f6dbd807adbb3fc2f94207fe578def439f6e9c49\/assets\/levels\/\w+\/\w+\/metadata\.json|archive\.ics\.uci\.edu\/dataset\/177\/uji\+pen\+characters\+version\+2)$/,
        );
      }
      const enye = latin.letters.find((letter) => letter.glyph === "ñ")!;
      expect(enye.strokeOrder.at(-1)).toBe("lift, then the tilde, left to right");
      const acute = latin.letters.find((letter) => letter.glyph === "á")!;
      expect(acute.strokeOrder.at(-1)).toBe("lift, then the acute, up to the right");
      const grave = latin.letters.find((letter) => letter.glyph === "è")!;
      expect(grave.strokeOrder.at(-1)).toBe("lift, then the grave, down to the right");
      expect(grave.strokeOrderSource!.citation).toMatch(/^By analogy with the cited é and ñ, not separately sourced: /);
    },
  },
] satisfies readonly GlyphEvidence[];
