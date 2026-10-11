// HL-C443, sequence strips — a headword that is a LIST of letters, or a WORD
// in a script whose letters stand apart, becomes one strip that writes each of
// its cited letters in turn. Everything else that teaches several letters stays
// undrawn, and each refusal below names the reason it would be dishonest.
import { describe, expect, it } from "vitest";
import {
  filmstripCandidates,
  filmstripImageMarkdown,
  filmstripLetters,
  SEPARATE_LETTER_SCRIPTS,
  withDerivedFilmstrips,
  withFilmstripImages,
  writingLetterOf,
  writingSequenceOf,
} from "../../src/figure-targets.js";
import { lesson } from "./fixture.js";

describe("which headwords are sequences", () => {
  it("reads a list of single letters, whatever separates them", () => {
    expect(writingSequenceOf(lesson("TA-W1", { headword: "வ, க" }), "tamil")).toEqual(["வ", "க"]);
    expect(writingSequenceOf(lesson("GU-R1", { headword: "ક — ણ — શ" }), "gujarati")).toEqual(["ક", "ણ", "શ"]);
    expect(writingSequenceOf(lesson("GU-R2", { headword: "છ શ" }), "gujarati")).toEqual(["છ", "શ"]);
    expect(writingSequenceOf(lesson("AR-W1", { headword: "ن، ت، ث" }), "arabic")).toEqual(["ن", "ت", "ث"]);
    expect(writingSequenceOf(lesson("ZH-W1", { headword: "人 · 口" }), "chinese")).toEqual(["人", "口"]);
  });

  it("allows a list in EVERY script, because a listed letter is written on its own", () => {
    // Devanagari letters written one by one each carry their own headline, and
    // an Arabic letter written alone takes its isolated form: both exactly what
    // the per-letter ductus draws.
    expect(writingSequenceOf(lesson("HI-W1", { headword: "न, म" }), "devanagari")).toEqual(["न", "म"]);
    expect(writingSequenceOf(lesson("RU-W1", { headword: "в, р" }), "cyrillic")).toEqual(["в", "р"]);
    // Gurmukhi letters listed one by one likewise each carry their own headline.
    expect(writingSequenceOf(lesson("PA-W1", { headword: "ਟ · ਠ · ਡ" }), "gurmukhi")).toEqual(["ਟ", "ਠ", "ਡ"]);
  });

  it("reads a word letter by letter only in a script whose letters stand apart", () => {
    expect(writingSequenceOf(lesson("JA-W1", { headword: "はい" }), "japanese")).toEqual(["は", "い"]);
    expect(writingSequenceOf(lesson("GU-C1", { headword: "ઘર" }), "gujarati")).toEqual(["ઘ", "ર"]);
    expect(writingSequenceOf(lesson("JA-W2", { headword: "お はよう" }), "japanese")).toEqual([
      "お",
      "は",
      "よ",
      "う",
    ]);
    expect(SEPARATE_LETTER_SCRIPTS.has("japanese")).toBe(true);
  });

  it("refuses a word where letters share a headline, join, or are cursive", () => {
    // मम would print two separate headlines where one is written; سلام would
    // print isolated forms that never appear in the word; привет would assert
    // pen lifts the cited cursive does not make.
    expect(writingSequenceOf(lesson("SA-W1", { headword: "मम" }), "devanagari")).toBeUndefined();
    expect(writingSequenceOf(lesson("UR-W1", { headword: "سلام" }), "urdu-nastaliq")).toBeUndefined();
    expect(writingSequenceOf(lesson("RU-W2", { headword: "привет" }), "cyrillic")).toBeUndefined();
    // ਕਰ (kar) is two cited Gurmukhi letters, but one headline runs across
    // the word, exactly as in मम, so it is refused too.
    expect(writingSequenceOf(lesson("PA-W2", { headword: "ਕਰ" }), "gurmukhi")).toBeUndefined();
    for (const script of ["devanagari", "arabic", "perso-arabic", "urdu-nastaliq", "cyrillic", "gurmukhi"]) {
      expect(SEPARATE_LETTER_SCRIPTS.has(script), script).toBe(false);
    }
  });

  it("refuses a word with a sign no written-order table places, even in a separate-letter script", () => {
    // Some marks are written BEFORE the consonant they follow in Unicode, so a
    // code-point-order strip would draw them in the wrong order. Only Tamil
    // and Gujarati have a table (WRITTEN_SIGN_SIDES), and the Tamil sign ு is
    // not in it; બજાર has a row for ા, but જા fuses into one glyph. See
    // vowel-signs-in-written-order and gujarati-signs-in-written-order.
    expect(writingSequenceOf(lesson("TA-W2", { headword: "பேசு" }), "tamil")).toBeUndefined();
    expect(writingSequenceOf(lesson("GU-W1", { headword: "બજાર" }), "gujarati")).toBeUndefined();
    expect(writingSequenceOf(lesson("JA-W3", { headword: "ラーメン" }), "japanese")).toBeUndefined();
  });

  it("refuses a list that mixes a letter with a word, unless the word itself qualifies", () => {
    expect(writingSequenceOf(lesson("AR-W2", { headword: "ب ت ث — مرحبا" }), "arabic")).toBeUndefined();
    expect(writingSequenceOf(lesson("JA-W4", { headword: "は — はい" }), "japanese")).toEqual(["は", "は", "い"]);
  });

  it("leaves one letter to writingLetterOf, and needs a writing lesson with a letter block", () => {
    expect(writingSequenceOf(lesson("TA-S1"), "tamil")).toBeUndefined();
    expect(writingSequenceOf(lesson("TA-W3", { headword: "" }), "tamil")).toBeUndefined();
    expect(writingSequenceOf(lesson("TA-W4", { headword: " , " }), "tamil")).toBeUndefined();
    expect(writingSequenceOf(lesson("TA-C1", { type: "word", headword: "வ, க" }), "tamil")).toBeUndefined();
    expect(
      writingSequenceOf(lesson("TA-W5", { headword: "வ, க", blocks: ["Warm-up", "Guided Practice"] }), "tamil"),
    ).toBeUndefined();
    const noHeadword = lesson("TA-W6");
    delete (noHeadword.realization as { headword?: string }).headword;
    expect(writingSequenceOf(noHeadword, "tamil")).toBeUndefined();
  });
});

