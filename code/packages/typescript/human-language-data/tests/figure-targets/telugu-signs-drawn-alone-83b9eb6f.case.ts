// Telugu vowel signs, the anusvara and the virama get a filmstrip only where
// a lesson teaches the sign BY ITSELF. Eleven signs have one: ం ా ి ీ ు ూ ె
// ే ొ ో ్. Their cited source — native writers' pen traces in HP Labs India's
// LipiTk Telugu recognizer — was written one sign at a time, with no
// consonant beside it, so it says how the sign is drawn but not when it is
// drawn against its consonant. So Telugu has no row in the composer's
// written-order table: a sign on a consonant ("కు", "కి") is never drawn
// letter by letter, and no Telugu word is composed from these signs.
//
// ై stays without a ductus: the recognizer's ai class stores only the length
// mark below (ౖ), never the e hook above it, so the order of the two parts is
// unattested. ృ and ౌ are not in the recognizer at all.
import { describe, expect, it } from "vitest";
import {
  filmstripCandidates,
  FUSED_SIGN_PAIRS,
  withDerivedFilmstrips,
  WRITTEN_SIGN_SIDES,
  writingLetterOf,
  writingSequenceOf,
  writtenPiecesOf,
} from "../../src/figure-targets.js";
import { defaultCurriculumRoot, loadScripts } from "../../src/loader.js";
import { lesson } from "./fixture.js";

const LIPITK = "https://lipitk.sourceforge.net/lipi-reco.htm";
const SIGNS = ["ం", "ా", "ి", "ీ", "ు", "ూ", "ె", "ే", "ొ", "ో", "్"];
const telugu = (id: string, headword: string) => lesson(id, { language: "telugu", headword });
/** The eleven signs that carry a cited ductus, plus one letter. */
const CITED = new Set([...SIGNS, "క"]);
const hasDuctus = (script: string, glyph: string) => script === "telugu" && CITED.has(glyph);

describe("Telugu signs drawn alone", () => {
  it("gives Telugu no written-order row, so no sign is placed against a consonant", () => {
    expect(WRITTEN_SIGN_SIDES.telugu).toBeUndefined();
    expect(FUSED_SIGN_PAIRS.telugu).toBeUndefined();
    for (const grapheme of ["కా", "కి", "కీ", "కు", "కూ", "కె", "కే", "కొ", "కో", "కం", "క్", ...SIGNS]) {
      expect(writtenPiecesOf(grapheme, "telugu"), grapheme).toBeUndefined();
    }
  });

  it("takes a sign taught alone as one glyph, never as a sequence", () => {
    for (const sign of SIGNS) {
      expect(writingSequenceOf(telugu("TE-1", sign), "telugu"), sign).toBeUndefined();
      expect(writingLetterOf(telugu("TE-1", sign)), sign).toBe(sign);
    }
  });

  it("draws the bare sign and refuses the same sign on a consonant, and ై", () => {
    const candidates = filmstripCandidates([
      telugu("TE-W1", "ు"),
      telugu("TE-W2", "కు"),
      telugu("TE-W3", "ై"),
      telugu("TE-W4", "ే"),
      telugu("TE-W5", "కే"),
    ]);
    expect(candidates.map((target) => [target.lessonId, target.glyph, target.letters])).toEqual([
      ["TE-W1", "ు", undefined],
      ["TE-W2", "కు", undefined],
      ["TE-W3", "ై", undefined],
      ["TE-W4", "ే", undefined],
      ["TE-W5", "కే", undefined],
    ]);
    expect(withDerivedFilmstrips([], candidates, hasDuctus).map((target) => target.lessonId)).toEqual([
      "TE-W1",
      "TE-W4",
    ]);
  });

  it("cites a ductus on exactly the eleven signs, each to the recognizer and drawn alone", () => {
    const script = loadScripts(defaultCurriculumRoot()).telugu!;
    const cited = (script.marks ?? []).filter((mark) => mark.strokeOrderSource !== undefined);
    // In the records' own order (telugu.json `marks`).
    expect(cited.map((mark) => mark.mark)).toEqual(["్", "ం", "ా", "ి", "ీ", "ు", "ూ", "ె", "ే", "ొ", "ో"]);
    for (const mark of cited) {
      expect(mark.strokeOrderSource?.url, mark.mark).toBe(LIPITK);
      expect(mark.strokeOrderSource?.variation, mark.mark).toMatch(
        /written by itself, with no consonant beside it.*composes no Telugu word from it.*only counts and shares are cited/,
      );
    }
    expect(script.marks?.find((mark) => mark.mark === "ై")).toBeUndefined();
    // The virama and the anusvara keep the carrier-first composition order
    // cited to Unicode; the traces add how each is drawn by itself.
    const ordered = (script.marks ?? []).filter((mark) => mark.compositionOrder !== undefined);
    expect(ordered.map((mark) => mark.mark)).toEqual(["్", "ం"]);
  });
});
