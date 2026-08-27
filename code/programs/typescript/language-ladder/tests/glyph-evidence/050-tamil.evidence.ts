import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

export default [
  {
    suite: "Tamil independent vowels in the starter inventory",
    suiteOrder: 40,
    caseOrder: 10,
    name: "keeps short உ sourced as one joined Frame 16 run",
    verify: ({ SCRIPTS }) => {
      const tamil = SCRIPTS.find((script) => script.script === "tamil")!;
      const shortU = tamil.letters.find((entry) => entry.glyph === "உ")!;
      expect(shortU.role).toBe("independent-vowel");
      expect(shortU.sound).toBe("u");
      expect(shortU.penLifts).toBe(0);
      expect(shortU.strokeOrder).toHaveLength(3);
      expect(shortU.strokeOrderSource?.url).toBe(
        "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
      );
    },
  },
  {
    suite: "Tamil consonants in the starter inventory",
    suiteOrder: 40,
    caseOrder: 20,
    name: "keeps ஞ sourced as four Frame 8 runs",
    verify: ({ SCRIPTS }) => {
      const tamil = SCRIPTS.find((script) => script.script === "tamil")!;
      const nya = tamil.letters.find((entry) => entry.glyph === "ஞ")!;
      expect(nya.sound).toBe("ña");
      expect(nya.penLifts).toBe(3);
      expect(nya.strokeOrder).toHaveLength(8);
      expect(nya.strokeOrderSource?.citation).toMatch(/Frame 8.*ஞ.*p\. 194/i);
    },
  },
  {
    suite: "Tamil independent vowels in the starter inventory",
    suiteOrder: 40,
    caseOrder: 30,
    name: "constructs long ஊ from familiar உ and ள",
    verify: ({ SCRIPTS }) => {
      const tamil = SCRIPTS.find((script) => script.script === "tamil")!;
      const longU = tamil.letters.find((entry) => entry.glyph === "ஊ")!;
      expect(longU.sound).toBe("ū");
      expect(longU.penLifts).toBe(3);
      expect(longU.strokeOrder).toHaveLength(9);
      expect(longU.strokeOrderSource?.url).toContain("frame-17");
    },
  },
  {
    suite: "Tamil independent vowels in the starter inventory",
    suiteOrder: 40,
    caseOrder: 40,
    name: "keeps short ஒ sourced as two Frame 14 runs",
    verify: ({ SCRIPTS }) => {
      const tamil = SCRIPTS.find((script) => script.script === "tamil")!;
      const shortO = tamil.letters.find((entry) => entry.glyph === "ஒ")!;
      expect(shortO.sound).toBe("o");
      expect(shortO.penLifts).toBe(1);
      expect(shortO.strokeOrder).toHaveLength(3);
      expect(shortO.strokeOrderSource?.citation).toMatch(/Module 14.*Frame 14/i);
    },
  },
] satisfies readonly GlyphEvidence[];