describe("sequence candidates", () => {
  const candidates = filmstripCandidates([
    lesson("TA-S1"),
    lesson("TA-W1", { headword: " வ, க " }),
    lesson("TA-W2", { headword: "பேசு" }),
  ]);

  it("become targets carrying the headword and the letters in order", () => {
    expect(candidates).toEqual([
      {
        kind: "script-filmstrip",
        lessonId: "TA-S1",
        script: "tamil",
        glyph: "அ",
        output: "tamil/book/figures/TA-S1-filmstrip.svg",
      },
      {
        kind: "script-filmstrip",
        lessonId: "TA-W1",
        script: "tamil",
        glyph: "வ, க",
        letters: ["வ", "க"],
        output: "tamil/book/figures/TA-W1-filmstrip.svg",
      },
    ]);
    expect(writingLetterOf(lesson("TA-W1", { headword: "வ, க" }))).toBeUndefined();
    expect(candidates.map(filmstripLetters)).toEqual([["அ"], ["வ", "க"]]);
  });

  it("are drawn only when EVERY letter is cited", () => {
    const cited = new Set(["அ", "வ"]);
    const some = withDerivedFilmstrips([], candidates, (_script, glyph) => cited.has(glyph));
    expect(some.map((target) => target.lessonId)).toEqual(["TA-S1"]);
    cited.add("க");
    const all = withDerivedFilmstrips([], candidates, (_script, glyph) => cited.has(glyph));
    expect(all.map((target) => target.lessonId)).toEqual(["TA-S1", "TA-W1"]);
  });

  it("are captioned as a list or as a word, and placed like a single letter", () => {
    const list = candidates[1]!;
    expect(filmstripImageMarkdown(list)).toBe(
      "![How these letters are written, one after another, stroke by stroke: வ, க](figures/TA-W1-filmstrip.svg)",
    );
    const [word] = filmstripCandidates([lesson("JA-W1", { language: "japanese", headword: "はい" })]);
    expect(filmstripImageMarkdown(word!)).toBe(
      "![How はい is written, letter by letter, stroke by stroke](figures/JA-W1-filmstrip.svg)",
    );
    const [placed] = withFilmstripImages([lesson("TA-W1", { headword: "வ, க" })], [list]);
    expect(placed!.blocks[1]!.markdown.startsWith(filmstripImageMarkdown(list))).toBe(true);
  });
});
