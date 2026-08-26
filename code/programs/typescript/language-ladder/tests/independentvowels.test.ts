import { describe, it, expect } from "vitest";
import { SCRIPTS } from "@coding-adventures/script-ductus";
import { isSyllabary } from "../src/syllabary";
import { buildSyllableMatrix } from "../src/matrix";

const DRAVIDIAN = ["telugu", "kannada", "malayalam"] as const;

describe("independent (word-initial) vowels", () => {
  it("Telugu carries the 13 independent vowels, grounded glyph + ISO-15919 roman", () => {
    const telugu = SCRIPTS.find((s) => s.script === "telugu")!;
    const iv = telugu.independentVowels!;
    expect(iv.map((v) => v.glyph)).toEqual(
      ["అ", "ఆ", "ఇ", "ఈ", "ఉ", "ఊ", "ఎ", "ఏ", "ఒ", "ఓ", "ఐ", "ఔ", "ఋ"],
    );
    expect(iv.map((v) => v.sound)).toEqual(
      ["a", "ā", "i", "ī", "u", "ū", "e", "ē", "o", "ō", "ai", "au", "r̥"],
    );
    // Independent vowels are vowels, not syllables. అ and ఎ have crossed the
    // source-and-font gate; every other row remains free of fabricated ductus.
    expect(iv.every((v) => v.role === "vowel")).toBe(true);
    expect(iv[0]!.strokeOrder).toHaveLength(4);
    expect(iv[0]!.penLifts).toBe(1);
    expect(iv[0]!.strokeOrderSource?.url).toBe(
      "https://write-telugu-alphabets.en.aptoide.com/app",
    );
    expect(iv[6]!.strokeOrder).toHaveLength(3);
    expect(iv[6]!.penLifts).toBe(1);
    expect(iv[6]!.strokeOrderSource?.url).toBe(
      "https://write-telugu-alphabets.en.aptoide.com/app",
    );
    expect(iv.filter((_, index) => index !== 0 && index !== 6).every((v) => v.strokeOrder.length === 0)).toBe(true);
    // The vocalic-R vowel is ISO-15919 r̥ = r + U+0325 (ring below), not IAST ṛ.
    expect([...iv[12]!.sound].map((c) => c.codePointAt(0))).toEqual([0x72, 0x325]);
  });

  it("all three Dravidian scripts carry them", () => {
    DRAVIDIAN.forEach((id) => {
      const s = SCRIPTS.find((x) => x.script === id)!;
      expect(s.independentVowels?.length).toBe(13);
    });
  });

  it("keeps Kannada independent ಅ and ಇ sourced while the remaining vowels stay unverified", () => {
    const kannada = SCRIPTS.find((s) => s.script === "kannada")!;
    const iv = kannada.independentVowels!;
    expect(iv[0]!.glyph).toBe("ಅ");
    expect(iv[0]!.strokeOrder).toHaveLength(4);
    expect(iv[0]!.penLifts).toBe(0);
    expect(iv[0]!.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Kannada-alphabet-a.gif",
    );
    expect(iv[2]!.glyph).toBe("ಇ");
    expect(iv[2]!.strokeOrder).toHaveLength(4);
    expect(iv[2]!.penLifts).toBe(0);
    expect(iv[2]!.strokeOrderSource?.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Animation_of_hand-writing_Kannada_character_%22%E0%B2%87%22.gif",
    );
    expect(iv.filter((_, index) => index !== 0 && index !== 2).every((v) => v.strokeOrder.length === 0)).toBe(true);
  });

  it("CONTROL: they are SEPARATE from the syllabary — letters, isSyllabary and the matrix are untouched", () => {
    const telugu = SCRIPTS.find((s) => s.script === "telugu")!;
    // None of the independent-vowel glyphs leak into the consonant syllable list…
    const syllableGlyphs = new Set(telugu.letters.map((l) => l.glyph));
    expect(telugu.independentVowels!.some((v) => syllableGlyphs.has(v.glyph))).toBe(false);
    // …so the script still reads as an all-syllable syllabary, and the matrix
    // still builds a full 35 × 13 grid (adding the vowels changed neither).
    expect(isSyllabary(telugu.letters)).toBe(true);
    const m = buildSyllableMatrix(telugu.letters as never)!;
    expect(m.rows.length).toBe(35);
    expect(m.rows.every((r) => r.cells.length === 13)).toBe(true);
  });

  it("an alphabet (Cyrillic) has no independent-vowel list", () => {
    const cyr = SCRIPTS.find((s) => s.script === "cyrillic")!;
    expect(cyr.independentVowels).toBeUndefined();
  });
});

describe("atomic final consonants", () => {
  it("keeps Malayalam chillus sourced and outside the all-syllable grid", () => {
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
  });

  it("does not invent final-consonant inventories for the sibling scripts", () => {
    for (const id of ["telugu", "kannada"] as const) {
      expect(SCRIPTS.find((script) => script.script === id)!.finalConsonants).toBeUndefined();
    }
  });
});

describe("source-verified base consonants", () => {
  it("keeps Malayalam ഴ as a complete sourced row in the syllable matrix", () => {
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
  });
});

