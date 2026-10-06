// Tamil vowel signs in a filmstrip — a sign is drawn where the hand WRITES it,
// which is not always where Unicode TYPES it. ெ, ே and ை are typed after their
// consonant and written before it; ொ and ோ are written in two halves around
// it. The puḷḷi ் is written after its consonant, as a dot once the body is
// complete. Every side is cited in the Tamil mark records, and this file holds
// the composer's table to them.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  filmstripCandidates,
  filmstripImageMarkdown,
  FUSED_LETTER_SEQUENCE_SOURCES,
  FUSED_SIGN_PAIRS,
  hasFusedLetterSequence,
  WRITTEN_SIGN_SIDES,
  writingLetterOf,
  writingSequenceOf,
  writtenPiecesOf,
} from "../../src/figure-targets.js";
import { defaultCurriculumRoot, loadScripts } from "../../src/loader.js";
import { lesson } from "./fixture.js";

describe("where each Tamil sign is written", () => {
  it("puts a sign written left of its consonant BEFORE it, and one written right of it after", () => {
    expect(writtenPiecesOf("கெ", "tamil")).toEqual(["ெ", "க"]);
    expect(writtenPiecesOf("கே", "tamil")).toEqual(["ே", "க"]);
    expect(writtenPiecesOf("கை", "tamil")).toEqual(["ை", "க"]);
    expect(writtenPiecesOf("கா", "tamil")).toEqual(["க", "ா"]);
    expect(writtenPiecesOf("கி", "tamil")).toEqual(["க", "ி"]);
    expect(writtenPiecesOf("கீ", "tamil")).toEqual(["க", "ீ"]);
    expect(writtenPiecesOf("க", "tamil")).toEqual(["க"]);
  });

  it("puts the puḷḷi after its consonant: the body first, then the dot", () => {
    expect(writtenPiecesOf("க்", "tamil")).toEqual(["க", "்"]);
    expect(writtenPiecesOf("ன்", "tamil")).toEqual(["ன", "்"]);
    expect(writtenPiecesOf("்", "tamil")).toEqual(["்"]);
  });

  it("splits a two-part sign around its consonant, typed whole or in halves", () => {
    // U+0BCA and U+0BCB decompose (NFD) into a left half and ா.
    expect(writtenPiecesOf("கொ", "tamil")).toEqual(["ெ", "க", "ா"]);
    expect(writtenPiecesOf("கோ", "tamil")).toEqual(["ே", "க", "ா"]);
    expect(writtenPiecesOf("கொ", "tamil")).toEqual(["ெ", "க", "ா"]);
    expect(writtenPiecesOf("ோ", "tamil")).toEqual(["ே", "ா"]);
    expect(writtenPiecesOf("ா", "tamil")).toEqual(["ா"]);
  });

  it("refuses every sign whose written place is not cited, and the fused pairs", () => {
    // ு and ூ have no row; ௌ's right half ௗ has none either.
    for (const grapheme of ["கு", "கூ", "கௌ", "ு"]) {
      expect(writtenPiecesOf(grapheme, "tamil"), grapheme).toBeUndefined();
    }
    // Unicode joins these pairs into ligatures of their own.
    for (const grapheme of ["டி", "டீ", "லீ"]) {
      expect(writtenPiecesOf(grapheme, "tamil"), grapheme).toBeUndefined();
    }
    expect(writtenPiecesOf("லி", "tamil")).toEqual(["ல", "ி"]);
    expect(writtenPiecesOf("௭", "tamil")).toBeUndefined();
  });

  it("keeps every other script's signs refused: only Tamil and Gujarati have a table", () => {
    // Gujarati's table is held to its own records in
    // gujarati-signs-in-written-order.
    expect(Object.keys(WRITTEN_SIGN_SIDES)).toEqual(["tamil", "gujarati"]);
    expect(Object.keys(FUSED_SIGN_PAIRS)).toEqual(["tamil", "gujarati"]);
    expect(writtenPiecesOf("कि", "devanagari")).toBeUndefined();
    expect(writtenPiecesOf("が", "japanese")).toEqual(["が"]);
    expect(writtenPiecesOf("ಕಾ", "kannada")).toBeUndefined();
    expect(writingSequenceOf(lesson("GU-W1", { headword: "બજાર" }), "gujarati")).toBeUndefined();
    expect(writingSequenceOf(lesson("TE-W1", { headword: "కాకి" }), "telugu")).toBeUndefined();
  });

  it("matches the cited written place in every Tamil mark record", () => {
    // Each row must stand on a record whose `compositionSource` cites it: a
    // row with no record, or a record saying the other side, fails here.
    const tamil = loadScripts(defaultCurriculumRoot()).tamil!;
    for (const [sign, side] of Object.entries(WRITTEN_SIGN_SIDES.tamil!)) {
      const mark = (tamil.marks ?? []).find((entry) => entry.mark === sign);
      expect(mark, sign).toBeDefined();
      expect(mark!.compositionSource?.citation, sign).toMatch(/\S/);
      expect(mark!.compositionSource?.url, sign).toMatch(/^https:\/\//);
      const order = (mark!.compositionOrder ?? []).join(" | ");
      if (side === "before") expect(order, sign).toMatch(/^in handwriting, write the .* to the left before the primary consonant/);
      else expect(order, sign).toMatch(/^write the Tamil consonant carrier first \| write the .* sign after it/);
    }
    // And no record claims a side the table leaves out.
    for (const mark of tamil.marks ?? []) {
      if (mark.mark in WRITTEN_SIGN_SIDES.tamil!) continue;
      expect(mark.compositionOrder?.join(" ") ?? "", mark.mark).not.toMatch(/before the primary consonant/);
    }
  });
});

describe("Tamil words and signs as strips", () => {
  it("draws a word in written order, and a two-part sign taught alone as its halves", () => {
    expect(writingSequenceOf(lesson("TA-W1", { headword: "மேசை" }), "tamil")).toEqual(["ே", "ம", "ை", "ச"]);
    expect(writingSequenceOf(lesson("TA-W2", { headword: "சொ" }), "tamil")).toEqual(["ெ", "ச", "ா"]);
    expect(writingSequenceOf(lesson("TA-W3", { headword: "சரியா" }), "tamil")).toEqual(["ச", "ர", "ி", "ய", "ா"]);
    expect(writingSequenceOf(lesson("TA-S1", { headword: "ோ" }), "tamil")).toEqual(["ே", "ா"]);
    expect(writingSequenceOf(lesson("TA-S2", { headword: "ா" }), "tamil")).toBeUndefined();
    expect(writingLetterOf(lesson("TA-S2", { headword: "ா" }))).toBe("ா");
  });

  it("refuses a word with any sign that has no cited place", () => {
    expect(writingSequenceOf(lesson("TA-W4", { headword: "பேசு" }), "tamil")).toBeUndefined();
    expect(writingSequenceOf(lesson("TA-W6", { headword: "குடி" }), "tamil")).toBeUndefined();
    expect(writingSequenceOf(lesson("TA-W7", { headword: "வண்டி" }), "tamil")).toBeUndefined();
  });

  it("draws a word with puḷḷi letter by letter, each dot after its consonant", () => {
    expect(writingSequenceOf(lesson("TA-W03", { headword: "வணக்கம்" }), "tamil")).toEqual([
      "வ", "ண", "க", "்", "க", "ம", "்",
    ]);
    // A left-hand sign still comes first; the dot still follows its own letter.
    expect(writingSequenceOf(lesson("TA-W06", { headword: "இல்லை" }), "tamil")).toEqual(["இ", "ல", "்", "ை", "ல"]);
    expect(writingSequenceOf(lesson("TA-W32", { headword: "சொல்" }), "tamil")).toEqual(["ெ", "ச", "ா", "ல", "்"]);
  });

  it("refuses a word in which the font joins letters across the puḷḷi", () => {
    // Noto Sans Tamil prints க்ஷ and ஸ்ரீ as one glyph each, so drawing their
    // parts would draw letters the page does not show.
    expect(hasFusedLetterSequence("லக்ஷ்மி", "tamil")).toBe(true);
    expect(hasFusedLetterSequence("ஸ்ரீ", "tamil")).toBe(true);
    expect(hasFusedLetterSequence("வணக்கம்", "tamil")).toBe(false);
    expect(hasFusedLetterSequence("க்ஷ", "gujarati")).toBe(false);
    expect(writingSequenceOf(lesson("TA-W10", { headword: "லக்ஷ்மி" }), "tamil")).toBeUndefined();
    // Listed apart, the same letters are each written by themselves.
    expect(writingSequenceOf(lesson("TA-W11", { headword: "க், ஷ" }), "tamil")).toEqual(["க", "்", "ஷ"]);
    for (const source of FUSED_LETTER_SEQUENCE_SOURCES.tamil!) {
      expect(source.url).toMatch(/^https:\/\//);
      for (const sequence of source.sequences) {
        expect(sequence.normalize("NFD")).toBe(sequence);
        for (const letter of [...sequence].filter((ch) => /\p{L}/u.test(ch))) {
          expect(source.citation, `${sequence}: ${letter}`).toContain(letter);
        }
      }
    }
    // The lookups were read from the bundled font; if it is replaced, its
    // version string changes and the citation must be read again. The
    // OpenType name table stores the version string as UTF-16BE.
    const font = readFileSync(join(defaultCurriculumRoot(), "_fonts", "NotoSansTamil-Static.ttf"));
    expect(FUSED_LETTER_SEQUENCE_SOURCES.tamil![0]!.citation).toContain("Noto Sans Tamil Version 2.004");
    expect(font.includes(Buffer.from("Version 2.004", "utf16le").swap16())).toBe(true);
  });

  it("keeps a list's digits as they are, and places a list's signs", () => {
    expect(writingSequenceOf(lesson("TA-W8", { headword: "௧ ௨ ௩" }), "tamil")).toEqual(["௧", "௨", "௩"]);
    expect(writingSequenceOf(lesson("TA-W9", { headword: "கெ, கா" }), "tamil")).toEqual(["ெ", "க", "க", "ா"]);
  });

  it("becomes a candidate captioned part by part", () => {
    const [sign, single, word] = filmstripCandidates([
      lesson("TA-S102", { headword: "ோ" }),
      lesson("TA-W26", { headword: "மேசை" }),
      lesson("TA-S114", { headword: "ா" }),
    ]);
    expect(sign).toMatchObject({ lessonId: "TA-S102", glyph: "ோ", letters: ["ே", "ா"] });
    expect(word).toMatchObject({ lessonId: "TA-W26", glyph: "மேசை", letters: ["ே", "ம", "ை", "ச"] });
    expect(single).toEqual({
      kind: "script-filmstrip",
      lessonId: "TA-S114",
      script: "tamil",
      glyph: "ா",
      output: "tamil/book/figures/TA-S114-filmstrip.svg",
    });
    expect(filmstripImageMarkdown(word!)).toBe(
      "![How மேசை is written, part by part, stroke by stroke](figures/TA-W26-filmstrip.svg)",
    );
    expect(filmstripImageMarkdown(sign!)).toBe(
      "![How ோ is written, part by part, stroke by stroke](figures/TA-S102-filmstrip.svg)",
    );
  });
});
