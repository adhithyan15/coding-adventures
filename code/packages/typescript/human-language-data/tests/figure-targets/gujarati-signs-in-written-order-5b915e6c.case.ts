// Gujarati vowel signs, anusvara and visarga in a filmstrip. Every one is
// WRITTEN after its consonant — even િ, which sits to the consonant's left —
// so a Gujarati word is drawn in typed order, part by part. Each row of the
// composer's table is cited in the Gujarati mark records (KanoAI's barakhadi
// templates draw the consonant before the sign), and this file holds the
// table to them.
import { describe, expect, it } from "vitest";
import {
  filmstripCandidates,
  filmstripImageMarkdown,
  FUSED_SIGN_PAIRS,
  WRITTEN_SIGN_SIDES,
  writingLetterOf,
  writingSequenceOf,
  writtenPiecesOf,
} from "../../src/figure-targets.js";
import { defaultCurriculumRoot, loadScripts } from "../../src/loader.js";
import { lesson } from "./fixture.js";

const KANO =
  "https://github.com/gajjartejas/KanoAI/tree/9d3e2949a3f265448e430e15329093ea3aa516b8/interpolate-svg/svgs/barakhadi";
const gujarati = (id: string, headword: string) => lesson(id, { language: "gujarati", headword });

describe("where each Gujarati sign is written", () => {
  it("puts every sign AFTER its consonant, including િ, which sits to the left", () => {
    expect(writtenPiecesOf("કા", "gujarati")).toEqual(["ક", "ા"]);
    expect(writtenPiecesOf("કિ", "gujarati")).toEqual(["ક", "િ"]);
    expect(writtenPiecesOf("કી", "gujarati")).toEqual(["ક", "ી"]);
    expect(writtenPiecesOf("કુ", "gujarati")).toEqual(["ક", "ુ"]);
    expect(writtenPiecesOf("કૂ", "gujarati")).toEqual(["ક", "ૂ"]);
    expect(writtenPiecesOf("કે", "gujarati")).toEqual(["ક", "ે"]);
    expect(writtenPiecesOf("કૈ", "gujarati")).toEqual(["ક", "ૈ"]);
    expect(writtenPiecesOf("કો", "gujarati")).toEqual(["ક", "ો"]);
    expect(writtenPiecesOf("કૌ", "gujarati")).toEqual(["ક", "ૌ"]);
    expect(writtenPiecesOf("કં", "gujarati")).toEqual(["ક", "ં"]);
    expect(writtenPiecesOf("કઃ", "gujarati")).toEqual(["ક", "ઃ"]);
  });

  it("keeps ો and ૌ whole: they have no Unicode decomposition", () => {
    expect("ો".normalize("NFD")).toBe("ો");
    expect("ૌ".normalize("NFD")).toBe("ૌ");
    expect(writtenPiecesOf("ો", "gujarati")).toEqual(["ો"]);
    expect(writtenPiecesOf("ૌ", "gujarati")).toEqual(["ૌ"]);
  });

  it("refuses the virama, the vocalic-r sign, and two signs on one consonant", () => {
    // ્ and ૃ have no Gujarati source; no source orders ા against ં.
    for (const grapheme of ["ક્", "કૃ", "્", "ૃ", "કાં", "કેં", "કીં", "કોં"]) {
      expect(writtenPiecesOf(grapheme, "gujarati"), grapheme).toBeUndefined();
    }
  });

  it("refuses the pairs the bundled font reshapes, and only those", () => {
    // Ligatures of their own (ણુ, રુ, રૂ); the 22 consonants that take a stem
    // form before ુ and ૂ (નુ, મૂ, ...); and જ and ૹ, which join the ā bar alone
    // or inside ો and ૌ (જા, જી, જો, જૌ).
    for (const grapheme of ["ણુ", "રુ", "રૂ", "નુ", "મૂ", "સુ", "ખૂ", "જા", "જી", "જો", "જૌ", "ૹા", "ૹી", "ૹો", "ૹૌ"]) {
      expect(writtenPiecesOf(grapheme, "gujarati"), grapheme).toBeUndefined();
    }
    expect(FUSED_SIGN_PAIRS.gujarati!.size).toBe(22 * 2 + 2 + 2 * 4);
    // Each partner on its own still composes.
    expect(writtenPiecesOf("રા", "gujarati")).toEqual(["ર", "ા"]);
    expect(writtenPiecesOf("જે", "gujarati")).toEqual(["જ", "ે"]);
    expect(writtenPiecesOf("નો", "gujarati")).toEqual(["ન", "ો"]);
    expect(writtenPiecesOf("ણા", "gujarati")).toEqual(["ણ", "ા"]);
    expect(writtenPiecesOf("કુ", "gujarati")).toEqual(["ક", "ુ"]);
    expect(writtenPiecesOf("દૂ", "gujarati")).toEqual(["દ", "ૂ"]);
  });

  it("matches the cited written place in every Gujarati mark record", () => {
    // Each row must stand on a record whose `compositionSource` cites it and
    // whose own stroke order is cited too: a row with no record, or a record
    // saying otherwise, fails here.
    const script = loadScripts(defaultCurriculumRoot()).gujarati!;
    const rows = WRITTEN_SIGN_SIDES.gujarati!;
    expect(Object.keys(rows)).toEqual(["ા", "િ", "ી", "ુ", "ૂ", "ે", "ૈ", "ો", "ૌ", "ં", "ઃ"]);
    for (const [sign, side] of Object.entries(rows)) {
      expect(side, sign).toBe("after");
      const mark = (script.marks ?? []).find((entry) => entry.mark === sign);
      expect(mark, sign).toBeDefined();
      expect(mark!.compositionSource?.url, sign).toBe(KANO);
      expect(mark!.compositionSource?.citation, sign).toMatch(
        /KanoAI.*draws the consonant's group \(g0\) before the sign's groups/,
      );
      expect((mark!.compositionOrder ?? []).join(" | "), sign).toMatch(
        /^write the Gujarati consonant first \| write the .* after it/,
      );
      expect(mark!.strokeOrderSource?.url, sign).toBe(KANO);
    }
    // And no record claims a place the table leaves out.
    for (const mark of script.marks ?? []) {
      if (mark.mark in rows) continue;
      expect(mark.compositionOrder, mark.mark).toBeUndefined();
      expect(mark.strokeOrderSource, mark.mark).toBeUndefined();
    }
  });
});