describe("Tamil independent vowels in the starter inventory", () => {
  it("keeps short உ sourced as one joined Frame 16 run", () => {
    const tamil = SCRIPTS.find((script) => script.script === "tamil")!;
    const shortU = tamil.letters.find((entry) => entry.glyph === "உ")!;
    expect(shortU.role).toBe("independent-vowel");
    expect(shortU.sound).toBe("u");
    expect(shortU.penLifts).toBe(0);
    expect(shortU.strokeOrder).toHaveLength(3);
    expect(shortU.strokeOrderSource?.url).toBe(
      "https://sites.la.utexas.edu/tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
  });
});

describe("shared Perso-Arabic letters retain script-owned provenance", () => {
  it("keeps Arabic maddah as sourced alif-plus-mark composition", () => {
    const arabic = SCRIPTS.find((script) => script.script === "arabic")!;
    const maddah = arabic.marks!.find((entry) => entry.mark === "ٓ")!;
    expect(maddah.sound).toBe("ʾā (long initial ā)");
    expect(maddah.example?.combined).toBe("آ");
    expect(maddah.compositionOrder).toHaveLength(2);
    expect(maddah.compositionOrder?.[0]).toMatch(/alif carrier downward/i);
    expect(maddah.compositionOrder?.[1]).toMatch(/maddah above.*horizontal wave/i);
    expect(maddah.compositionSource?.url).toBe(
      "https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-9/",
    );
  });

  it("keeps Persian and Urdu maddah on each script's sourced alif carrier", () => {
    const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
      .marks!.find((entry) => entry.mark === "ٓ")!;
    const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
      .marks!.find((entry) => entry.mark === "ٓ")!;
    expect(persian.example?.combined).toBe("آ");
    expect(urdu.example?.combined).toBe("آ");
    expect(persian.compositionOrder?.[0]).toMatch(/Persian alif carrier downward/i);
    expect(urdu.compositionOrder?.[0]).toMatch(/Urdu alif carrier downward/i);
    expect(persian.compositionSource?.url).toBe(urdu.compositionSource?.url);
    expect(persian.compositionSource?.variation).toMatch(/Persian curriculum/i);
    expect(urdu.compositionSource?.variation).toMatch(/Urdu curriculum.*Nastaliq/i);
  });


  it("keeps Persian and Urdu چ body-first with independently sourced provenance", () => {
    const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
      .letters.find((entry) => entry.glyph === "چ")!;
    const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
      .letters.find((entry) => entry.glyph === "چ")!;
    expect(persian.sound).toBe("ch");
    expect(persian.penLifts).toBe(3);
    expect(persian.strokeOrder).toHaveLength(5);
    expect(persian.strokeOrder[0]).toMatch(/head.*left to right/i);
    expect(persian.strokeOrder[2]).toMatch(/lower-left dot/i);
    expect(persian.strokeOrder[4]).toMatch(/lower-center dot/i);
    expect(persian.strokeOrderSource?.url).toContain(
      "laits.utexas.edu/persian_grammar/video",
    );
    expect(urdu.sound).toBe("ch");
    expect(urdu.penLifts).toBe(3);
    expect(urdu.strokeOrder).toHaveLength(5);
    expect(urdu.strokeOrder[0]).toMatch(/pointed hooked head/i);
    expect(urdu.strokeOrder[2]).toMatch(/lower-left dot/i);
    expect(urdu.strokeOrder[4]).toMatch(/lower-center dot/i);
    expect(urdu.strokeOrderSource?.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/te-mim-jim-che/",
    );
    expect(persian.strokeOrderSource?.url).not.toBe(urdu.strokeOrderSource?.url);
  });

  it("keeps Urdu ھ as one sourced two-eyed aspiration path", () => {
    const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
      .letters.find((entry) => entry.glyph === "ھ")!;
    expect(urdu.role).toBe("other");
    expect(urdu.sound).toMatch(/aspirates the preceding consonant/i);
    expect(urdu.penLifts).toBe(0);
    expect(urdu.strokeOrder).toHaveLength(4);
    expect(urdu.strokeOrder[0]).toMatch(/right eye clockwise/i);
    expect(urdu.strokeOrder[3]).toMatch(/leftward sweep.*without lifting/i);
    expect(urdu.strokeOrderSource?.url).toBe(
      "https://openbooks.library.northwestern.edu/zerozabar/chapter/chhoti-he-do-chashmi-he-chhoti-ye-bari-ye/",
    );
  });

  it("keeps Persian and Urdu پ separate while both preserve the four-stroke triangle order", () => {
    const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
      .letters.find((entry) => entry.glyph === "پ")!;
    const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
      .letters.find((entry) => entry.glyph === "پ")!;
    expect(persian.penLifts).toBe(3);
    expect(urdu.penLifts).toBe(3);
    expect(persian.strokeOrder).toHaveLength(4);
    expect(urdu.strokeOrder).toHaveLength(4);
    expect(urdu.strokeOrder[1]).toMatch(/lower-left dot.*main line/i);
    expect(urdu.strokeOrder[3]).toMatch(/lower-center dot/i);
    expect(persian.strokeOrderSource?.url).not.toBe(urdu.strokeOrderSource?.url);
  });

  it("keeps Persian and Urdu ف separate while both use one lifted upper dot", () => {
    const persian = SCRIPTS.find((script) => script.script === "perso-arabic")!
      .letters.find((entry) => entry.glyph === "ف")!;
    const urdu = SCRIPTS.find((script) => script.script === "urdu-nastaliq")!
      .letters.find((entry) => entry.glyph === "ف")!;
    expect(persian.penLifts).toBe(1);
    expect(urdu.penLifts).toBe(1);
    expect(persian.strokeOrder).toHaveLength(3);
    expect(urdu.strokeOrder).toHaveLength(3);
    expect(persian.strokeOrderSource?.url).not.toBe(urdu.strokeOrderSource?.url);
  });
});
