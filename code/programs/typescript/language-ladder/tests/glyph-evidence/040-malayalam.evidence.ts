import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

export default [
  {
    suite: "independent (word-initial) vowels",
    suiteOrder: 10,
    caseOrder: 40,
    name: "keeps Malayalam independent അ, ആ, ഇ, ഉ, and എ sourced while the remaining vowels stay unverified",
    verify: ({ SCRIPTS }) => {
      const malayalam = SCRIPTS.find((s) => s.script === "malayalam")!;
      const iv = malayalam.independentVowels!;
      expect(iv[0]!.glyph).toBe("അ");
      expect(iv[0]!.strokeOrder).toHaveLength(5);
      expect(iv[0]!.penLifts).toBe(1);
      expect(iv[0]!.strokeOrderSource?.url).toBe(
        "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
      );
      expect(iv[1]!.glyph).toBe("ആ");
      expect(iv[1]!.strokeOrder).toHaveLength(5);
      expect(iv[1]!.penLifts).toBe(1);
      expect(iv[1]!.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Ml_%E0%B4%86_order.gif",
      );
      expect(iv[2]!.glyph).toBe("ഇ");
      expect(iv[2]!.strokeOrder).toHaveLength(4);
      expect(iv[2]!.penLifts).toBe(0);
      expect(iv[2]!.strokeOrderSource?.url).toBe(
        "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
      );
      expect(iv[4]!.glyph).toBe("ഉ");
      expect(iv[4]!.strokeOrder).toHaveLength(3);
      expect(iv[4]!.penLifts).toBe(0);
      expect(iv[4]!.strokeOrderSource?.url).toBe(
        "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
      );
      expect(iv[6]!.glyph).toBe("എ");
      expect(iv[6]!.strokeOrder).toHaveLength(3);
      expect(iv[6]!.penLifts).toBe(1);
      expect(iv.filter((_, index) => ![0, 1, 2, 4, 6].includes(index)).every((v) => v.strokeOrder.length === 0)).toBe(true);
    },
  },
  {
    suite: "atomic final consonants",
    suiteOrder: 20,
    caseOrder: 10,
    name: "keeps Malayalam chillus sourced and outside the all-syllable grid",
    verify: ({ SCRIPTS, isSyllabary, buildSyllableMatrix }) => {
      const malayalam = SCRIPTS.find((s) => s.script === "malayalam")!;
      expect(malayalam.finalConsonants?.map((entry) => entry.glyph)).toEqual(["ൽ", "ൻ", "ൾ", "ർ"]);
      const chilluL = malayalam.finalConsonants![0]!;
      expect(chilluL.role).toBe("consonant");
      expect(chilluL.penLifts).toBe(0);
      expect(chilluL.strokeOrder).toHaveLength(5);
      expect(chilluL.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BD_order.gif",
      );
      expect(malayalam.letters.some((entry) => entry.glyph === "ൽ")).toBe(false);
      const chilluN = malayalam.finalConsonants![1]!;
      expect(chilluN.role).toBe("consonant");
      expect(chilluN.penLifts).toBe(1);
      expect(chilluN.strokeOrder).toHaveLength(4);
      expect(chilluN.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BB_order.gif",
      );
      expect(malayalam.letters.some((entry) => entry.glyph === "ൻ")).toBe(false);
      const chilluLL = malayalam.finalConsonants![2]!;
      expect(chilluLL.role).toBe("consonant");
      expect(chilluLL.penLifts).toBe(0);
      expect(chilluLL.strokeOrder).toHaveLength(4);
      expect(chilluLL.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BE_order.gif",
      );
      expect(malayalam.letters.some((entry) => entry.glyph === "ൾ")).toBe(false);
      const chilluRR = malayalam.finalConsonants![3]!;
      expect(chilluRR.role).toBe("consonant");
      expect(chilluRR.penLifts).toBe(0);
      expect(chilluRR.strokeOrder).toHaveLength(3);
      expect(chilluRR.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BC_order.gif",
      );
      expect(malayalam.letters.some((entry) => entry.glyph === "ർ")).toBe(false);
      expect(isSyllabary(malayalam.letters)).toBe(true);
      expect(buildSyllableMatrix(malayalam.letters as never)).not.toBeNull();
    },
  },
  {
    suite: "atomic final consonants",
    suiteOrder: 20,
    caseOrder: 20,
    name: "does not invent final-consonant inventories for the sibling scripts",
    verify: ({ SCRIPTS }) => {
      for (const id of ["telugu", "kannada"] as const) {
        expect(SCRIPTS.find((script) => script.script === id)!.finalConsonants).toBeUndefined();
      }
    },
  },
  {
    suite: "source-verified base consonants",
    suiteOrder: 30,
    caseOrder: 10,
    name: "keeps Malayalam ഴ as a complete sourced row in the syllable matrix",
    verify: ({ SCRIPTS, buildSyllableMatrix }) => {
      const malayalam = SCRIPTS.find((script) => script.script === "malayalam")!;
      const zha = malayalam.letters.find((entry) => entry.glyph === "ഴ")!;
      expect(zha.sound).toBe("ḻa");
      expect(zha.penLifts).toBe(0);
      expect(zha.strokeOrder).toHaveLength(3);
      expect(zha.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Ml_%E0%B4%B4_order.gif",
      );
      const matrix = buildSyllableMatrix(malayalam.letters as never)!;
      const zhaRow = matrix.rows.find((row) => row.cells[0]?.glyph === "ഴ")!;
      expect(zhaRow.cells.map((cell) => cell.glyph)).toEqual([
        "ഴ", "ഴാ", "ഴി", "ഴീ", "ഴു", "ഴൂ", "ഴെ", "ഴേ", "ഴൊ", "ഴോ", "ഴൈ", "ഴൌ", "ഴൃ",
      ]);
    },
  },
] satisfies readonly GlyphEvidence[];


