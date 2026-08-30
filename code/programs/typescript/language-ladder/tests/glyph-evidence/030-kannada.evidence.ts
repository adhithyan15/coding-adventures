import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

export default [
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 225,
    name: "keeps Kannada visarga as two source-backed closed-loop runs",
    verify: ({ SCRIPTS }) => {
      const visarga = SCRIPTS.find((script) => script.script === "kannada")!
        .marks!.find((entry) => entry.mark === "ಃ")!;
      expect(visarga.sound).toBe("ḥ");
      expect(visarga.role).toBe("other");
      expect(visarga.penLifts).toBe(1);
      expect(visarga.strokeOrder).toEqual([
        "draw the upper dot as a closed loop",
        "lift, then draw the lower dot as a closed loop",
      ]);
      expect(visarga.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-Alphabet-Aha.gif",
      );
      expect(visarga.strokeOrderSource?.variation).toMatch(
        /standalone U\+0C83.*excludes that carrier/i,
      );
    },
  },
  {
    suite: "independent (word-initial) vowels",
    suiteOrder: 10,
    caseOrder: 30,
    name: "keeps Kannada independent ಅ, ಆ, ಇ, ಈ, ಉ, ಊ, ಎ, ಏ, ಒ, ಓ, ಐ, and ಋ sourced while the remaining vowels stay unverified",
    verify: ({ SCRIPTS }) => {
      const kannada = SCRIPTS.find((s) => s.script === "kannada")!;
      const iv = kannada.independentVowels!;
      expect(iv[0]!.glyph).toBe("ಅ");
      expect(iv[0]!.strokeOrder).toHaveLength(4);
      expect(iv[0]!.penLifts).toBe(0);
      expect(iv[0]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-a.gif",
      );
      expect(iv[1]!.glyph).toBe("ಆ");
      expect(iv[1]!.strokeOrder).toHaveLength(4);
      expect(iv[1]!.penLifts).toBe(1);
      expect(iv[1]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-aa.gif",
      );
      expect(iv[2]!.glyph).toBe("ಇ");
      expect(iv[2]!.strokeOrder).toHaveLength(4);
      expect(iv[2]!.penLifts).toBe(0);
      expect(iv[2]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Animation_of_hand-writing_Kannada_character_%22%E0%B2%87%22.gif",
      );
      expect(iv[3]!.glyph).toBe("ಈ");
      expect(iv[3]!.strokeOrder).toHaveLength(4);
      expect(iv[3]!.penLifts).toBe(1);
      expect(iv[3]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ee.gif",
      );
      expect(iv[4]!.glyph).toBe("ಉ");
      expect(iv[4]!.strokeOrder).toHaveLength(4);
      expect(iv[4]!.penLifts).toBe(0);
      expect(iv[4]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-u.gif",
      );
      expect(iv[5]!.glyph).toBe("ಊ");
      expect(iv[5]!.strokeOrder).toHaveLength(4);
      expect(iv[5]!.penLifts).toBe(0);
      expect(iv[5]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-uu.gif",
      );
      expect(iv[6]!.glyph).toBe("ಎ");
      expect(iv[6]!.strokeOrder).toHaveLength(4);
      expect(iv[6]!.penLifts).toBe(0);
      expect(iv[6]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ae.gif",
      );
      expect(iv[7]!.glyph).toBe("ಏ");
      expect(iv[7]!.strokeOrder).toHaveLength(4);
      expect(iv[7]!.penLifts).toBe(1);
      expect(iv[7]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-aee.gif",
      );
      expect(iv[8]!.glyph).toBe("ಒ");
      expect(iv[8]!.strokeOrder).toHaveLength(4);
      expect(iv[8]!.penLifts).toBe(0);
      expect(iv[8]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-o.gif",
      );
      expect(iv[9]!.glyph).toBe("ಓ");
      expect(iv[9]!.strokeOrder).toHaveLength(5);
      expect(iv[9]!.penLifts).toBe(1);
      expect(iv[9]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-oo.gif",
      );
      expect(iv[10]!.glyph).toBe("ಐ");
      expect(iv[10]!.strokeOrder).toHaveLength(3);
      expect(iv[10]!.penLifts).toBe(0);
      expect(iv[10]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ai.gif",
      );
      expect(iv[12]!.glyph).toBe("ಋ");
      expect(iv[12]!.strokeOrder).toHaveLength(3);
      expect(iv[12]!.penLifts).toBe(2);
      expect(iv[12]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ru.gif",
      );
      expect(iv.filter((_, index) => ![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12].includes(index)).every((v) => v.strokeOrder.length === 0)).toBe(true);
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 160,
    name: "keeps Kannada ಉ as a source-backed one-run independent vowel",
    verify: ({ SCRIPTS }) => {
      const kannada = SCRIPTS.find((script) => script.script === "kannada")!
        .independentVowels!.find((entry) => entry.glyph === "ಉ")!;
      expect(kannada.sound).toBe("u");
      expect(kannada.role).toBe("vowel");
      expect(kannada.penLifts).toBe(0);
      expect(kannada.strokeOrder).toHaveLength(4);
      expect(kannada.strokeOrder[0]).toMatch(/upper-left loop/i);
      expect(kannada.strokeOrder[1]).toMatch(/without lifting.*lower-left bowl/i);
      expect(kannada.strokeOrder[2]).toMatch(/without lifting.*tall middle arch.*lower-right bowl/i);
      expect(kannada.strokeOrder[3]).toMatch(/without lifting.*open upper terminal/i);
      expect(kannada.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-u.gif",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 170,
    name: "keeps Kannada ಊ as a source-backed one-run independent vowel",
    verify: ({ SCRIPTS }) => {
      const kannada = SCRIPTS.find((script) => script.script === "kannada")!
        .independentVowels!.find((entry) => entry.glyph === "ಊ")!;
      expect(kannada.sound).toBe("ū");
      expect(kannada.role).toBe("vowel");
      expect(kannada.penLifts).toBe(0);
      expect(kannada.strokeOrder).toHaveLength(4);
      expect(kannada.strokeOrder[0]).toMatch(/upper-left spiral/i);
      expect(kannada.strokeOrder[2]).toMatch(/without lifting.*first tall arch.*second arch/i);
      expect(kannada.strokeOrder[3]).toMatch(/without lifting.*lower-right spiral/i);
      expect(kannada.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-uu.gif",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 180,
    name: "keeps Kannada ಏ as a source-backed two-run independent vowel",
    verify: ({ SCRIPTS }) => {
      const kannada = SCRIPTS.find((script) => script.script === "kannada")!
        .independentVowels!.find((entry) => entry.glyph === "ಏ")!;
      expect(kannada.sound).toBe("ē");
      expect(kannada.role).toBe("vowel");
      expect(kannada.penLifts).toBe(1);
      expect(kannada.strokeOrder).toHaveLength(4);
      expect(kannada.strokeOrder[0]).toMatch(/compact left loop/i);
      expect(kannada.strokeOrder[2]).toMatch(/without lifting.*tall outer arch.*upper left/i);
      expect(kannada.strokeOrder[3]).toMatch(/lift.*small upper loop.*left to right/i);
      expect(kannada.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-aee.gif",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 190,
    name: "keeps Kannada ಒ as a source-backed one-run independent vowel",
    verify: ({ SCRIPTS }) => {
      const kannada = SCRIPTS.find((script) => script.script === "kannada")!
        .independentVowels!.find((entry) => entry.glyph === "ಒ")!;
      expect(kannada.sound).toBe("o");
      expect(kannada.role).toBe("vowel");
      expect(kannada.penLifts).toBe(0);
      expect(kannada.strokeOrder).toHaveLength(4);
      expect(kannada.strokeOrder[0]).toMatch(/upper-left loop/i);
      expect(kannada.strokeOrder[1]).toMatch(/without lifting.*curved middle.*lower-left bowl/i);
      expect(kannada.strokeOrder[3]).toMatch(/without lifting.*open terminal/i);
      expect(kannada.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-o.gif",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 200,
    name: "keeps Kannada ಓ as a source-backed two-run independent vowel",
    verify: ({ SCRIPTS }) => {
      const kannada = SCRIPTS.find((script) => script.script === "kannada")!
        .independentVowels!.find((entry) => entry.glyph === "ಓ")!;
      expect(kannada.sound).toBe("ō");
      expect(kannada.role).toBe("vowel");
      expect(kannada.penLifts).toBe(1);
      expect(kannada.strokeOrder).toHaveLength(5);
      expect(kannada.strokeOrder[0]).toMatch(/upper-left loop/i);
      expect(kannada.strokeOrder[3]).toMatch(/without lifting.*open terminal/i);
      expect(kannada.strokeOrder[4]).toMatch(/lift.*small upper flourish/i);
      expect(kannada.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-oo.gif",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 210,
    name: "keeps Kannada ಐ as a source-backed one-run independent vowel",
    verify: ({ SCRIPTS }) => {
      const kannada = SCRIPTS.find((script) => script.script === "kannada")!
        .independentVowels!.find((entry) => entry.glyph === "ಐ")!;
      expect(kannada.sound).toBe("ai");
      expect(kannada.role).toBe("vowel");
      expect(kannada.penLifts).toBe(0);
      expect(kannada.strokeOrder).toHaveLength(3);
      expect(kannada.strokeOrder[0]).toMatch(/left spiral.*lower bowl/i);
      expect(kannada.strokeOrder[1]).toMatch(/without lifting.*broad right loop/i);
      expect(kannada.strokeOrder[2]).toMatch(/without lifting.*high arch.*upper-left terminal/i);
      expect(kannada.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ai.gif",
      );
    },
  },
  {
    suite: "shared Perso-Arabic letters retain script-owned provenance",
    suiteOrder: 50,
    caseOrder: 220,
    name: "keeps Kannada ಋ as a source-backed three-run independent vowel",
    verify: ({ SCRIPTS }) => {
      const kannada = SCRIPTS.find((script) => script.script === "kannada")!
        .independentVowels!.find((entry) => entry.glyph === "ಋ")!;
      expect(kannada.sound).toBe("r̥");
      expect(kannada.role).toBe("vowel");
      expect(kannada.penLifts).toBe(2);
      expect(kannada.strokeOrder).toHaveLength(3);
      expect(kannada.strokeOrder[0]).toMatch(/upper-left spiral.*lower-left spiral.*middle bowl/i);
      expect(kannada.strokeOrder[1]).toMatch(/lift.*inward bar.*high hook/i);
      expect(kannada.strokeOrder[2]).toMatch(/lift.*rightward.*lower bowl.*open upper terminal/i);
      expect(kannada.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-ru.gif",
      );
    },
  },
] satisfies readonly GlyphEvidence[];
