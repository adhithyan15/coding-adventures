// Malayalam ാ and the candrakkala ് in a filmstrip. Both are WRITTEN after the
// letter they follow, on the side they are printed: Jayasree's composer
// (github.com/sachn1/jayasree, commit e0c9d57) animates a consonant with
// either sign as the consonant's recorded strokes first and the sign's after
// them, and its recorder's own ോ and ൊ end with ാ. That is the composer's
// order, not a recording of the pair, so each mark record says confidence is
// medium. This file holds the two rows to those records.
//
// The candrakkala also joins two consonants inside ONE grapheme (സ്കാ). The
// bundled font prints a few such clusters as their unchanged parts, and only a
// cluster cited in APART_CLUSTER_SOURCES is drawn that way: consonant,
// candrakkala, then the next consonant. A cluster the font fuses (ന്ത, മ്മ)
// stays refused. A digit stands apart, one piece by itself.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  APART_CLUSTER_SOURCES,
  filmstripImageMarkdown,
  WRITTEN_SIGN_SIDES,
  writingSequenceOf,
  writtenPiecesOf,
} from "../../src/figure-targets.js";
import { defaultCurriculumRoot, loadScripts } from "../../src/loader.js";
import { lesson } from "./fixture.js";

const JAYASREE_COMPOSER =
  "https://github.com/sachn1/jayasree/blob/e0c9d57dd32031c948da4d5f8432aae3e22c5bba/js/src/index.js";
const malayalam = (id: string, headword: string) => lesson(id, { language: "malayalam", headword });

describe("where Malayalam ാ and ് are written", () => {
  it("puts each after the consonant it follows", () => {
    expect(writtenPiecesOf("കാ", "malayalam")).toEqual(["ക", "ാ"]);
    expect(writtenPiecesOf("രാ", "malayalam")).toEqual(["ര", "ാ"]);
    expect(writtenPiecesOf("ഴ്", "malayalam")).toEqual(["ഴ", "്"]);
    expect(writtenPiecesOf("ക്", "malayalam")).toEqual(["ക", "്"]);
    // Taught alone, each is still one piece: the one-glyph strip draws it.
    expect(writtenPiecesOf("ാ", "malayalam")).toEqual(["ാ"]);
    expect(writtenPiecesOf("്", "malayalam")).toEqual(["്"]);
    expect(writingSequenceOf(malayalam("ML-S1", "്"), "malayalam")).toBeUndefined();
  });

  it("keeps the left-hand signs, and two signs after one letter, refused", () => {
    // ോ is േ + ാ (NFD): ാ has a row, but േ does not.
    for (const grapheme of ["കോ", "കൊ", "കേ", "കാം", "ക്ം"]) {
      expect(writtenPiecesOf(grapheme, "malayalam"), grapheme).toBeUndefined();
    }
  });

  it("holds both rows to their cited mark records", () => {
    expect(WRITTEN_SIGN_SIDES.malayalam).toEqual({ "ം": "after", "ാ": "after", "്": "after" });
    const marks = loadScripts(defaultCurriculumRoot()).malayalam!.marks ?? [];
    const aa = marks.find((entry) => entry.mark === "ാ")!;
    expect(aa.compositionOrder).toEqual(["write the Malayalam consonant first", "add ാ after it, to its right"]);
    const candrakkala = marks.find((entry) => entry.mark === "്")!;
    expect(candrakkala.compositionOrder).toEqual([
      "write the Malayalam carrier first",
      "add the candrakkala to suppress its inherent vowel or prepare the following conjunct",
    ]);
    for (const [sign, record] of [["ാ", aa], ["്", candrakkala]] as const) {
      expect(record.compositionSource?.url, sign).toBe(JAYASREE_COMPOSER);
      expect(record.compositionSource?.citation, sign).toMatch(
        new RegExp(
          `^Sachin Nandakumar, Jayasree: .*commit e0c9d57.*js/src/index\\.js, applyMarkStroke: a consonant with ${sign} is animated as the consonant’s recorded strokes, then .*"Jayasree" by Sachin Nandakumar`,
        ),
      );
      expect(record.compositionSource?.variation, sign).toMatch(
        /classes . as a suffix mark.*applyMarkStroke.*not a recording of a consonant with .*confidence is medium.*38 consonants .* unmoved.*no code or recorded coordinate is copied/,
      );
    }
  });
});

