// The Malayalam anusvara ം in a filmstrip. It is WRITTEN after its base, as a
// ring to the right of it: Rodney F. Moag's Malayalam: A University Course and
// Reference Grammar numbers the ring of അം as movement 9, after the eight of
// അ, and draws ം by itself to the right of a dash standing for the consonant.
// Moag draws the vowel signs beside the same dash but never numbers the
// consonant against them. ാ and the candrakkala ് have rows of their own,
// cited to Jayasree's composer (malayalam-signs-after-their-letter); every
// other Malayalam vowel sign stays refused inside a word, and is drawn only
// when a lesson teaches it alone. This file holds the anusvara's row to its
// mark record.
import { describe, expect, it } from "vitest";
import {
  filmstripImageMarkdown,
  FUSED_SIGN_PAIRS,
  WRITTEN_SIGN_SIDES,
  writingLetterOf,
  writingSequenceOf,
  writtenPiecesOf,
} from "../../src/figure-targets.js";
import { defaultCurriculumRoot, loadScripts } from "../../src/loader.js";
import { lesson } from "./fixture.js";

const MOAG_SCAN =
  "https://github.com/matjic/malayalam/blob/7141acd2f310bc8928822a7aec61d1149efa6fa3/docs/assets/images/front-writing-029.jpg";
const malayalam = (id: string, headword: string) => lesson(id, { language: "malayalam", headword });

describe("where the Malayalam anusvara is written", () => {
  it("puts ം after its base, whatever the base", () => {
    expect(writtenPiecesOf("അം", "malayalam")).toEqual(["അ", "ം"]);
    expect(writtenPiecesOf("കം", "malayalam")).toEqual(["ക", "ം"]);
    expect(writtenPiecesOf("ണം", "malayalam")).toEqual(["ണ", "ം"]);
  });

  it("refuses every vowel sign without a row, and any second sign before ം", () => {
    // ാ and ് have rows (malayalam-signs-after-their-letter), but no source
    // orders either against a ring after the same letter.
    for (const grapheme of ["കി", "കു", "കെ", "കേ", "കൊ", "കോ", "കൈ", "കാം", "കോം", "ക്ക"]) {
      expect(writtenPiecesOf(grapheme, "malayalam"), grapheme).toBeUndefined();
    }
    // A sign taught by itself is a one-glyph strip, not a sequence: the
    // ledger decides whether its own ductus is cited.
    expect(writingSequenceOf(malayalam("ML-S1", "ാ"), "malayalam")).toBeUndefined();
    expect(writingLetterOf(malayalam("ML-S1", "ാ"))).toBe("ാ");
    expect(writingSequenceOf(malayalam("ML-S2", "ം"), "malayalam")).toBeUndefined();
    expect(writingLetterOf(malayalam("ML-S2", "ം"))).toBe("ം");
  });

  it("draws a word whose only sign is ം letter by letter, then the ring", () => {
    const money = malayalam("ML-W1", "പണം");
    expect(writingSequenceOf(money, "malayalam")).toEqual(["പ", "ണ", "ം"]);
    // The ring is written last, where it is also typed; as with every strip
    // that holds a sign, the caption names the word "part by part".
    expect(
      filmstripImageMarkdown({
        kind: "script-filmstrip",
        lessonId: "ML-W1",
        script: "malayalam",
        glyph: "പണം",
        letters: ["പ", "ണ", "ം"],
        output: "malayalam/book/figures/ML-W1-filmstrip.svg",
      }),
    ).toBe("![How പണം is written, part by part, stroke by stroke](figures/ML-W1-filmstrip.svg)");
    // A word with a sign that has no row, or a cluster the font fuses, stays
    // refused (നമസ്കാരം, whose സ്ക the font prints apart, is drawn: see
    // malayalam-signs-after-their-letter).
    expect(writingSequenceOf(malayalam("ML-W2", "സന്തോഷം"), "malayalam")).toBeUndefined();
    expect(writingSequenceOf(malayalam("ML-W3", "കെട്ടിടം"), "malayalam")).toBeUndefined();
    // A list keeps each item as it is: ാ has no row, so it stays one piece.
    expect(writingSequenceOf(malayalam("ML-W4", "ാ ര ം"), "malayalam")).toEqual(["ാ", "ര", "ം"]);
  });

  it("holds the anusvara's row to its cited mark record", () => {
    expect(WRITTEN_SIGN_SIDES.malayalam!["ം"]).toBe("after");
    // The bundled font fuses no base with ം: its GSUB lookups that mention
    // the anusvara only reorder it against Vedic signs.
    expect(FUSED_SIGN_PAIRS.malayalam).toBeUndefined();
    const script = loadScripts(defaultCurriculumRoot()).malayalam!;
    const anusvara = (script.marks ?? []).find((entry) => entry.mark === "ം")!;
    expect(anusvara.compositionOrder).toEqual([
      "write the Malayalam base first",
      "add the anusvara after it",
    ]);
    expect(anusvara.compositionSource?.url).toBe(MOAG_SCAN);
    expect(anusvara.compositionSource?.citation).toMatch(
      /^Rodney F\. Moag, Malayalam: A University Course and Reference Grammar .*Table II .*p\. xix: movements 1-9 for അം.*Table III, p\. xxiv/,
    );
    expect(anusvara.compositionSource?.variation).toMatch(
      /eight movements of അ first and the ring of ം last, as movement 9.*base is written first and the anusvara after it/,
    );
    // No vowel-sign record claims a written order the table leaves out.
    for (const mark of script.marks ?? []) {
      if (mark.mark in WRITTEN_SIGN_SIDES.malayalam!) continue;
      expect(mark.compositionOrder, mark.mark).toBeUndefined();
      expect(mark.compositionSource, mark.mark).toBeUndefined();
    }
  });
});