describe("Gujarati words and signs as strips", () => {
  it("draws a word in typed order, consonant then sign", () => {
    expect(writingSequenceOf(gujarati("GU-1", "કેમકે"), "gujarati")).toEqual(["ક", "ે", "મ", "ક", "ે"]);
    expect(writingSequenceOf(gujarati("GU-2", "અને"), "gujarati")).toEqual(["અ", "ન", "ે"]);
    expect(writingSequenceOf(gujarati("GU-3", "હા"), "gujarati")).toEqual(["હ", "ા"]);
    expect(writingSequenceOf(gujarati("GU-4", "તે"), "gujarati")).toEqual(["ત", "ે"]);
  });

  it("refuses a word with an uncited sign or a fused pair", () => {
    expect(writingSequenceOf(gujarati("GU-5", "નમસ્તે"), "gujarati")).toBeUndefined();
    expect(writingSequenceOf(gujarati("GU-6", "ક્યાં"), "gujarati")).toBeUndefined();
    expect(writingSequenceOf(gujarati("GU-7", "બજાર"), "gujarati")).toBeUndefined();
    expect(writingSequenceOf(gujarati("GU-7", "જો"), "gujarati")).toBeUndefined();
    expect(writingSequenceOf(gujarati("GU-7", "દુકાન"), "gujarati")).toEqual(["દ", "ુ", "ક", "ા", "ન"]);
  });

  it("draws a sign taught alone as one glyph, and a list of signs as its items", () => {
    expect(writingSequenceOf(gujarati("GU-8", "ો"), "gujarati")).toBeUndefined();
    expect(writingLetterOf(gujarati("GU-8", "ો"))).toBe("ો");
    expect(writingSequenceOf(gujarati("GU-9", "ુ ી"), "gujarati")).toEqual(["ુ", "ી"]);
    expect(writingSequenceOf(gujarati("GU-10", "ળ — થ — અ — િ"), "gujarati")).toEqual(["ળ", "થ", "અ", "િ"]);
  });

  it("becomes a candidate captioned part by part", () => {
    const [word, single] = filmstripCandidates([gujarati("GU-W1", "કે"), gujarati("GU-W2", "ા")]);
    expect(word).toMatchObject({ lessonId: "GU-W1", script: "gujarati", glyph: "કે", letters: ["ક", "ે"] });
    expect(single).toEqual({
      kind: "script-filmstrip",
      lessonId: "GU-W2",
      script: "gujarati",
      glyph: "ા",
      output: "gujarati/book/figures/GU-W2-filmstrip.svg",
    });
    expect(filmstripImageMarkdown(word!)).toBe(
      "![How કે is written, part by part, stroke by stroke](figures/GU-W1-filmstrip.svg)",
    );
  });
});