describe("Malayalam clusters the font prints as their parts", () => {
  it("draws a cited cluster consonant by consonant, each candrakkala after its own", () => {
    expect(writtenPiecesOf("സ്ക", "malayalam")).toEqual(["സ", "്", "ക"]);
    expect(writtenPiecesOf("സ്കാ", "malayalam")).toEqual(["സ", "്", "ക", "ാ"]);
  });

  it("refuses a fused cluster, and a cited one with a sign written before it", () => {
    // ന്ത and മ്മ print as glyphs of their own; no cited ductus draws them.
    for (const grapheme of ["ന്ത", "ന്തോ", "മ്മ", "ക്ക"]) {
      expect(writtenPiecesOf(grapheme, "malayalam"), grapheme).toBeUndefined();
    }
    // The font prints േ between ് and ക; no source orders it against the cluster.
    expect(writtenPiecesOf("സ്കോ", "malayalam")).toBeUndefined();
    expect(writtenPiecesOf("സ്കെ", "malayalam")).toBeUndefined();
  });

  it("names every cluster's letters in its citation, read from the bundled font", () => {
    expect(Object.keys(APART_CLUSTER_SOURCES)).toEqual(["malayalam"]);
    for (const source of APART_CLUSTER_SOURCES.malayalam!) {
      expect(source.url).toMatch(/^https:\/\//);
      for (const cluster of source.clusters) {
        expect(cluster.normalize("NFD")).toBe(cluster);
        const parts = [...cluster];
        // consonant, candrakkala, consonant: the candrakkala has an "after" row.
        expect(parts).toHaveLength(3);
        expect(parts[0]).toMatch(/^\p{L}$/u);
        expect(parts[2]).toMatch(/^\p{L}$/u);
        expect(WRITTEN_SIGN_SIDES.malayalam![parts[1]!]).toBe("after");
        expect(source.citation, cluster).toContain(cluster);
        for (const part of parts) expect(source.citation, `${cluster}: ${part}`).toContain(part);
      }
    }
    // The shaping was read from the bundled font; if it is replaced, its
    // version string changes and the citation must be read again. The
    // OpenType name table stores the version string as UTF-16BE.
    const font = readFileSync(join(defaultCurriculumRoot(), "_fonts", "NotoSansMalayalam-Static.ttf"));
    expect(APART_CLUSTER_SOURCES.malayalam![0]!.citation).toContain("Noto Sans Malayalam Version 2.104");
    expect(font.includes(Buffer.from("Version 2.104", "utf16le").swap16())).toBe(true);
  });
});

describe("Malayalam words drawn part by part", () => {
  it("draws നമസ്കാരം in written order, its cluster apart", () => {
    const greeting = malayalam("ML-W1", "നമസ്കാരം");
    expect(writingSequenceOf(greeting, "malayalam")).toEqual(["ന", "മ", "സ", "്", "ക", "ാ", "ര", "ം"]);
    expect(
      filmstripImageMarkdown({
        kind: "script-filmstrip",
        lessonId: "ML-W1",
        script: "malayalam",
        glyph: "നമസ്കാരം",
        letters: ["ന", "മ", "സ", "്", "ക", "ാ", "ര", "ം"],
        output: "malayalam/book/figures/ML-W1-filmstrip.svg",
      }),
    ).toBe("![How നമസ്കാരം is written, part by part, stroke by stroke](figures/ML-W1-filmstrip.svg)");
  });

  it("draws a word with a final candrakkala, then the digit after it", () => {
    expect(writingSequenceOf(malayalam("ML-W2", "ഏഴ് ൭"), "malayalam")).toEqual(["ഏ", "ഴ", "്", "൭"]);
  });

  it("refuses a word with a fused cluster or a left-hand sign", () => {
    expect(writingSequenceOf(malayalam("ML-W3", "സന്തോഷം"), "malayalam")).toBeUndefined();
    expect(writingSequenceOf(malayalam("ML-W4", "അമ്മ"), "malayalam")).toBeUndefined();
    expect(writingSequenceOf(malayalam("ML-W5", "കോട്ട"), "malayalam")).toBeUndefined();
  });
});
