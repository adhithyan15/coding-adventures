import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

export default [
  {
    suite: "independent (word-initial) vowels",
    suiteOrder: 10,
    caseOrder: 40,
    name: "keeps Malayalam independent അ, ആ, ഇ, ഉ, ഊ, എ, ഏ, ഒ, and ഓ sourced while the remaining vowels stay unverified",
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
      expect(iv[5]!.glyph).toBe("ഊ");
      expect(iv[5]!.strokeOrder).toHaveLength(5);
      expect(iv[5]!.penLifts).toBe(1);
      expect(iv[5]!.strokeOrderSource?.url).toBe(
        "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
      );
      expect(iv[6]!.glyph).toBe("എ");
      expect(iv[6]!.strokeOrder).toHaveLength(3);
      expect(iv[6]!.penLifts).toBe(1);
      // ഏ cites Moag's Table II (seven numbered movements, one run).
      expect(iv[7]!.glyph).toBe("ഏ");
      expect(iv[7]!.strokeOrder).toHaveLength(7);
      expect(iv[7]!.penLifts).toBe(0);
      expect(iv[7]!.strokeOrderSource?.url).toBe(
        "https://github.com/matjic/malayalam/blob/7141acd2f310bc8928822a7aec61d1149efa6fa3/docs/assets/images/front-writing-028.jpg",
      );
      expect(iv[8]!.glyph).toBe("ഒ");
      expect(iv[8]!.strokeOrder).toHaveLength(2);
      expect(iv[8]!.penLifts).toBe(1);
      expect(iv[8]!.strokeOrderSource?.url).toBe(
        "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
      );
      expect(iv[9]!.glyph).toBe("ഓ");
      expect(iv[9]!.strokeOrder).toHaveLength(3);
      expect(iv[9]!.penLifts).toBe(2);
      expect(iv[9]!.strokeOrderSource?.url).toBe(
        "https://malayalam.la.utexas.edu/resources/the-malayalam-script/",
      );
      expect(
        iv
          .filter((_, index) => ![0, 1, 2, 4, 5, 6, 7, 8, 9].includes(index))
          .every((v) => v.strokeOrder.length === 0),
      ).toBe(true);
    },
  },
  {
    suite: "atomic final consonants",
    suiteOrder: 20,
    caseOrder: 10,
    name: "keeps Malayalam chillus sourced and outside the all-syllable grid",
    verify: ({ SCRIPTS, isSyllabary, buildSyllableMatrix }) => {
      const malayalam = SCRIPTS.find((s) => s.script === "malayalam")!;
      expect(malayalam.finalConsonants?.map((entry) => entry.glyph)).toEqual([
        "ൺ",
        "ൽ",
        "ൻ",
        "ൾ",
        "ർ",
      ]);
      const chilluNN = malayalam.finalConsonants!.find((entry) => entry.glyph === "ൺ")!;
      expect(chilluNN.role).toBe("consonant");
      expect(chilluNN.penLifts).toBe(0);
      expect(chilluNN.strokeOrder).toHaveLength(5);
      expect(chilluNN.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BA_order.gif",
      );
      expect(malayalam.letters.some((entry) => entry.glyph === "ൺ")).toBe(false);
      const chilluL = malayalam.finalConsonants!.find((entry) => entry.glyph === "ൽ")!;
      expect(chilluL.role).toBe("consonant");
      expect(chilluL.penLifts).toBe(0);
      expect(chilluL.strokeOrder).toHaveLength(5);
      expect(chilluL.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BD_order.gif",
      );
      expect(malayalam.letters.some((entry) => entry.glyph === "ൽ")).toBe(false);
      const chilluN = malayalam.finalConsonants!.find((entry) => entry.glyph === "ൻ")!;
      expect(chilluN.role).toBe("consonant");
      expect(chilluN.penLifts).toBe(1);
      expect(chilluN.strokeOrder).toHaveLength(4);
      expect(chilluN.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BB_order.gif",
      );
      expect(malayalam.letters.some((entry) => entry.glyph === "ൻ")).toBe(false);
      const chilluLL = malayalam.finalConsonants!.find((entry) => entry.glyph === "ൾ")!;
      expect(chilluLL.role).toBe("consonant");
      expect(chilluLL.penLifts).toBe(0);
      expect(chilluLL.strokeOrder).toHaveLength(4);
      expect(chilluLL.strokeOrderSource?.url).toBe(
        "https://commons.wikimedia.org/wiki/File:Ml_%E0%B5%BE_order.gif",
      );
      expect(malayalam.letters.some((entry) => entry.glyph === "ൾ")).toBe(false);
      const chilluRR = malayalam.finalConsonants!.find((entry) => entry.glyph === "ർ")!;
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
  {
    suite: "source-verified base consonants",
    suiteOrder: 30,
    caseOrder: 20,
    name: "keeps the seventeen Thooval-cited Malayalam consonants as one unbroken run each",
    verify: ({ SCRIPTS }) => {
      const malayalam = SCRIPTS.find((script) => script.script === "malayalam")!;
      // glyph -> [Thooval formation-image slug, movements]
      const cited: Record<string, readonly [string, number]> = {
        "ന": ["NA", 4], "മ": ["MA", 4], "സ": ["SA", 5], "ര": ["RA", 3],
        "ത": ["TA", 4], "ഷ": ["SSA", 6], "പ": ["PA", 3], "വ": ["VA", 3],
        "ണ": ["NNA", 6], "ട": ["TTA", 3], "ദ": ["DA", 3], "ഹ": ["HA", 4],
        "ഗ": ["GA", 3], "റ": ["RRA", 2], "ല": ["LA", 5], "ശ": ["SHA", 4],
        "ബ": ["BA", 6],
      };
      for (const [glyph, [slug, movements]] of Object.entries(cited)) {
        const row = malayalam.letters.find((entry) => entry.glyph === glyph)!;
        expect(row.penLifts).toBe(0);
        expect(row.strokeOrder).toHaveLength(movements);
        expect(row.strokeOrderSource?.url).toBe(
          `https://github.com/spacekerala/Thooval/blob/87143b560bf5aab43837d9da2cddab9bd59cd391/data/${slug}.png`,
        );
      }
    },
  },
  {
    suite: "source-verified base consonants",
    suiteOrder: 30,
    caseOrder: 30,
    name: "keeps the Moag-cited Malayalam consonants, ഏ and the vowel signs as one unbroken run each",
    verify: ({ SCRIPTS }) => {
      const malayalam = SCRIPTS.find((script) => script.script === "malayalam")!;
      const moag = (page: string): string =>
        `https://github.com/matjic/malayalam/blob/7141acd2f310bc8928822a7aec61d1149efa6fa3/docs/assets/images/front-writing-${page}.jpg`;
      // glyph -> [scan page of Moag's table, numbered movements]. ക and യ,
      // held while the sources disagreed on their start, now cite Moag.
      const cited: Record<string, readonly [string, number]> = {
        "ക": ["035", 7], "യ": ["040", 4], "ഖ": ["035", 4], "ങ": ["035", 5],
        "ച": ["036", 4], "ഛ": ["036", 5], "ഞ": ["036", 7], "ഥ": ["038", 3],
        "ധ": ["038", 4], "ഭ": ["039", 4], "ഫ": ["039", 3], "ള": ["041", 4],
        "ഏ": ["028", 7],
      };
      for (const [glyph, [page, movements]] of Object.entries(cited)) {
        const row = [...malayalam.letters, ...(malayalam.independentVowels ?? [])].find(
          (entry) => entry.glyph === glyph,
        )!;
        expect(row.penLifts).toBe(0);
        expect(row.strokeOrder).toHaveLength(movements);
        expect(row.strokeOrderSource?.url).toBe(moag(page));
      }
      const signs: Record<string, readonly [string, number]> = {
        "ം": ["034", 1], "ാ": ["030", 1], "ി": ["030", 1], "ീ": ["030", 2],
        "ു": ["031", 3], "ൂ": ["031", 4], "ൃ": ["031", 2], "െ": ["032", 2],
        "േ": ["032", 3],
      };
      for (const [sign, [page, movements]] of Object.entries(signs)) {
        const mark = (malayalam.marks ?? []).find((entry) => entry.mark === sign)!;
        expect(mark.penLifts).toBe(0);
        expect(mark.strokeOrder).toHaveLength(movements);
        expect(mark.strokeOrderSource?.url).toBe(moag(page));
      }
    },
  },
] satisfies readonly GlyphEvidence[];
