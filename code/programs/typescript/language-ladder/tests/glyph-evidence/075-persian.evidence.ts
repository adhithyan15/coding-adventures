import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

export default [
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 195,
    name: "keeps Persian گ as an independently sourced three-run kāf-family letter",
    verify: ({ SCRIPTS }) => {
      const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
        .letters.find((entry) => entry.glyph === "گ")!;
      expect(persian.sound).toBe("g");
      expect(persian.role).toBe("consonant");
      expect(persian.penLifts).toBe(2);
      expect(persian.strokeOrder).toHaveLength(3);
      expect(persian.strokeOrder[0]).toMatch(/stem downward.*shallow bowl.*hook/i);
      expect(persian.strokeOrder[1]).toMatch(/lift once.*long slash/i);
      expect(persian.strokeOrder[2]).toMatch(/lift again.*shorter floating slash/i);
      expect(persian.strokeOrderSource?.url).toBe(
        "https://laits.utexas.edu/persian_grammar/video/gr/kooroshalphabet",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 196,
    name: "keeps the Persian digits ۰-۹ as one-stroke rows counted from POH-Db",
    verify: ({ SCRIPTS }) => {
      const digits = SCRIPTS.find((script) => script.script === "perso-arabic")!.digits ?? [];
      expect(digits.map((digit) => digit.glyph)).toEqual([..."۰۱۲۳۴۵۶۷۸۹"]);
      for (const digit of digits) {
        expect(digit.penLifts, digit.glyph).toBe(0);
        expect(digit.strokeOrderSource?.citation, digit.glyph).toMatch(
          /^POH-Db, Persian Online Handwriting Database .*AGPL-3\.0, facts only\)/,
        );
      }
    },
  },
] satisfies readonly GlyphEvidence[];
